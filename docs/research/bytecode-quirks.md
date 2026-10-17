---
title: Quirks in BioWare's KotOR bytecode and sources
date: 2026-10-16
status: measured
---

# Quirks in BioWare's KotOR bytecode and sources

What building the toolchain has turned up about the scripts BioWare shipped with KotOR 1: behaviour of its compiler that a compatible one must reproduce, defects in the shipped sources that a strict one must report, and traps for anyone reading the bytecode. Each entry says how it was found and how to see it again. The corpus is the one `cargo xtask corpus extract` writes; file names below are its paths.

The grammar's own findings (literal forms, precedence evidence, what the corpus never uses) are in [`../grammar.md`](../grammar.md). The compilers' measurement is in [`nwnnsscomp-conformance.md`](nwnnsscomp-conformance.md).

## Code generation

### `||` evaluates both operands; `&&` short-circuits

`&&` jumps over its right operand when the left one is zero. `||` does not skip its right operand when the left one is true: it copies the left value, jumps to the right operand if it is zero, and on the other path copies it again and jumps to the `LOGOR` if *that* copy is zero, which it cannot be. Either way the right operand runs, and `LOGORII` combines the two. A right operand with side effects, such as a call that changes state, runs every time.

`ncs/k_pdan_mand04.ncs`, from `return bCondition == 0 || bCondition == 1;`:

```text
    77: CPTOPSP      -4, 4      ; copy the left value
    85: JZ           +20        ; zero: go to the right operand at 105
    91: CPTOPSP      -4, 4      ; non-zero: copy it again
    99: JZ           +22        ; never taken; the target is LOGORII at 121
   105: CPTOPSP      -8, 4      ; the right operand
   113: CONSTI       1
   119: EQUALII
   121: LOGORII
```

`ncs/k_con_canderpm.ncs` shows `&&` for contrast: one copy, one `JZ` straight past the right operand and the `LOGANDII`, leaving the zero as the result.

The second jump's target would reach `LOGORII` with one operand on the stack, so a control-flow analysis that follows it finds stack depths that disagree. `nwscript_ncs::cfg::build` recognises the exact five-instruction shape and records the edge in `Cfg::infeasible` rather than as a successor. nwnnsscomp with `-v1.00` emits the same shape; it is part of BioWare's bytecode a compiler must reproduce, and part of what a decompiler must read as `||`.

### Dead code after `return`

6,841 of the 13,139 NCS files hold code no path reaches. A `return` jumps to the function's epilogue, and the cleanup the compiler emits for the end of the enclosing block stays behind the jump. The smallest case is `modules/danm13_s/false.ncs`, a `StartingConditional` returning `FALSE`, where the `MOVSP -4` at offset 49 sits between a `JMP` and the `RETN` it jumps to. Byte-identical output has to emit it; a decompiler has to drop it.

### `break` out of a `case` that declared a local leaves the stack unbalanced

A local declared directly inside a `case` is cleaned up by a `MOVSP` at the end of the switch, but a `break` from that case jumps past the cleanup. The bytecode then reaches the instruction after the switch with two different stack depths. Six scripts built from the default user-defined-event template have it: `ncs/k_pkas_randwalk`, `modules/tat_m18ac_s/k_ptat_dewbuser`, `modules/tat_m18ac_s/k_pkas_banthwalk`, `modules/tar_m02ab_s/k_ptar_drunk_ud`, `modules/lev_m40ac_s/k_plev_fcpush_ud`, `modules/lev_m40ac_s/k_plev_fpush2_ud`. In `k_pkas_randwalk.nss` the local is `object oCreature`, declared under `case 4:`. These scripts run in the shipped game, so the engine tolerates the extra four bytes. A compiler reproducing BioWare's bytes must reproduce the imbalance, and a strict one should warn about the source.

The stack-depth analysis over the corpus (`crates/nwscript-ncs/tests/cfg.rs`) finds every other file consistent at every `RETN`; these six and the next entry are the only exceptions, pinned in that test.

### `STORE_STATE`'s type byte is a jump distance

For every other instruction the second byte is a type. For `STORE_STATE` it is the distance from the instruction to the code the deferred action resumes at, and in BioWare's bytecode it is always `0x10`: past the 10-byte `STORE_STATE` and the 6-byte `JMP` that skips the action's body. The decoder accepts only `0x10`, as Torlack's specification gives it.

### Object constants are 0 and 1

`OBJECT_SELF` compiles to `CONSTO 0` and `OBJECT_INVALID` to `CONSTO 1`; no other `CONSTO` operand appears in the corpus. `k_sup_gohawk` and `k_sup_guiopen`, which use `OBJECT_INVALID` and not `OBJECT_SELF`, hold only `CONSTO 1`.

## Running the bytecode

### BP points at the saved BP

`SAVEBP` sets BP to the stack slot it saves the old BP into, so the last global variable is at `BP-4` and the first at `BP-4×n`. Treating BP as the slot above the saved one is off by one element, and it shows: in `modules/danm13_s/k_pdan_elise10.ncs`, `CPTOPBP -32, 4` must read the integer global just below the string `"bastila"`, which `SetGlobalNumber` then receives. With BP one slot higher the call gets the string. A deferred action's saved globals are the elements below BP, without the saved BP.

### Module bytecode compiled against other declarations

Fourteen module scripts call engine routines with arguments the shipped `nwscript.nss` does not declare for them, so their bytecode was compiled against an earlier table:

- `k_act_carthjoin`, in eleven Taris module capsules, and `k_ptar_missjoin` (`tar_m04aa_s`) call routine 574 with an object alone. The shipped declaration is `int AddPartyMember(int nNPC, object oCreature)`; the source-level failures of `k_act_*rmv` and `k_act_bastadd` in the conformance report are the same change seen from the other side.
- `k_plev_corpse1`, in two Leviathan capsules, calls routine 700 with an object and an int. The shipped routine 700 is `void ActionBarkString(int strRef)`, one argument.

The interpreter reports these as argument mismatches rather than machine faults (`FaultKind::ArgumentMismatch`).

### Division by a defaulted zero

Under an engine that answers every routine with its type's default, 158 module scripts divide by zero: `k_pdan_attack` divides `IntToFloat(GetCurrentHitPoints())` by `IntToFloat(GetMaxHitPoints())`, and the default maximum is 0. What the game's machine does on a float division by zero is not known; the interpreter stops with a fault marked as depending on the engine's answers.

## The shipped sources

### `nwscript.nss` line 5145

`void SetAvailableNPCId(INT nNPC, OBJECT_ID oidNPC);` has its types in capitals. nwnnsscomp loops forever on it instead of reporting an error, which is why the corpus carries an `oracle/` copy with the line corrected. The line still counts as a prototype: an engine routine's number in `ACTION` is its position among the prototypes from zero, and this one is number 767. No vanilla script calls it, so the slot has no reference bytes.

### Engine routine numbers are prototype positions

`ACTION`'s routine operand is the index of the function's prototype in `nwscript.nss`, counting from zero and counting the malformed line above. `Random` is 0 and `PrintString` is 1. The grammar census checked this against the bytecode for the calls past position 600; the two misses are in `k_trg_transfail1`, whose shipped NCS is older than its shipped source.

### Scripts compiled against declarations that did not ship

Eighteen scripts fail in every compiler because the declarations they were compiled against are not the ones that shipped: ten call `AddPartyMember` or `RemovePartyMember` with an argument the shipped prototype rejects, five include `k_inc_ebonhawk`, whose forward declaration of `EBO_PlayTakeOff` disagrees with its definition, and three use names the shipped sources never declare. `conformance/known.toml` lists them by cause, and the conformance report shows the cause beside each failure.

### `ActionStartConversation` had eleven parameters

The shipped `nwscript.nss` declares `ActionStartConversation` with twelve parameters, the last an int defaulting to 0. BioWare's bytecode passes eleven: every call compiled by BioWare leaves the twelfth out, so those scripts were compiled when the routine had eleven. A compiler working from the shipped declaration pushes one more `CONSTI 0`, six bytes, which is the whole of the largest cluster of nwnnsscomp's different-length outputs (32 scripts, +6 bytes each). The conformance report's trace comparison shows it directly, in `k_pend_door01` for one:

```text
BioWare's   ActionStartConversation(OBJECT_INVALID, "end_trask01", 0, 0, 1, "", "", "", "", "", "")
nwnnsscomp  ActionStartConversation(OBJECT_INVALID, "end_trask01", 0, 0, 1, "", "", "", "", "", "", 0)
```

Of the 32, 17 differ in their trace; in the other 15 the call sits on a path the recording engine does not reach. Reproducing BioWare's bytes takes the eleven-parameter declaration; the engine evidently accepts both.

### Bytecode older than its source

Some shipped NCS was compiled from an earlier version of the source that ships beside it. `k_pla_actmap.ncs` posts `"K_STAR_MAP Before = "` where `k_pla_actmap.nss` writes `"v3.0 - K_STAR_MAP Before = "`; `k_trg_transfail1.ncs` calls routines its source does not (see the grammar census). The `k_act_com3*` family, which includes `k_inc_generic`, makes different calls from its first, which points the same way for the include. These scripts cannot be reproduced from the shipped sources by any compiler.

### Neverwinter Nights leftovers

`nw_g0_conversat` and `nw_s0_lghtnbolt` are Neverwinter Nights scripts in KotOR's BIF. Their sources call `StartConversation` and `ResistSpell`, which KotOR's engine table does not have. `nw_s0_lghtnbolt.ncs` was compiled against NWN's table: its `ACTION` numbers do not fit KotOR's signatures, and the stack-depth analysis finds a conflict at offset 909.

## Building the oracle

meaglyn/nwn-tools at `ed2c227` needs two build changes for current tools, kept as a patch series in modshop (`games/kotor/patches/nwn-tools/`): CMake 4 refuses the declared minimum of 2.8.12, and the `string(REGEX MATCH …)` that looks for RedHat fails when `lsb_release` is absent because the unquoted empty variable disappears. Neither touches the compiler.

## nwnrs

nwnrs 0.0.1's compiler accepts KotOR's `nwscript.nss`, including the shipped line 5145, and compiles exactly the scripts nwnnsscomp compiles. Its output's program-size record differs from BioWare's: for `k_act_mispaz` at `--optimization O1`, the same length as BioWare's 85-byte file, the record holds 72.
