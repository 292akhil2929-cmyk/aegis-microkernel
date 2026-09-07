.section .user.text, "ax"
.global __user_entry
__user_entry:
    svc #0
    ldr x1, =0x09000000
    mov w0, #0x41
    str w0, [x1]

.global __user_after_fault
__user_after_fault:
    svc #2
1:  wfe
    b 1b

.section .text, "ax"
.global launch_user_demo
launch_user_demo:
    ldr x0, =__user_stack_top
    msr sp_el0, x0
    ldr x0, =__user_entry
    msr elr_el1, x0
    mov x0, #0
    msr spsr_el1, x0
    isb
    eret
