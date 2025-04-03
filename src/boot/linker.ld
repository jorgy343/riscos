ENTRY(_boot_entrypoint)

SECTIONS
{
    . = 0x80200000;
    _boot_start = .;

    .text : ALIGN(4K) {
        _boot_text_start = .;
        *libboot.a:*(.text.boot_entrypoint)
        *libboot.a:*(.text*)
    }

    .data : ALIGN(4K) {
        _boot_data_start = .;
        *libboot.a:*(.data*)
    }

    .bss : ALIGN(4K) {
        _boot_bss_start = .;
        *libboot.a:*(.bss*)
        *libboot.a:*(COMMON)
    }

    .rodata : ALIGN(4K) {
        _boot_rodata_start = .;
        *libboot.a:*(.rodata*)
    }

    .stack : ALIGN(4K) {
        _boot_stack_start = .;
        BYTE(0) /* Force the stack section to be present in the final binary. */
        . = . + 16K - 1; /* Subtract 1 to account for the dummy byte. */
    }

    _boot_text_length = SIZEOF(.text);
    _boot_data_length = SIZEOF(.data);
    _boot_bss_length = SIZEOF(.bss);
    _boot_rodata_length = SIZEOF(.rodata);
    _boot_stack_length = SIZEOF(.stack);

    _boot_end = . - 1;
}
