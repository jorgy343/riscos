SECTIONS
{
    . = 0x80200000;
    _kernel_begin = .;

    .text : ALIGN(4K) {
        _text_begin = .;
        *(.text.kernel_boot)
        *(.text*)
        _text_end = .;
    }

    .data : ALIGN(4K) {
        _data_begin = .;
        *(.data*)
        _data_end = .;
    }

    .bss : ALIGN(4K) {
        _bss_begin = .;
        *(.bss*)
        *(COMMON)
        _bss_end = .;
    }

    .rodata : ALIGN(4K) {
        _rodata_begin = .;
        *(.rodata*)
        _rodata_end = .;
    }

    _stack_begin = .;
    . = ALIGN(4K);
    . = . + 4K;
    _stack_end = .;

    _kernel_end = .;
}
