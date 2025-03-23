ENTRY(_boot_entrypoint)

SECTIONS
{
    . = 0x80200000;
    _boot_start = .;

    .text : ALIGN(4K) {
        _boot_text_start = .;
        *libboot.a:*(.text.boot_entrypoint)
        *libboot.a:*(.text*)
        _boot_text_end = . - 1;
    }

    .data : ALIGN(4K) {
        _boot_data_start = .;
        *libboot.a:*(.data*)
        _boot_data_end = . - 1;
    }

    .bss : ALIGN(4K) {
        _boot_bss_start = .;
        *libboot.a:*(.bss*)
        *libboot.a:*(COMMON)
        _boot_bss_end = . - 1;
    }

    .rodata : ALIGN(4K) {
        _boot_rodata_start = .;
        *libboot.a:*(.rodata*)
        _boot_rodata_end = . - 1;
    }

    .stack : ALIGN(4K) {
        _boot_stack_start = .;
        BYTE(0) /* Force the stack section to be present in the final binary. */
        . = . + 16K - 1; /* Subtract 1 to account for the dummy byte. */
        _boot_stack_end = . - 1;
    }

    _boot_end = . - 1;
}
