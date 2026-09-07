ENTRY(_start)

SECTIONS
{
    . = 0x40080000;
    __kernel_start = .;

    .text : ALIGN(0x1000) {
        KEEP(*(.text.boot))
        KEEP(*(.text.vectors))
        *(.text .text.*)
    }

    .user_text : ALIGN(0x1000) {
        __user_text_start = .;
        KEEP(*(.user.text))
        __user_text_end = .;
    }

    .rodata : ALIGN(0x1000) {
        *(.rodata .rodata.*)
    }

    .data : ALIGN(0x1000) {
        *(.data .data.*)
    }

    .bss (NOLOAD) : ALIGN(0x1000) {
        __bss_start = .;
        *(.bss.stack)
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    }

    .user_stack (NOLOAD) : ALIGN(0x1000) {
        __user_stack_bottom = .;
        . += 4096;
        __user_stack_top = .;
    }

    __kernel_end = ALIGN(0x1000);
    /DISCARD/ : { *(.comment) *(.eh_frame*) }
}
