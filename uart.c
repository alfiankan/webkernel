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
