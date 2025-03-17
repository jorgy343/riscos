SECTIONS
{
    . = 0x80200000;
    _kernel_begin = .;

    .text : ALIGN(4K) {
        _text_begin = .;
        *(.text.kernel_boot)
        *(.text*)
        _text_end = . - 1;
    }

    .data : ALIGN(4K) {
        _data_begin = .;
        *(.data*)
        _data_end = . - 1;
    }

    .bss : ALIGN(4K) {
        _bss_begin = .;
        *(.bss*)
        *(COMMON)
        _bss_end = . - 1;
    }

    .rodata : ALIGN(4K) {
        _rodata_begin = .;
        *(.rodata*)
        _rodata_end = . - 1;
    }

    _stack_begin = .;
    . = ALIGN(4K);
    . = . + 16K;
    _stack_end = . - 1;

    _kernel_end_exclusive = .;
}
