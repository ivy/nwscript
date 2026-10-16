# NCS assembly

The text form of NCS bytecode that `nws disasm` prints and `nws asm` reads. Assembling a disassembly gives back the original file byte for byte; the test suite checks this for every script in the vanilla KotOR corpus.

```text
; 5 instructions, 44 bytes
    JSR          sub_15
    RETN
sub_15:
    CONSTS       "k_act_mispaz"
    ACTION       1, 1                 ; PrintString
    RETN
```

## Lines

Each line holds at most one instruction, optionally preceded by labels. Whitespace between parts is free, and blank lines are ignored.

- **Comments** run from `;` to the end of the line, except inside a string.
- **Labels** are `name:`, where the name starts with a letter or `_` and continues with letters, digits and `_`. A label marks the instruction that follows it, or the end of the program when nothing follows. A label may share a line with its instruction. Defining a label twice is error E0113.
- **Instructions** are a mnemonic and its operands, separated by commas.
- **Offsets**: a line may start with a decimal file offset and a colon, `    13: JSR +8`, as `nws disasm --raw` writes. The assembler ignores the column; offsets come from the instructions themselves.

The text must be ASCII. Bytes above 0x7F, which NWScript strings carry in Windows-1252, are written as `\xNN` escapes.

## Mnemonics

A mnemonic is the opcode's name and the suffix its type byte selects: `I` int, `F` float, `S` string, `O` object, `En` engine structure `n`, `II` `FF` `OO` `SS` `TT` (struct) `IF` `FI` `VV` `VF` `FV` for pairs, `EEn` a pair of engine structures `n`. Where the type byte is fixed, the bare name is the mnemonic. An unknown mnemonic is error E0110.

| Opcode | Mnemonics |
| --- | --- |
| `0x01` CPDOWNSP | `CPDOWNSP` |
| `0x02` RSADD | `RSADDI` `RSADDF` `RSADDS` `RSADDO` `RSADDE0` `RSADDE1` `RSADDE2` `RSADDE3` `RSADDE4` `RSADDE5` `RSADDE6` `RSADDE7` `RSADDE8` `RSADDE9` `RSADDE10` `RSADDE11` `RSADDE12` `RSADDE13` `RSADDE14` `RSADDE15` |
| `0x03` CPTOPSP | `CPTOPSP` |
| `0x04` CONST | `CONSTI` `CONSTF` `CONSTS` `CONSTO` |
| `0x05` ACTION | `ACTION` |
| `0x06` LOGAND | `LOGANDII` |
| `0x07` LOGOR | `LOGORII` |
| `0x08` INCOR | `INCORII` |
| `0x09` EXCOR | `EXCORII` |
| `0x0a` BOOLAND | `BOOLANDII` |
| `0x0b` EQUAL | `EQUALII` `EQUALFF` `EQUALOO` `EQUALSS` `EQUALTT` `EQUALEE0` `EQUALEE1` `EQUALEE2` `EQUALEE3` `EQUALEE4` `EQUALEE5` `EQUALEE6` `EQUALEE7` `EQUALEE8` `EQUALEE9` |
| `0x0c` NEQUAL | `NEQUALII` `NEQUALFF` `NEQUALOO` `NEQUALSS` `NEQUALTT` `NEQUALEE0` `NEQUALEE1` `NEQUALEE2` `NEQUALEE3` `NEQUALEE4` `NEQUALEE5` `NEQUALEE6` `NEQUALEE7` `NEQUALEE8` `NEQUALEE9` |
| `0x0d` GEQ | `GEQII` `GEQFF` |
| `0x0e` GT | `GTII` `GTFF` |
| `0x0f` LT | `LTII` `LTFF` |
| `0x10` LEQ | `LEQII` `LEQFF` |
| `0x11` SHLEFT | `SHLEFTII` |
| `0x12` SHRIGHT | `SHRIGHTII` |
| `0x13` USHRIGHT | `USHRIGHTII` |
| `0x14` ADD | `ADDII` `ADDFF` `ADDSS` `ADDIF` `ADDFI` `ADDVV` |
| `0x15` SUB | `SUBII` `SUBFF` `SUBIF` `SUBFI` `SUBVV` |
| `0x16` MUL | `MULII` `MULFF` `MULIF` `MULFI` `MULVF` `MULFV` |
| `0x17` DIV | `DIVII` `DIVFF` `DIVIF` `DIVFI` `DIVVF` |
| `0x18` MOD | `MODII` |
| `0x19` NEG | `NEGI` `NEGF` |
| `0x1a` COMP | `COMPI` |
| `0x1b` MOVSP | `MOVSP` |
| `0x1c` STORE_STATEALL | `STORE_STATEALL` |
| `0x1d` JMP | `JMP` |
| `0x1e` JSR | `JSR` |
| `0x1f` JZ | `JZ` |
| `0x20` RETN | `RETN` |
| `0x21` DESTRUCT | `DESTRUCT` |
| `0x22` NOT | `NOTI` |
| `0x23` DECISP | `DECISP` |
| `0x24` INCISP | `INCISP` |
| `0x25` JNZ | `JNZ` |
| `0x26` CPDOWNBP | `CPDOWNBP` |
| `0x27` CPTOPBP | `CPTOPBP` |
| `0x28` DECIBP | `DECIBP` |
| `0x29` INCIBP | `INCIBP` |
| `0x2a` SAVEBP | `SAVEBP` |
| `0x2b` RESTOREBP | `RESTOREBP` |
| `0x2c` STORE_STATE | `STORE_STATE` |
| `0x2d` NOP | `NOP` |

`STORE_STATE`'s type byte is the distance to the code the saved state resumes at, always 0x10 in BioWare's bytecode, so it has no suffix and the assembler writes 0x10. `STORE_STATEALL` is the same with 0x08.

## Operands

| Mnemonics | Operands |
| --- | --- |
| `CPDOWNSP` `CPTOPSP` `CPDOWNBP` `CPTOPBP` | offset (signed 32-bit), size (unsigned 16-bit) |
| `CONSTI` | a signed 32-bit integer |
| `CONSTF` | a float: decimal as Rust writes it (`1.0`, `-0.0`, `1e-7`, `inf`), or `bits(0x7fc00001)` for an exact bit pattern; the disassembler uses `bits` only for NaN |
| `CONSTS` | a quoted string with escapes `\"`, `\\`, `\n`, `\r`, `\t`, `\xNN` |
| `CONSTO` | a signed 32-bit object id: 0 is `OBJECT_SELF`, 1 is `OBJECT_INVALID` |
| `ACTION` | the engine routine's number (its position among `nwscript.nss`'s prototypes, from 0) and the argument count; the disassembler adds the routine's name as a comment when it has a `nwscript.nss` |
| `EQUALTT` `NEQUALTT` | the struct's size in bytes |
| `MOVSP` `DECISP` `INCISP` `DECIBP` `INCIBP` | a signed 32-bit offset |
| `JMP` `JSR` `JZ` `JNZ` | a label, or a signed offset from the start of the instruction such as `+8` or `-24` |
| `DESTRUCT` | size, keep offset, keep size (each unsigned 16-bit) |
| `STORE_STATE` | bytes saved from the globals, bytes saved from the locals |

Integers may be decimal or `0x` hexadecimal, with an optional sign. A wrong operand count or a value of the wrong kind is error E0111, and a jump to a label that is not defined is E0112.

## Labels the disassembler writes

Every jump target that starts an instruction, or the end of the program, gets a label named by its file offset in hexadecimal: `sub_15` for the target of a `JSR`, `L_2a` for the others. A target in the middle of an instruction keeps its relative offset, so the round trip holds even for bytecode no compiler would write. `nws disasm --raw` writes no labels: every line carries its offset, and every jump its relative offset.

The file header and the program size record are not written; the assembler computes them.
