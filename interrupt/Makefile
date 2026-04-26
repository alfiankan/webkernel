CC = riscv64-unknown-elf-gcc
AS = riscv64-unknown-elf-as
LD = riscv64-unknown-elf-ld
OBJCOPY = riscv64-unknown-elf-objcopy

CFLAGS = -Wall -Wextra -mcmodel=medany -ffreestanding -nostdlib -mabi=lp64 -march=rv64imafdc
LDFLAGS = -T linker.ld

kernel.elf: kernel.S linker.ld
	$(CC) $(CFLAGS) $(LDFLAGS) kernel.S -o kernel.elf

run: kernel.elf
	qemu-system-riscv64 -M virt -cpu rv64 -nographic -kernel kernel.elf

clean:
	rm -f kernel.elf
