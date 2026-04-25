// trap.c
#include "riscv.h"
#include "proc.h"

void uartputc(int c);
void print(char *s);
void uservec();
void userret(uint64_t, uint64_t);
void swtch(struct context*, struct context*);
void usertrapret();
void yield();

void usertrap() {
  uint64_t scause = r_scause();
  if(scause == 8) { // syscall
    current_proc->trapframe->epc += 4;
    if(current_proc->trapframe->a7 == 1) { // print char
       uartputc(current_proc->trapframe->a0);
    } else if(current_proc->trapframe->a7 == 2) { // yield
       yield();
    }
  } else if(scause == (1L << 63 | 5)) { // software/timer interrupt (STIP=5)
    // reset timer
    unsigned long clint = 0x2000000;
    *(uint64_t*)(clint + 0x4000) = *(uint64_t*)(clint + 0xBFF8) + 1000000;
    yield();
  }
  usertrapret();
}

void yield() {
  current_proc->state = READY;
  swtch(&current_proc->context, &kernel_context);
}

void usertrapret() {
  current_proc->state = RUNNING;

  // set up trapframe for next trap
  extern pagetable_t kernel_pagetable;
  current_proc->trapframe->kernel_satp = (8L << 60) | (((uint64_t)kernel_pagetable) >> 12);
  current_proc->trapframe->kernel_sp = current_proc->kstack + PGSIZE;
  current_proc->trapframe->kernel_trap = (uint64_t)usertrap;

  // set stvec to trampoline uservec
  uint64_t trampoline_uservec = 0x3FFFFFF000;
  asm volatile("csrw stvec, %0" : : "r" (trampoline_uservec));

  uint64_t satp = (8L << 60) | (((uint64_t)current_proc->pagetable) >> 12);
  uint64_t trapframe_va = 0x3FFFFFE000;

  // jump to trampoline userret
  uint64_t trampoline_userret = 0x3FFFFFF000 + ((uint64_t)userret - (uint64_t)uservec);
  ((void (*)(uint64_t, uint64_t))trampoline_userret)(satp, trapframe_va);
}
