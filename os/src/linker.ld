OUTPUT_ARCH(riscv)
ENTRY(_start)
BASE_ADDRESS = 0x80200000;

SECTIONS
{
    . = BASE_ADDRESS;   // . 为临时变量
    skernel = .;

    stext = .;
    .text : {
        *(.text.entry)
        *(.text .text.*)
    }

    . = ALIGN(4K);
    /*含义: . = ALIGN(4K)把当前位置指针.向上对齐到最近的4KB边界（4096字节）。
        数值公式: 对齐后的.等于(. + 4096 - 1) & ~(4096 - 1)，也就是把当前值四舍五入到下一个4096的倍数。
        具体值: 取决于前面已放入段的大小。比如
        若 .text 结束时.为0x80203456，对齐后变为0x80204000。
        若刚好在边界，如.为0x80204000，对齐结果仍是0x80204000。*/
    etext = .;
    srodata = .;
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }   // 将文件中所有该段合并到.rodata，并将rodata置于此。
    //写下.rodata证明.rodata数据放在这里。
    . = ALIGN(4K);
    erodata = .;
    sdata = .;
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }

    . = ALIGN(4K);
    edata = .;
    .bss : {
        *(.bss.stack)
        sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }

    . = ALIGN(4K);
    ebss = .;
    ekernel = .;

    /DISCARD/ : {
        *(.eh_frame)
    }
}