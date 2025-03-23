ENTRY(_boot_entrypoint)

SECTIONS
{
    . = 0x80200000;
    _kernel_start = .;

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
        . = . + 16K;
        _boot_stack_end = . - 1;
    }

    .text : ALIGN(4K) {
        _kernel_text_start = .;
        *libkernel.a:*(.text.kernel_entrypoint)
        *libkernel.a:*(.text*)
        _kernel_text_end = . - 1;
    }

    .data : ALIGN(4K) {
        _kernel_data_start = .;
        *libkernel.a:*(.data*)
        _kernel_data_end = . - 1;
    }

    .bss : ALIGN(4K) {
        _kernel_bss_start = .;
        *libkernel.a:*(.bss*)
        *libkernel.a:*(COMMON)
        _kernel_bss_end = . - 1;
    }

    .rodata : ALIGN(4K) {
        _kernel_rodata_start = .;
        *libkernel.a:*(.rodata*)
        _kernel_rodata_end = . - 1;
    }

    .stack : ALIGN(4K) {
        _kernel_stack_start = .;
        . = . + 16K;
        _kernel_stack_end = . - 1;
    }

    _kernel_end = . - 1;
}
