# trampoline.S
.section trampsec
.globl uservec
.globl userret
.align 4

uservec:
    # swap a0 with sscratch (contains pointer to trapframe)
    csrrw a0, sscratch, a0
    # save registers to trapframe
    sd ra, 48(a0)
    sd sp, 56(a0)
    sd gp, 64(a0)
    sd tp, 72(a0)
    sd t0, 80(a0)
    sd t1, 88(a0)
    sd t2, 96(a0)
    sd s0, 104(a0)
    sd s1, 112(a0)
    sd a1, 128(a0)
    sd a2, 136(a0)
    sd a3, 144(a0)
    sd a4, 152(a0)
    sd a5, 160(a0)
    sd a6, 168(a0)
    sd a7, 176(a0)
    sd s2, 184(a0)
    sd s3, 192(a0)
    sd s4, 200(a0)
    sd s5, 208(a0)
    sd s6, 216(a0)
    sd s7, 224(a0)
    sd s8, 232(a0)
    sd s9, 240(a0)
    sd s10, 248(a0)
    sd s11, 256(a0)
    sd t3, 264(a0)
    sd t4, 272(a0)
    sd t5, 280(a0)
    sd t6, 288(a0)

    # save swapped a0 (original a0)
    csrr t0, sscratch
    sd t0, 120(a0)

    # save sstatus
    csrr t0, sstatus
    sd t0, 40(a0)

    # save sepc
    csrr t0, sepc
    sd t0, 24(a0)

    # restore kernel stack, satp, trap handler
    ld sp, 8(a0)
    ld t0, 16(a0)
    ld t1, 0(a0) # kernel_satp

    csrw satp, t1
    sfence.vma zero, zero

    jr t0

userret:
    # switch to user pagetable (passed in a0)
    # a1 is pointer to trapframe
    csrw satp, a0
    sfence.vma zero, zero

    # restore sstatus and sepc
    ld t0, 40(a1)
    csrw sstatus, t0
    ld t0, 24(a1)
    csrw sepc, t0

    # restore general purpose registers
    ld ra, 48(a1)
    ld sp, 56(a1)
    ld gp, 64(a1)
    ld tp, 72(a1)
    ld t0, 80(a1)
    ld t1, 88(a1)
    ld t2, 96(a1)
    ld s0, 104(a1)
    ld s1, 112(a1)
    ld a0, 120(a1)
    # ld a1... restore a1 last
    ld a2, 136(a1)
    ld a3, 144(a1)
    ld a4, 152(a1)
    ld a5, 160(a1)
    ld a6, 168(a1)
    ld a7, 176(a1)
    ld s2, 184(a1)
    ld s3, 192(a1)
    ld s4, 200(a1)
    ld s5, 208(a1)
    ld s6, 216(a1)
    ld s7, 224(a1)
    ld s8, 232(a1)
    ld s9, 240(a1)
    ld s10, 248(a1)
    ld s11, 256(a1)
    ld t3, 264(a1)
    ld t4, 272(a1)
    ld t5, 280(a1)
    ld t6, 288(a1)

    # swap a1 with sscratch for next trap
    csrw sscratch, a1
    ld a1, 128(a1) # restore a1

    sret
