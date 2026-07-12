ORG $40
LD HL, $C000
INC (HL)
RETI

ORG $100
NOP
JP start

ORG $134
DB "IRQTEST"

ORG $147
DB $00
DB $00
DB $00

ORG $150
start:
  DI
  LD SP, $FFFE
  XOR A
  LD ($C000), A
  XOR A
  LDH ($0F), A
  LD A, $01
  LDH ($FF), A
  EI
loop:
  HALT
  NOP
  JR loop
