CROSS_COMPILE ?= riscv64-unknown-elf-
CC = $(CROSS_COMPILE)gcc
AS = $(CROSS_COMPILE)as
LD = $(CROSS_COMPILE)ld
OBJCOPY = $(CROSS_COMPILE)objcopy

CFLAGS = -Wall -Wextra -mcmodel=medany -ffreestanding -nostdlib -mabi=lp64 -march=rv64imafdc
LDFLAGS = -T linker.ld

kernel.elf: kernel.S linker.ld
	$(CC) $(CFLAGS) $(LDFLAGS) kernel.S -o kernel.elf

run: kernel.elf
	qemu-system-riscv64 -M virt -cpu rv64 -nographic -kernel kernel.elf

clean:
	rm -f kernel.elf

help:
	@echo "Usage:"
	@echo "  make                - Build the kernel"
	@echo "  make run            - Run the kernel in QEMU"
	@echo "  make CROSS_COMPILE=riscv64-linux-gnu-  - Build using a different toolchain prefix"
