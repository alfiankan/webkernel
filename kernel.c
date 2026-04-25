// kernel.c
void uartinit();
void print(char *s);
void kinit();
void kvminit();
void kvminithart();
void userinit(int pid, void (*func)());
void scheduler();
void user_a();
void user_b();

void main() {
  uartinit();
  print("SimpleOS Kernel Starting...\n");
  kinit();
  kvminit();
  kvminithart();
  print("Paging enabled!\n");
  
  userinit(0, user_a);
  userinit(1, user_b);
  print("Processes initialized. Starting scheduler...\n");
  
  // set default stvec
  void usertrap();
  asm volatile("csrw stvec, %0" : : "r" (usertrap));

  // enable interrupts
  asm volatile("csrs sstatus, %0" : : "r" (1L << 1)); // SIE bit
  asm volatile("csrs sie, %0" : : "r" (1L << 5));     // STIE bit

  scheduler();
}
