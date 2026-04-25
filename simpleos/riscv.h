// riscv.h
#ifndef RISCV_H
#define RISCV_H

#include <stdint.h>

#define PGSIZE 4096
#define PGROUNDUP(sz)  (((sz)+PGSIZE-1) & ~(PGSIZE-1))
#define PGROUNDDOWN(a) (((a)) & ~(PGSIZE-1))

#define PTE_V (1L << 0) // valid
#define PTE_R (1L << 1) // readable
#define PTE_W (1L << 2) // writable
#define PTE_X (1L << 3) // executable
#define PTE_U (1L << 4) // user

typedef uint64_t pte_t;
typedef uint64_t *pagetable_t;

// extract the three 9-bit page table indices from a virtual address.
#define PXMASK          0x1FF
#define PXSHIFT(level)  (12 + (9*(level)))
#define PX(level, va) ((((uint64_t) (va)) >> PXSHIFT(level)) & PXMASK)

// convert physical address to page table entry.
#define PA2PTE(pa) ((((uint64_t)pa) >> 12) << 10)
#define PTE2PA(pte) (((pte) >> 10) << 12)

struct trapframe {
  uint64_t kernel_satp;   // 0
  uint64_t kernel_sp;     // 8
  uint64_t kernel_trap;   // 16
  uint64_t epc;           // 24
  uint64_t kernel_hartid; // 32
  uint64_t sstatus;       // 40
  uint64_t ra, sp, gp, tp, t0, t1, t2, s0, s1, a0, a1, a2, a3, a4, a5, a6, a7, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, t3, t4, t5, t6;
};

static inline uint64_t r_scause() {
  uint64_t x;
  asm volatile("csrr %0, scause" : "=r" (x) );
  return x;
}

#endif
