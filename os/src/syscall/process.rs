//! Process management syscalls
use crate::{task::{TASK_MANAGER, change_program_brk , exit_current_and_run_next, suspend_current_and_run_next}, timer::get_time};
use crate::mm::{translated_byte_buffer, PageTable, VirtAddr, PTEFlags, StepByOne};
use crate::mm::MapPermission;
use crate::mm::FRAME_ALLOCATOR;
#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}
/// 全局系统调用计数器：每个任务一个数组，统计每种系统调用的次数
pub static mut SYSCALL_QUANTITY: [[usize; 512]; 16] = [[0; 512]; 16];
/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let timeval = TimeVal {
        sec: get_time() / 1_000_000,
        usec: get_time() % 1_000_000,
    };
    let timeval_bytes = unsafe {
        core::slice::from_raw_parts(
            &timeval as *const TimeVal as *const u8,
            core::mem::size_of::<TimeVal>(),
        )
    };
    let mut buffer = translated_byte_buffer(
        TASK_MANAGER.get_current_token(),
        ts as *const u8,
        core::mem::size_of::<TimeVal>(),
    );
    let mut offset = 0;
    for buf in &mut buffer {
        let len = buf.len().min(timeval_bytes.len() - offset);
        buf[..len].copy_from_slice(&timeval_bytes[offset..offset + len]);
        offset += len;
    }
    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    let token = TASK_MANAGER.get_current_token();
    let page_table = PageTable::from_token(token);

    match trace_request {
        0 => {
            let va = VirtAddr::from(id);
            if let Some(pte) = page_table.translate(va.floor()) {
                let flags = pte.flags();
                if flags.contains(PTEFlags::V) && flags.contains(PTEFlags::R) && flags.contains(PTEFlags::U) {
                    let buffer = translated_byte_buffer(token, id as *const u8, 1);
                    return buffer[0][0] as isize;
                }
            }
            -1
        }
        1 => {
            let va = VirtAddr::from(id);
            if let Some(pte) = page_table.translate(va.floor()) {
                let flags = pte.flags();
                if flags.contains(PTEFlags::V) && flags.contains(PTEFlags::W) && flags.contains(PTEFlags::U) {
                    let mut buffer = translated_byte_buffer(token, id as *const u8, 1);
                    buffer[0][0] = data as u8;
                    return 0;
                }
            }
            -1
        }
        2 => unsafe { SYSCALL_QUANTITY[TASK_MANAGER.get_current_task()][id] as isize },
        _ => -1,
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel: sys_mmap");
    // Check if start is page aligned
    if start & 0xfff != 0 {
        return -1;
    }
    // Check permissions
    // port: 1=R, 2=W, 4=X
    // If port has bits other than 0x7 (R|W|X), it's invalid
    if port & !0x7 != 0 {
        return -1;
    }
    // If port is empty, it's invalid (usually)
    if port & 0x7 == 0 {
        return -1;
    }
    // Check specific invalid combination: Write but not Read?
    // The original code had: if (_port & 0x2 != 0) && (_port & 0x1 == 0) { return -1; }
    // This means if Write is set, Read must also be set.
    // This is a common restriction in some systems or specific to this lab's requirements.
    if (port & 0x2 != 0) && (port & 0x1 == 0) {
        return -1;
    }

    let start_va = VirtAddr::from(start);
    let end_va = VirtAddr::from(start + len);
    let start_vpn = start_va.floor();
    let end_vpn = end_va.ceil();

    // Check if any page in the range is already mapped
    let mut is_mapped = false;
    TASK_MANAGER.get_current_memset(|memset| {
        let mut vpn = start_vpn;
        while vpn.0 < end_vpn.0 {
            if memset.translate(vpn).is_some() {
                is_mapped = true;
                break;
            }
            vpn.step();
        }
    });
    if is_mapped {
        return -1;
    }

    // Check if we have enough frames
    // Calculate number of pages needed
    let pages_needed = end_vpn.0 - start_vpn.0;
    // Use _full check if it exists, otherwise just trust allocator or check recycled
    // The user provided code used: FRAME_ALLOCATOR.exclusive_access()._full(_len/4095 + 1)
    // _len/4095 + 1 is an approximation. Correct is pages_needed.
    if FRAME_ALLOCATOR.exclusive_access()._full(pages_needed) {
        return -1;
    }

    // Convert port to MapPermission
    // port: 1=R, 2=W, 4=X
    // MapPermission: R=1<<1, W=1<<2, X=1<<3, U=1<<4
    // (port & 0x7) << 1 maps 1->2, 2->4, 4->8. Correct.
    // | 0x10 adds User bit. Correct.
    let permission = MapPermission::from_bits_truncate(((port as u8 & 0x7) << 1) | 0x10);

    TASK_MANAGER.get_current_memset(|memset| {
        memset.insert_framed_area(start_va, end_va, permission);
    });
    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap");
    if start & 0xfff != 0 {
        return -1;
    }
    let start_va = VirtAddr::from(start);
    let end_va = VirtAddr::from(start + len);
    let start_vpn = start_va.floor();
    let end_vpn = end_va.ceil();

    let mut ret = 0;
    TASK_MANAGER.get_current_memset(|memset| {
        // 1. Check if all pages in range are mapped
        let mut vpn = start_vpn;
        while vpn.0 < end_vpn.0 {
            if memset.translate(vpn).is_none() {
                ret = -1;
                return;
            }
            vpn.step();
        }

        // 2. If check passed, remove the range
        memset.remove_area_range(start_va, end_va);
    });
    ret
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

