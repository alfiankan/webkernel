// proc.c
#include "proc.h"

struct proc proc[2];
struct proc *current_proc;
struct context kernel_context;

void swtch(struct context*, struct context*);
void* kalloc();
int mappages(pagetable_t, uint64_t, uint64_t, uint64_t, int);
void* memmove(void*, const void*, uint64_t);
void usertrapret();

extern char _trampoline[];

void forkret() {
  usertrapret();
}

void userinit(int pid, void (*func)()) {
  struct proc *p = &proc[pid];
  p->state = READY;
  p->pid = pid;
  p->kstack = (uint64_t)kalloc();
  p->pagetable = (pagetable_t)kalloc();
  p->trapframe = (struct trapframe*)kalloc();

  // Map trampoline at top of VA space
  mappages(p->pagetable, 0x3FFFFFF000, PGSIZE, (uint64_t)_trampoline, PTE_R | PTE_X);
  // Map trapframe just below trampoline
  mappages(p->pagetable, 0x3FFFFFE000, PGSIZE, (uint64_t)p->trapframe, PTE_R | PTE_W);

  // Map user code at 0x0
  uint64_t upage = (uint64_t)kalloc();
  memmove((void*)upage, (void*)func, PGSIZE);
  mappages(p->pagetable, 0, PGSIZE, upage, PTE_R | PTE_W | PTE_X | PTE_U);

  // Map user stack at PGSIZE
  uint64_t ustack = (uint64_t)kalloc();
  mappages(p->pagetable, PGSIZE, PGSIZE, ustack, PTE_R | PTE_W | PTE_U);

  p->context.ra = (uint64_t)forkret;
  p->context.sp = p->kstack + PGSIZE;
  p->trapframe->epc = (uint64_t)func & (PGSIZE - 1);
  p->trapframe->sp = PGSIZE * 2; // top of stack
  p->trapframe->sstatus = 0x20; // SPP=0 (User), SPIE=1
}

void scheduler() {
  current_proc = 0;
  for(;;){
    for(int i = 0; i < 2; i++){
      if(proc[i].state != READY) continue;
      proc[i].state = RUNNING;
      current_proc = &proc[i];
      swtch(&kernel_context, &proc[i].context);
      // coming back from proc
      current_proc = 0;
    }
  }
}
