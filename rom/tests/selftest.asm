ORG $100
NOP
JP start

ORG $134
DB "SELFTEST"

ORG $147
DB $00
DB $00
DB $00

ORG $150
start:
  DI
  LD SP, $FFFE
  XOR A
  LDH ($40), A

  LD HL, $8000
  LD B, 16
  LD A, $FF
fill_tile:
  LD (HL), A
  INC HL
  DEC B
  JR NZ, fill_tile

  LD A, $E4
  LDH ($47), A

  LD A, $91
  LDH ($40), A

loop:
  JR loop
