OBJS = \
  entry.o \
  start.o \
  kernel.o \
  uart.o \
  vm.o \
  string.o \
  proc.o \
  swtch.o \
  trap.o \
  trampoline.o \
  user.o

TOOLPREFIX = riscv64-elf-
CC = $(TOOLPREFIX)gcc
AS = $(TOOLPREFIX)gcc
LD = $(TOOLPREFIX)ld
OBJCOPY = $(TOOLPREFIX)objcopy
OBJDUMP = $(TOOLPREFIX)objdump

CFLAGS = -Wall -Werror -O -fno-omit-frame-pointer -ggdb -gdwarf-2
CFLAGS += -mcmodel=medany
CFLAGS += -ffreestanding -fno-common -nostdlib -mno-relax
CFLAGS += -I.

LDFLAGS = -z max-page-size=4096

kernel.elf: $(OBJS) linker.ld
	$(LD) $(LDFLAGS) -T linker.ld -o kernel.elf $(OBJS)

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

%.o: %.S
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -f *.o kernel.elf

qemu: kernel.elf
	qemu-system-riscv64 -machine virt -bios none -kernel kernel.elf -nographic
