// user.c
__attribute__((aligned(4096)))
void user_a() {
  while(1) {
    asm volatile("li a7, 1; li a0, 'A'; ecall");
    asm volatile("li a7, 2; ecall"); // yield
    for(int i = 0; i < 100000; i++);
  }
}

__attribute__((aligned(4096)))
void user_b() {
  while(1) {
    asm volatile("li a7, 1; li a0, 'B'; ecall");
    asm volatile("li a7, 2; ecall"); // yield
    for(int i = 0; i < 100000; i++);
  }
}
