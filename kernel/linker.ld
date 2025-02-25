SECTIONS
{
    . = 0x80200000;

    .text.boot : {
        *(.text.boot)
    }

    .text : {
        *(.text)
    }

    .rodata : {
        *(.rodata)
    }

    .data : {
        *(.data)
    }

    .bss : {
        *(.bss)
    }

    . = ALIGN(8);
    . = . + 32768;

    _stack_start = .;
}
