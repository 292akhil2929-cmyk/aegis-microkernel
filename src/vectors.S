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
REPORT vector_irq_current_spx, 5
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

