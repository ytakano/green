.global spawn

spawn:
    stp x30, xzr, [sp, #-16]
    br  x0
    ldp x30, xzr, [sp]
    ret
