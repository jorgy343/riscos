ENTRY(_kernel_entrypoint)

SECTIONS
{
    . = 0xFFFFFFC000000000;
    _kernel_start = .;

    .text : ALIGN(4K) {
        _kernel_text_start = .;
        *libkernel.a:*(.text.kernel_entrypoint)
        *libkernel.a:*(.text*)
    }

    .data : ALIGN(4K) {
        _kernel_data_start = .;
        *libkernel.a:*(.data*)
    }

    .bss : ALIGN(4K) {
        _kernel_bss_start = .;
        *libkernel.a:*(.bss*)
        *libkernel.a:*(COMMON)
    }

    .rodata : ALIGN(4K) {
        _kernel_rodata_start = .;
        *libkernel.a:*(.rodata*)
    }

    _kernel_text_length = SIZEOF(.text);
    _kernel_data_length = SIZEOF(.data);
    _kernel_bss_length = SIZEOF(.bss);
    _kernel_rodata_length = SIZEOF(.rodata);

    _kernel_end = . - 1;
}
