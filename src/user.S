.section .user.text, "ax"
.global __user_entry
__user_entry:
    mov x19, #0x19
    mov x20, #0x20
    mov x21, #0x21
.Lapp_preempt_wait:
    add x22, x22, #1
    b .Lapp_preempt_wait

.global __user_a_after_preempt
__user_a_after_preempt:
    svc #0
    mov x0, #65
    svc #10

.global __user_a_resumed
__user_a_resumed:
    svc #12
    ldr x1, =0x09000000
    mov w0, #0x41
    str w0, [x1]

.global __user_after_fault
__user_after_fault:
    svc #2
1:  wfe
    b 1b

.global __user_b_entry
__user_b_entry:
    svc #11
1:  wfe
    b 1b

.global __preempt_b_entry
__preempt_b_entry:
    mov x19, #0xb
.Lpreempt_b_wait:
    add x23, x23, #1
    b .Lpreempt_b_wait

.section .text, "ax"
.global launch_user_demo
launch_user_demo:
    ldr x0, =__user_stack_a_top
    msr sp_el0, x0
    ldr x0, =__user_entry
    msr elr_el1, x0
    mov x0, #0
    msr spsr_el1, x0
    isb
    eret
