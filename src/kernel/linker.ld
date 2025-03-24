ENTRY(_kernel_entrypoint)

SECTIONS
{
    . = 0x0000004000000000;
    _kernel_start = .;

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

    _kernel_end = . - 1;
}
