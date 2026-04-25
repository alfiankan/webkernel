// proc.h
#ifndef PROC_H
#define PROC_H

#include "riscv.h"

struct context {
  uint64_t ra;
  uint64_t sp;
  // callee-saved
  uint64_t s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11;
};

enum procstate { UNUSED, READY, RUNNING };

struct proc {
  enum procstate state;
  int pid;
  uint64_t kstack;
  pagetable_t pagetable;
  struct context context;
  struct trapframe *trapframe;
};

extern struct proc proc[2];
extern struct proc *current_proc;
extern struct context kernel_context;

#endif
