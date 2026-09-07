.section .text.boot, "ax"
.global _start
.type _start, %function
_start:
    msr daifset, #0xf
    ldr x0, =__boot_stack_top
    mov sp, x0

    mrs x0, CurrentEL
    lsr x0, x0, #2
    cmp x0, #2
    b.eq .Ldrop_from_el2
    cmp x0, #1
    b.eq .Lenter_rust
1:  wfe
    b 1b

.Ldrop_from_el2:
    mov x0, #(1 << 31)
    msr hcr_el2, x0
    mov x0, #0x3c5
    msr spsr_el2, x0
    adr x0, .Lenter_rust
    msr elr_el2, x0
    ldr x0, =__boot_stack_top
    msr sp_el1, x0
    eret

.Lenter_rust:
    ldr x0, =__bss_start
    ldr x1, =__bss_end
.Lclear_bss:
    cmp x0, x1
    b.hs .Lbss_ready
    str xzr, [x0], #8
    b .Lclear_bss
.Lbss_ready:
    bl kernel_main
1:  wfe
    b 1b

.section .bss.stack, "aw", %nobits
.align 12
__boot_stack_bottom:
    .skip 16384
.global __boot_stack_top
__boot_stack_top:
