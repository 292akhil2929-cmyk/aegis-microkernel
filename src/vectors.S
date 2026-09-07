.macro VECTOR target
    b \target
    .space 124
.endm

.section .text.vectors, "ax"
.align 11
.global __exception_vectors
__exception_vectors:
    VECTOR vector_sync_current_sp0
    VECTOR vector_irq_current_sp0
    VECTOR vector_fiq_current_sp0
    VECTOR vector_serror_current_sp0
    VECTOR vector_sync_current_spx
    VECTOR vector_irq_current_spx
    VECTOR vector_fiq_current_spx
    VECTOR vector_serror_current_spx
    VECTOR vector_sync_lower_a64
    VECTOR vector_irq_lower_a64
    VECTOR vector_fiq_lower_a64
    VECTOR vector_serror_lower_a64
    VECTOR vector_sync_lower_a32
    VECTOR vector_irq_lower_a32
    VECTOR vector_fiq_lower_a32
    VECTOR vector_serror_lower_a32

.macro REPORT name, number
\name:
    mov x0, #\number
    mrs x1, esr_el1
    mrs x2, elr_el1
    b exception_report
.endm

REPORT vector_sync_current_sp0, 0
REPORT vector_irq_current_sp0, 1
REPORT vector_fiq_current_sp0, 2
REPORT vector_serror_current_sp0, 3
REPORT vector_sync_current_spx, 4

.align 4
vector_irq_current_spx:
    sub sp, sp, #256
    stp x0, x1, [sp, #0]
    stp x2, x3, [sp, #16]
    stp x4, x5, [sp, #32]
    stp x6, x7, [sp, #48]
    stp x8, x9, [sp, #64]
    stp x10, x11, [sp, #80]
    stp x12, x13, [sp, #96]
    stp x14, x15, [sp, #112]
    stp x16, x17, [sp, #128]
    stp x18, x19, [sp, #144]
    stp x20, x21, [sp, #160]
    stp x22, x23, [sp, #176]
    stp x24, x25, [sp, #192]
    stp x26, x27, [sp, #208]
    stp x28, x29, [sp, #224]
    str x30, [sp, #240]
    bl irq_dispatch
    ldp x0, x1, [sp, #0]
    ldp x2, x3, [sp, #16]
    ldp x4, x5, [sp, #32]
    ldp x6, x7, [sp, #48]
    ldp x8, x9, [sp, #64]
    ldp x10, x11, [sp, #80]
    ldp x12, x13, [sp, #96]
    ldp x14, x15, [sp, #112]
    ldp x16, x17, [sp, #128]
    ldp x18, x19, [sp, #144]
    ldp x20, x21, [sp, #160]
    ldp x22, x23, [sp, #176]
    ldp x24, x25, [sp, #192]
    ldp x26, x27, [sp, #208]
    ldp x28, x29, [sp, #224]
    ldr x30, [sp, #240]
    add sp, sp, #256
    eret

REPORT vector_fiq_current_spx, 6
REPORT vector_serror_current_spx, 7
REPORT vector_sync_lower_a64, 8
REPORT vector_irq_lower_a64, 9
REPORT vector_fiq_lower_a64, 10
REPORT vector_serror_lower_a64, 11
REPORT vector_sync_lower_a32, 12
REPORT vector_irq_lower_a32, 13
REPORT vector_fiq_lower_a32, 14
REPORT vector_serror_lower_a32, 15
