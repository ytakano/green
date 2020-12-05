.global asm_spawn
.global set_context
.global switch_context
.global enter_entry

asm_spawn:
    stp x30, xzr, [sp, #-16]!
    blr x0
    ldp x30, xzr, [sp], #16
    ret

set_context:
    stp d8, d9, [x0]
    stp d10, d11, [x0, #16]
    stp d12, d13, [x0, #16 * 2]
    stp d14, d15, [x0, #16 * 3]
    stp x19, x20, [x0, #16 * 4]
    stp x21, x22, [x0, #16 * 5]
    stp x23, x24, [x0, #16 * 6]
    stp x25, x26, [x0, #16 * 7]
    stp x27, x28, [x0, #16 * 8]
    mov x1, sp
    stp x30, x1, [x0, #16 * 9]

    // return 0
    mov x0, 0
    ret

switch_context:
    ldp d8, d9, [x0]
    ldp d10, d11, [x0, #16]
    ldp d12, d13, [x0, #16 * 2]
    ldp d14, d15, [x0, #16 * 3]
    ldp x19, x20, [x0, #16 * 4]
    ldp x21, x22, [x0, #16 * 5]
    ldp x23, x24, [x0, #16 * 6]
    ldp x25, x26, [x0, #16 * 7]
    ldp x27, x28, [x0, #16 * 8]
    ldp x30, x2, [x0, #16 * 9]
    mov sp, x2

    // return 1
    mov x0, 1
    ret
