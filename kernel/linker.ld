SECTIONS
{
    . = 0x80200000;

    .text.boot : ALIGN(16) {
        *(.text.boot)
    }

    .text : ALIGN(16) {
        *(.text)
        *(.text.*)
    }

    .rodata : ALIGN(16) {
        *(.rodata)
        *(.rodata.*)
    }

    .data : ALIGN(16) {
        *(.data)
        *(.data.*)
    }

    .bss : ALIGN(16) {
        *(.bss)
        *(.bss.*)
        *(COMMON)
    }

    . = ALIGN(16);
    . = . + 32768;

    _stack_start = .;
}
