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
    mov x0, #3
    msr cnthctl_el2, x0
    msr cntvoff_el2, xzr
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
    // Early 39-bit stage-1 map. L1[0] covers devices. RAM uses L2/L3
    // tables so the EL0 demo receives only its code and stack pages.
    ldr x0, =__boot_l1_table
    ldr x1, =0x0060000000000401
    str x1, [x0]
    ldr x1, =__boot_l2_table
    orr x1, x1, #3
    str x1, [x0, #8]

    ldr x2, =__boot_l3_table
    orr x1, x2, #3
    ldr x3, =__boot_l2_table
    str x1, [x3]

    // Kernel-only 4 KiB mappings for the first 2 MiB of RAM.
    mov x3, #0
    ldr x4, =0x40000707
.Lmap_l3:
    str x4, [x2, x3, lsl #3]
    add x4, x4, #0x1000
    add x3, x3, #1
    cmp x3, #512
    b.ne .Lmap_l3

    // User text: EL0 read/execute, EL1 execute-never.
    ldr x4, =__user_text_start
    ldr x6, =0x40000000
    sub x3, x4, x6
    lsr x3, x3, #12
    ldr x5, =0x00200000000007c7
    orr x5, x4, x5
    str x5, [x2, x3, lsl #3]

    // User stack: EL0 read/write, never executable.
    ldr x4, =__user_stack_bottom
    sub x3, x4, x6
    lsr x3, x3, #12
    ldr x5, =0x0060000000000747
    orr x5, x4, x5
    str x5, [x2, x3, lsl #3]

    // Remaining RAM uses kernel-only 2 MiB blocks.
    ldr x3, =__boot_l2_table
    mov x4, #1
    ldr x5, =0x0060000040200705
.Lmap_l2:
    str x5, [x3, x4, lsl #3]
    add x5, x5, #0x200000
    add x4, x4, #1
    cmp x4, #512
    b.ne .Lmap_l2

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
.global __boot_l2_table
__boot_l2_table:
    .skip 4096
.global __boot_l3_table
__boot_l3_table:
    .skip 4096
