// vm.c
#include <stdint.h>
#include "riscv.h"

void* memset(void*, int, uint64_t);

extern char end[]; // defined by linker
char *next_free = 0;

void kinit() {
  next_free = (char*)PGROUNDUP((uint64_t)end);
}

void* kalloc() {
  void *p = (void*)next_free;
  next_free += PGSIZE;
  // Zero out the page
  memset(p, 0, PGSIZE);
  return p;
}

// walk the 3-level page table
pte_t *walk(pagetable_t pagetable, uint64_t va, int alloc) {
  for(int level = 2; level > 0; level--) {
    pte_t *pte = &pagetable[PX(level, va)];
    if(*pte & PTE_V) {
      pagetable = (pagetable_t)PTE2PA(*pte);
    } else {
      if(!alloc || (pagetable = (pagetable_t)kalloc()) == 0)
        return 0;
      *pte = PA2PTE(pagetable) | PTE_V;
    }
  }
  return &pagetable[PX(0, va)];
}

int mappages(pagetable_t pagetable, uint64_t va, uint64_t size, uint64_t pa, int perm) {
  uint64_t a, last;
  pte_t *pte;

  a = PGROUNDDOWN(va);
  last = PGROUNDDOWN(va + size - 1);
  for(;;){
    if((pte = walk(pagetable, a, 1)) == 0)
      return -1;
    *pte = PA2PTE(pa) | perm | PTE_V;
    if(a == last)
      break;
    a += PGSIZE;
    pa += PGSIZE;
  }
  return 0;
}

pagetable_t kernel_pagetable;

void kvminit() {
  kernel_pagetable = (pagetable_t)kalloc();
  // UART
  mappages(kernel_pagetable, 0x10000000, PGSIZE, 0x10000000, PTE_R | PTE_W);
  // CLINT
  mappages(kernel_pagetable, 0x2000000, 0x10000, 0x2000000, PTE_R | PTE_W);
  // RAM (starting at 0x80000000, size 128MB)
  mappages(kernel_pagetable, 0x80000000, 0x8000000, 0x80000000, PTE_R | PTE_W | PTE_X);
  
  // Trampoline
  extern char _trampoline[];
  mappages(kernel_pagetable, 0x3FFFFFF000, PGSIZE, (uint64_t)_trampoline, PTE_R | PTE_X);
}

void kvminithart() {
  asm volatile("csrw satp, %0" : : "r" ( (8L << 60) | (((uint64_t)kernel_pagetable) >> 12) ));
  asm volatile("sfence.vma zero, zero");
}
