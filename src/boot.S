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
    // Early 39-bit stage-1 map. L1[0] covers devices, L1[1] covers RAM.
    ldr x0, =__boot_l1_table
    ldr x1, =0x0060000000000401
    str x1, [x0]
    ldr x1, =0x0000000040000705
    str x1, [x0, #8]

    ldr x1, =0x000000000000ff00
    msr mair_el1, x1
    ldr x1, =0x0000000200803519
    msr tcr_el1, x1
    msr ttbr0_el1, x0
    dsb ish
    tlbi vmalle1
    dsb ish
    isb
    mrs x1, sctlr_el1
    orr x1, x1, #1
    orr x1, x1, #4
    orr x1, x1, #0x1000
    msr sctlr_el1, x1
    isb

    bl kernel_main
1:  wfe
    b 1b

.section .bss.stack, "aw", %nobits
.align 12
__boot_stack_bottom:
    .skip 16384
.global __boot_stack_top
__boot_stack_top:

.section .bss.page_tables, "aw", %nobits
.align 12
.global __boot_l1_table
__boot_l1_table:
    .skip 4096
