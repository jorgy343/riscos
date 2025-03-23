ENTRY(_boot_start)

SECTIONS
{
    . = 0x80200000;
    _kernel_begin = .;

    .text : ALIGN(4K) {
        _text_begin = .;
        libboot.a:*(.text.boot_start)
        libboot.a:*(.text*)
        _text_end = . - 1;
    }

    .data : ALIGN(4K) {
        _data_begin = .;
        libboot.a:*(.data*)
        _data_end = . - 1;
    }

    .bss : ALIGN(4K) {
        _bss_begin = .;
        libboot.a:*(.bss*)
        libboot.a:*(COMMON)
        _bss_end = . - 1;
    }

    .rodata : ALIGN(4K) {
        _rodata_begin = .;
        libboot.a:*(.rodata*)
        _rodata_end = . - 1;
    }

    .stack : ALIGN(4K) {
        _stack_begin = .;
        . = . + 16K;
        _stack_end = . - 1;
    }

    _kernel_end_exclusive = .;
}
