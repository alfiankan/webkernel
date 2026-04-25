// start.c
#include <stdint.h>

void main();
void uartinit();
void uartputc(int c);

__attribute__ ((aligned (16))) char stack0[4096];

void start() {
  uartinit();
  uartputc('S'); // Print 'S' in M-mode

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
  asm volatile("csrs mie, %0" : : "r" (1L << 7)); // MTIE

  // give S-mode access to all of physical memory.
  asm volatile("csrw pmpaddr0, %0" : : "r" (0x3fffffffffffffull));
  asm volatile("csrw pmpcfg0, %0" : : "r" (0xf));

  // ask for clock interrupts in S-mode.
  unsigned long clint = 0x2000000;
  *(uint64_t*)(clint + 0x4000) = *(uint64_t*)(clint + 0xBFF8) + 1000000;

  // switch to S-mode and jump to main().
  asm volatile("mret");
}
