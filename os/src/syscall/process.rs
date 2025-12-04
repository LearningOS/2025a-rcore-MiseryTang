//! Process management syscalls
use crate::{task::{TASK_MANAGER, change_program_brk , exit_current_and_run_next, suspend_current_and_run_next}, timer::get_time};
use crate::mm::translated_byte_buffer;
#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}
/// 全局系统调用计数器：每个任务一个数组，统计每种系统调用的次数
pub static mut SYSCALL_QUANTITY: [[usize; 256]; 16] = [[0; 256]; 16];
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
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
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
    let mut _buffer = translated_byte_buffer(
        TASK_MANAGER.get_current_token(),
        _ts as *const u8,
        core::mem::size_of::<TimeVal>(),
    );
    let mut offset = 0;
    for buf in &mut _buffer {
        let len = buf.len().min(timeval_bytes.len() - offset);
        buf[..len].copy_from_slice(&timeval_bytes[offset..offset + len]);
        offset += len;
    }
    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    trace!("kernel: sys_trace");
    match _trace_request {
        0 => {
            let buffer = translated_byte_buffer(
                TASK_MANAGER.get_current_token(),
                _id as *const u8,
                1,
            );
            let offset = (_id as usize) & 0xfff;
            return buffer[0][offset] as isize;
        },
        1 => {
            let mut buffer = translated_byte_buffer(
                TASK_MANAGER.get_current_token(),
                _id as *const u8,
                1,
            );
            let offset = (_id as usize) & 0xfff;
            buffer[0][offset] = _data as u8;
            return 0;
        }
            
        2 => unsafe { return SYSCALL_QUANTITY[TASK_MANAGER.get_current_task()][_id] as isize ;},
        _ => return -1,
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    -1
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    -1
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

