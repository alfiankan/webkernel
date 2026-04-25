# Simple RISC-V Kernel Scheduler Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a minimal RISC-V 64-bit kernel for QEMU with SV39 paging and a preemptive round-robin scheduler running two user-mode programs.

**Architecture:** The kernel runs in S-mode, handling traps and scheduling. SV39 paging provides isolated address spaces where user programs run at virtual address 0x0. Preemption is driven by timer interrupts.

**Tech Stack:** C, RISC-V Assembly, GNU toolchain (`riscv64-unknown-elf-gcc`), QEMU.

---

### Task 1: Project Skeleton & Bootstrapping

**Files:**
- Create: `Makefile`
- Create: `linker.ld`
- Create: `entry.S`
- Create: `start.c`
- Create: `kernel.c`

- [ ] **Step 1: Create Linker Script**
Map memory starting at 0x80000000 (QEMU virt RAM start).

```ld
OUTPUT_ARCH( "riscv" )
ENTRY( _entry )

SECTIONS
{
  . = 0x80000000;

  .text : {
    *(.text .text.*)
    . = ALIGN(0x1000);
    _trampoline = .;
    *(trampsec)
    . = ALIGN(0x1000);
    ASSERT(. - _trampoline == 0x1000, "error: trampoline larger than one page");
  }

  .rodata : {
    . = ALIGN(16);
    *(.srodata .srodata.*)
    . = ALIGN(16);
    *(.rodata .rodata.*)
  }

  .data : {
    . = ALIGN(16);
    *(.sdata .sdata.*)
    . = ALIGN(16);
    *(.data .data.*)
  }

  .bss : {
    . = ALIGN(16);
    *(.sbss .sbss.*)
    . = ALIGN(16);
    *(.bss .bss.*)
  }

  PROVIDE(end = .);
}
```

- [ ] **Step 2: Create Entry Assembly**
Setup stack and jump to C.

```assembly
# entry.S
.section .text
.global _entry
_entry:
    # set up a stack for C.
    # stack0 is declared in start.c,
    # with a 4096-byte stack per CPU.
    # sp = stack0 + (hartid * 4096)
    la sp, stack0
    li a0, 4096
    csrr a1, mhartid
    addi a1, a1, 1
    mul a0, a0, a1
    add sp, sp, a0
    # jump to start() in start.c
    call start
spin:
    j spin
```

- [ ] **Step 3: Create Start-up C (M-mode to S-mode)**

```c
// start.c
#include <stdint.h>

void main();

__attribute__ ((aligned (16))) char stack0[4096];

void start() {
  // set M Previous Privilege mode to Supervisor, for mret.
  unsigned long x = 0; // mstatus
  x &= ~0x1800; // MPP field
  x |= 0x0800;  // Supervisor
  asm volatile("csrw mstatus, %0" : : "r" (x));

  // set M Exception Program Counter to main, for mret.
  asm volatile("csrw mepc, %0" : : "r" (main));

  // disable paging for now.
  asm volatile("csrw satp, zero");

  // delegate all interrupts and exceptions to S-mode.
  asm volatile("csrw medeleg, %0" : : "r" (0xffff));
  asm volatile("csrw mideleg, %0" : : "r" (0xffff));

  // switch to S-mode and jump to main().
  asm volatile("mret");
}
```

- [ ] **Step 4: Create Kernel Main**

```c
// kernel.c
void main() {
  while(1);
}
```

- [ ] **Step 5: Create Makefile**

```makefile
K=kernel
U=user

OBJS = \
  entry.o \
  start.o \
  kernel.o

TOOLPREFIX = riscv64-unknown-elf-
CC = $(TOOLPREFIX)gcc
AS = $(TOOLPREFIX)gas
LD = $(TOOLPREFIX)ld
OBJCOPY = $(TOOLPREFIX)objcopy
OBJDUMP = $(TOOLPREFIX)objdump

CFLAGS = -Wall -Werror -O -fno-omit-frame-pointer -ggdb -gdwarf-2
CFLAGS += -mcmodel=medany
CFLAGS += -ffreestanding -fno-common -nostdlib -mno-relax
CFLAGS += -I.

LDFLAGS = -z max-page-size=4096

$K/kernel: $(OBJS) linker.ld
	$(LD) $(LDFLAGS) -T linker.ld -o kernel.elf $(OBJS)

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

%.o: %.S
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -f *.o kernel.elf

qemu: $K/kernel
	qemu-system-riscv64 -machine virt -bios none -kernel kernel.elf -nographic
```

- [ ] **Step 6: Verify Build**
Run `make kernel.elf`. It should compile.

- [ ] **Step 7: Commit**

---

### Task 2: UART and Printing

**Files:**
- Create: `uart.c`
- Modify: `kernel.c`
- Modify: `Makefile`

- [ ] **Step 1: Implement UART Driver**
Minimal 16550a UART driver for QEMU virt (base address 0x10000000).

```c
// uart.c
#include <stdint.h>

#define UART0 0x10000000L
#define REG(reg) ((volatile unsigned char *)(UART0 + reg))

void uartinit() {
  // disable interrupts.
  *REG(1) = 0x00;
  // special mode to set baud rate.
  *REG(3) = 0x80;
  // LSB for 38.4K baud.
  *REG(0) = 0x03;
  // MSB for 38.4K baud.
  *REG(1) = 0x00;
  // leave set-baud mode, and set word length to 8 bits, no parity.
  *REG(3) = 0x03;
}

void uartputc(int c) {
  while((*REG(5) & 0x20) == 0)
    ;
  *REG(0) = c;
}

void print(char *s) {
  while(*s) {
    uartputc(*s);
    s++;
  }
}
```

- [ ] **Step 2: Update Main to Print**

```c
// kernel.c
void uartinit();
void print(char *s);

void main() {
  uartinit();
  print("SimpleOS Kernel Starting...\n");
  while(1);
}
```

- [ ] **Step 3: Update Makefile**
Add `uart.o` to `OBJS`.

- [ ] **Step 4: Verify with QEMU**
Run `make qemu`. Expected: "SimpleOS Kernel Starting..." printed.

- [ ] **Step 5: Commit**

---

### Task 3: Memory Allocator & SV39 Paging

**Files:**
- Create: `vm.c`
- Create: `riscv.h` (Helper macros)
- Modify: `kernel.c`

- [ ] **Step 1: Create riscv.h**
Macros for PTE flags and page table indices.

```c
// riscv.h
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
```

- [ ] **Step 2: Implement simple allocator in vm.c**
A simple pointer to the end of the kernel binary.

```c
// vm.c
#include <stdint.h>
#include "riscv.h"

extern char end[]; // defined by linker
char *next_free = end;

void* kalloc() {
  void *p = (void*)next_free;
  next_free += PGSIZE;
  // Zero out the page
  for(int i = 0; i < PGSIZE; i++) ((char*)p)[i] = 0;
  return p;
}

// walk the 3-level page table
pte_t *walk(pagetable_t pagetable, uint64_t va, int alloc) {
  for(int level = 2; level > 0; level--) {
    pte_t *pte = &pagetable[PX(level, va)];
    if(*pte & PTE_V) {
      pagetable = (pagetable_t)PTE2PA(*pte);
    } else {
      if(!alloc || (pagetable = (pate_t*)kalloc()) == 0)
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
```

- [ ] **Step 3: Implement Kernel Page Table**
Identity map UART, RAM, and Kernel.

```c
// vm.c (continued)
pagetable_t kernel_pagetable;

void kvminit() {
  kernel_pagetable = (pagetable_t)kalloc();
  // UART
  mappages(kernel_pagetable, 0x10000000, PGSIZE, 0x10000000, PTE_R | PTE_W);
  // RAM (starting at 0x80000000, size 128MB)
  mappages(kernel_pagetable, 0x80000000, 0x8000000, 0x80000000, PTE_R | PTE_W | PTE_X);
}

void kvminithart() {
  asm volatile("csrw satp, %0" : : "r" ( (8L << 60) | (((uint64_t)kernel_pagetable) >> 12) ));
  asm volatile("sfence.vma zero, zero");
}
```

- [ ] **Step 4: Update Main**
Call `kvminit()` and `kvminithart()`.

- [ ] **Step 5: Verify**
Kernel should still boot and print UART message with paging enabled.

- [ ] **Step 6: Commit**

---

### Task 4: Process Structures and Context Switching

**Files:**
- Create: `proc.h`
- Create: `proc.c`
- Create: `swtch.S`
- Modify: `Makefile`

- [ ] **Step 1: Define Process Structures**

```c
// proc.h
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
```

- [ ] **Step 2: Implement switch_to in swtch.S**

```assembly
# swtch.S
.globl swtch
swtch:
    sd ra, 0(a0)
    sd sp, 8(a0)
    sd s0, 16(a0)
    sd s1, 24(a0)
    sd s2, 32(a0)
    sd s3, 40(a0)
    sd s4, 48(a0)
    sd s5, 56(a0)
    sd s6, 64(a0)
    sd s7, 72(a0)
    sd s8, 80(a0)
    sd s9, 88(a0)
    sd s10, 96(a0)
    sd s11, 104(a0)

    ld ra, 0(a1)
    ld sp, 8(a1)
    ld s0, 16(a1)
    ld s1, 24(a1)
    ld s2, 32(a1)
    ld s3, 40(a1)
    ld s4, 48(a1)
    ld s5, 56(a1)
    ld s6, 64(a1)
    ld s7, 72(a1)
    ld s8, 80(a1)
    ld s9, 88(a1)
    ld s10, 96(a1)
    ld s11, 104(a1)
    ret
```

- [ ] **Step 3: Implement Scheduler in proc.c**
Initial empty scheduler.

```c
// proc.c
#include "proc.h"

struct proc proc[2];
struct proc *current_proc;
struct context kernel_context;

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
```

- [ ] **Step 4: Commit**

---

### Task 5: Trap Handling (Interrupts & Syscalls)

**Files:**
- Create: `trap.c`
- Create: `trampoline.S`
- Modify: `Makefile`

- [ ] **Step 1: Create Trapframe**

```c
// riscv.h
struct trapframe {
  uint64_t kernel_satp;
  uint64_t kernel_sp;
  uint64_t kernel_trap;
  uint64_t epc;
  uint64_t kernel_hartid;
  uint64_t ra, sp, gp, tp, t0, t1, t2, s0, s1, a0, a1, a2, a3, a4, a5, a6, a7, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, t3, t4, t5, t6;
};
```

- [ ] **Step 2: Implement Trampoline Assembly**
Save user state, switch to kernel stack, jump to C handler.

```assembly
# trampoline.S
.section trampsec
.globl uservec
uservec:
    # swap a0 with sscratch (contains pointer to trapframe)
    csrrw a0, sscratch, a0
    # save registers to trapframe
    sd ra, 40(a0)
    sd sp, 48(a0)
    # ... (save all other regs) ...
    ld sp, 8(a0) # load kernel_sp
    ld t0, 16(a0) # load kernel_trap
    csrr t1, sstatus
    sd t1, 32(a0) # save sstatus
    jr t0

.globl userret
userret:
    # switch to user pagetable (passed in a0)
    csrw satp, a0
    sfence.vma zero, zero
    # restore registers from trapframe (pointer in a1)
    ld t1, 32(a1) # restore sstatus
    csrw sstatus, t1
    csrw sepc, 24(a1) # restore epc
    # ... (restore all other regs) ...
    sret
```

- [ ] **Step 3: Implement C Trap Handler**

```c
// trap.c
#include "riscv.h"
#include "proc.h"

void usertrap() {
  uint64_t scause = r_scause();
  if(scause == 8) { // syscall
    current_proc->trapframe->epc += 4;
    // syscall handler
    if(current_proc->trapframe->a7 == 1) { // print char
       uartputc(current_proc->trapframe->a0);
    }
  } else if(scause == (1L << 63 | 1)) { // software interrupt (timer)
    yield();
  }
  usertrapret();
}

void yield() {
  current_proc->state = READY;
  swtch(&current_proc->context, &kernel_context);
}
```

- [ ] **Step 4: Commit**

---

### Task 6: Example Programs & User-mode Jump

**Files:**
- Create: `user.c`
- Modify: `proc.c`
- Modify: `kernel.c`

- [ ] **Step 1: Create User Example Programs**

```c
// user.c
void user_a() {
  while(1) {
    asm volatile("li a7, 1; li a0, 'A'; ecall");
    for(int i = 0; i < 1000000; i++);
  }
}

void user_b() {
  while(1) {
    asm volatile("li a7, 1; li a0, 'B'; ecall");
    for(int i = 0; i < 1000000; i++);
  }
}
```

- [ ] **Step 2: Implement Process Allocation & Loading**
Map user program at 0x0 and setup trapframe.

```c
// proc.c
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

  p->context.ra = (uint64_t)forkret;
  p->context.sp = p->kstack + PGSIZE;
  p->trapframe->epc = 0;
  p->trapframe->sp = PGSIZE; // user stack
}
```

- [ ] **Step 3: Implement forkret and first return**
Function that jumps to `usertrapret`.

- [ ] **Step 4: Update Main**
Call `userinit(0, user_a)` and `userinit(1, user_b)`, then `scheduler()`.

- [ ] **Step 5: Verify**
Should see 'A' and 'B' printing alternating (manually triggered first).

- [ ] **Step 6: Commit**

---

### Task 7: Timer Preemption

**Files:**
- Modify: `start.c`
- Modify: `kernel.c`
- Modify: `trap.c`

- [ ] **Step 1: Setup Timer in start.c**
Configure CLINT to fire every X ticks.

- [ ] **Step 2: Enable Interrupts**
Enable S-mode timer interrupts in `sstatus` and `sie`.

- [ ] **Step 3: Run Final QEMU Test**
`make qemu`. Success = `ABABABABAB...`

- [ ] **Step 4: Commit**
