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

### `StartingConditional`'s entry stub reserves the result first

A script with `main` starts `JSR; RETN`; one with `StartingConditional` starts `RSADDI; JSR; RETN`, reserving the slot the conditional's result is returned in before calling into the globals' initialisation (or straight into the function when there are no globals). The globals therefore begin at the third instruction of a `main` script and at the fourth of a conditional. `nws disasm modules/korr_m37aa_s/k37_frd_wrongswd.ncs` shows the three-instruction stub, `ncs/k_con_light.ncs` the same for a vanilla conditional. Code that takes the globals to start at a fixed index reads a conditional's `RETN` as the first of them, matches no include's globals against any conditional, and leaves `k_inc_utility`'s constants unnamed (`sub_39d(oLocal1, nGlobal1)` for `UT_GetPlotBooleanFlag(oLocal1, SW_PLOT_BOOLEAN_01)`).

26 scripts, 21 of them in `modules/korr_m37aa_s` and the rest in `unk_m41aa_s`, `unk_m42aa_s`, `unk_m44aa_s` and `ebo_m46ab_s`, declare one more `int` global after `k_inc_utility`'s 27, initialised to 24, that no shipped include declares; `modules/korr_m37aa_s/k_pkor_actdroid1.ncs` passes it to `ActionPlayAnimation`. Its declaration stays in the recovered source, unnamed.

### Module bytecode compiled against other declarations

Fourteen module scripts call engine routines with arguments the shipped `nwscript.nss` does not declare for them, so their bytecode was compiled against an earlier table:

- `k_act_carthjoin`, in eleven Taris module capsules, and `k_ptar_missjoin` (`tar_m04aa_s`) call routine 574 with an object alone. The shipped declaration is `int AddPartyMember(int nNPC, object oCreature)`; the source-level failures of `k_act_*rmv` and `k_act_bastadd` in the conformance report are the same change seen from the other side.
- `k_plev_corpse1`, in two Leviathan capsules, calls routine 700 with an object and an int. The shipped routine 700 is `void ActionBarkString(int strRef)`, one argument.

The decompiler finds more of the same, as calls that pass fewer arguments than the shipped prototype declares even counting its defaults: a compiler working from the shipped table pushes the defaults, so BioWare's table had fewer parameters when these were compiled.

- `ShowPartySelectionGUI` with one argument (three declared), in 18 module functions.
- `ActionStartConversation` with five, in six module functions, as in `k_trg_senni`.
- `EndGame` with none (one declared), `StartNewModule` with four (eight), `PlayPazaak` with four (five).
- `StartCreditSequence` with two arguments where one is declared (`stunt_57_s/k_creditsplay`, `ACTION 518, 2` at offset 31).

The interpreter reports these as argument mismatches rather than machine faults (`FaultKind::ArgumentMismatch`).

### Division by a defaulted zero

Under an engine that answers every routine with its type's default, 158 module scripts divide by zero: `k_pdan_attack` divides `IntToFloat(GetCurrentHitPoints())` by `IntToFloat(GetMaxHitPoints())`, and the default maximum is 0. What the game's machine does on a float division by zero is not known; the interpreter stops with a fault marked as depending on the engine's answers.

## The shipped sources

### `nwscript.nss` line 5145

`void SetAvailableNPCId(INT nNPC, OBJECT_ID oidNPC);` has its types in capitals. nwnnsscomp loops forever on it instead of reporting an error, which is why the corpus carries an `oracle/` copy with the line corrected. The line still counts as a prototype: an engine routine's number in `ACTION` is its position among the prototypes from zero, and this one is number 767. No vanilla pair calls it; the module script `modules/danm13_s/k_pdan_13_area.ncs` does, with `ACTION 767, 2` at offsets 4789 and 4833, which are the slot's only reference bytes: two arguments, as the line declares.

### Engine routine numbers are prototype positions

`ACTION`'s routine operand is the index of the function's prototype in `nwscript.nss`, counting from zero and counting the malformed line above. `Random` is 0 and `PrintString` is 1. The grammar census checked this against the bytecode for the calls past position 600; the two misses are in `k_trg_transfail1`, whose shipped NCS is older than its shipped source.

### Scripts compiled against declarations that did not ship

Eighteen scripts fail in every compiler because the declarations they were compiled against are not the ones that shipped: ten call `AddPartyMember` or `RemovePartyMember` with an argument the shipped prototype rejects, five include `k_inc_ebonhawk`, whose forward declaration of `EBO_PlayTakeOff` disagrees with its definition, and three use names the shipped sources never declare. `conformance/known.toml` lists them by cause, and the conformance report shows the cause beside each failure.

What each needs to compile to BioWare's bytes:

- **The ten party-member scripts**: an earlier table. Their bytecode calls routines 569 and 570 with one object and discards a four-byte result, where the shipped table has `AddPartyMember(int, object)` at 574 and `RemovePartyMember(int)` at 575; the nine `k_act_*add` scripts and two `k_pman` scripts call the shipped forms, so the two eras are both in the vanilla set. `conformance/overlays/k1-party-object.nss` states the earlier one, and with it all ten are identical.
- **`nw_g0_conversat`**: `StartConversation()` is `ACTION 255, 0`, routine 255 with no arguments (in KotOR's table 255 is `BeginConversation`, whose defaults a compiler would push). `conformance/overlays/k1-nwn-conversation.nss`; identical.
- **Four of the five `k_inc_ebonhawk` scripts** (`k_act_makeitem`, `k_con_makeitem`, `k_pebn_galaxy`, `k_sup_galaxymap`): nothing. The include's forward declarations disagree with its definitions, but neither function is called; accepting that, as `--compat bioware` does, they are identical.
- **`k_sup_galaxmap`** and **`k_pend_door04`**: older includes, not declarations. `k_sup_galaxmap`'s `EBO_GetPlanetFrom2DA` and `k_inc_stunt` functions are shorter in BioWare's bytecode. `k_pend_door04`'s `TRASK_DOOR_JAMMED` is a global of an older `k_inc_end`, 16, where the shipped file has `TRASK_COMBAT_WARNING = 16` and comments out `TRASK_COMBAT_WARNING = 27`; that older file also lacked a block of negative-valued globals, so restoring the two lines is not enough.
- **`nw_s0_lghtnbolt`**: Neverwinter Nights code compiled against NWN's table and constants, most of them in functions it never calls. The bytecode holds no trace of those values, so there is nothing to infer them from.

The overlays are inferences: each file says what in the bytecode it rests on, and `crates/nwscript/tests/overlays_corpus.rs` compiles the scripts `known.toml` lists for it and requires BioWare's bytes.

### `ActionStartConversation` had eleven parameters

The shipped `nwscript.nss` declares `ActionStartConversation` with twelve parameters, the last an int defaulting to 0. BioWare's bytecode passes eleven: every call compiled by BioWare leaves the twelfth out, so those scripts were compiled when the routine had eleven. A compiler working from the shipped declaration pushes one more `CONSTI 0`, six bytes, which is the whole of the largest cluster of nwnnsscomp's different-length outputs (32 scripts, +6 bytes each). The conformance report's trace comparison shows it directly, in `k_pend_door01` for one:

```text
BioWare's   ActionStartConversation(OBJECT_INVALID, "end_trask01", 0, 0, 1, "", "", "", "", "", "")
nwnnsscomp  ActionStartConversation(OBJECT_INVALID, "end_trask01", 0, 0, 1, "", "", "", "", "", "", 0)
```

Of the 32, 17 differ in their trace; in the other 15 the call sits on a path the recording engine does not reach. Reproducing BioWare's bytes takes the eleven-parameter declaration; the engine evidently accepts both.

### Bytecode older than its source

Some shipped NCS was compiled from earlier versions of the sources that ship beside it, so no compiler reproduces it from them. Code generation locates the first difference in each (`crates/nwscript-codegen/tests/corpus.rs` re-derives every one), and `conformance/known.toml` names them:

- **An include changed** (`stale-include`, 73 scripts). Most are globals of `k_inc_utility` missing from BioWare's bytecode: the six `SW_CONSTANT_*_HIT_*` globals (lines 37 to 42) are absent from 60 scripts, and the same names are declared again as locals inside a function at lines 818 to 823, hiding them. The rest are `k_inc_gensupport` globals, `k_inc_generic`'s `GN_TalentMasterRoutine` (the +334-byte group), `k_inc_force`'s `Sp_RunForcePowers` (`k_sp1_generic`), and `k_inc_utility`'s `UT_StarMap1VariableSet`, whose `"v3.0 - K_STAR_MAP Before = "` is the string that differs in `k_pla_actmap`.
- **The script changed** (`stale-bytecode`, 8 scripts): `k_ptat_exitkrayt`, `k_qa`, `k_sup_droid`, `k_sup_gohawk`, `k_sup_grenade`, `k_trg_senni`, `k_trg_transfail1`, `k_trp_generic`. `k_qa.ncs` has a subroutine that calls `AddAvailableNPCByTemplate` once per NPC and has no counterpart in `k_qa.nss`.

### Three eras of declarations

The engine table changed while the game was being written, and the scripts carry the era each was compiled in. `ActionStartConversation` is called with eleven arguments in 51 scripts, twelve (the shipped declaration) in three (`k_ai_master`, `k_pman_27_init01`, `k_trg_transfail`), and five in `k_trg_senni` (`ACTION 204, 5`). `AddPartyMember` and `RemovePartyMember` are routines 569 and 570 in the BIF scripts that call them with an object alone (`k_act_bastadd`, `k_act_bastrmv`), routine 574 with an object alone in the module scripts `k_act_carthjoin` and `k_ptar_missjoin`, and 574 and 575 with two arguments in the shipped table. `nws compile --declarations bioware` loads an overlay of the eleven-argument `ActionStartConversation` (`crates/nwscript/src/overlays/k1-bioware.nss`), which reproduces the 51. The party-member routines' earlier era is `conformance/overlays/k1-party-object.nss`, applied with `--overlay`; it cannot join the built-in overlay, because the scripts that call the shipped forms would then break.

### Where the sources disagree with themselves

- A local hides a parameter: `GN_GetTargetedForcePower(int nDroid = FALSE)` in `k_inc_gensupport.nss` (lines 2544 and 2547) declares a local `nDroid`, so every test of `nDroid` reads the local, never the argument. Lint W0004 reports it.
- `k_inc_force.nss` declares `Sp_GetRandomDelay`'s defaults as `(0.0, 0.75)` at line 34 and defines them as `(0.4, 1.1)` at line 266. No call omits them, so there are no reference bytes for which BioWare's compiler used; nwnnsscomp uses the prototype's, and so does `nws`.
- `k_inc_ebonhawk.nss` repeats the prototype of `EBO_BastilaStartConversation2` (lines 11 and 15) and declares `EBO_PlayTakeOff` and `EBO_PlayLanding` as `void` (lines 19, 21) but defines them as `string` (lines 209, 243). Neither is called.
- `k_inc_generic.nss` declares `struct tLastRound tPR;` at line 27, a ten-field struct global (lines 14 to 25). Module bytecode reads its fields slot by slot, so `nws decompile` recovers it as ten globals, one per field; `rakata corpus decompile` folds them back as `tPR.FIELD` when the script includes the library. A grep for `^int SW_COMBO_` in `k_inc_gensupport.nss` finds the 29 combo constants twice, at lines 29 to 57 and lines 486 to 514; the second set sits inside the block comment that opens at line 482, so only the first is declared.

### Encodings the corpus confirms

- A negative number in an expression is the literal and a negation, `CONSTI 1; NEGI`, but a `case -1` label or a parameter default is folded to `CONSTI -1` or `CONSTF -1.5`.
- An `if` without an `else` still ends its branch with a `JMP` to the very next instruction.
- A `return` with a value copies the value down to the return slot, pops everything above the parameters with one `MOVSP`, and jumps to the function's epilogue (`MOVSP` of the parameters, `RETN`); the `MOVSP` of the enclosing block's locals that follows is never reached.
- A vector literal is three `CONSTF`, x first: `modules/tat_m17mg_s/onfire.ncs` passes `[20.0, 0.0, 0.0]` to `SWMG_SetPlayerTunnelPos` as `CONSTF 20.0; CONSTF 0.0; CONSTF 0.0` (offsets 574 to 586).
- A component of a vector is read by pushing the whole vector and keeping one float with `DESTRUCT 12, offset, 4`, whether the vector is a variable or a call's result: `modules/m12ab_s/k_heartbeat.ncs` reads `SWMG_GetPlayerOffset().z` as `ACTION 641, 0; DESTRUCT 12, 8, 4`.
- `0xffffffff` compiles to `CONSTI -1`, so the decompiler writes a negative integer that is not a folded default in hex: written `-1`, it would recompile to `CONSTI 1; NEGI`.
- `break` and `continue` leave the locals of the blocks they leave on the stack, which is what unbalances the stack after a `case` that declared one.
- Functions are emitted in the order the calls to them are emitted, depth first, starting from the globals routine's callees and then the entry point; arguments are pushed last first, so a callee in the last argument comes first.
- `STORE_STATE` saves the globals' size and the current frame: return slot, parameters and locals.

### Neverwinter Nights leftovers

`nw_g0_conversat` and `nw_s0_lghtnbolt` are Neverwinter Nights scripts in KotOR's BIF. Their sources call `StartConversation` and `ResistSpell`, which KotOR's engine table does not have. `nw_s0_lghtnbolt.ncs` was compiled against NWN's table: its `ACTION` numbers do not fit KotOR's signatures, and the stack-depth analysis finds a conflict at offset 909.

## Building the oracle

meaglyn/nwn-tools at `ed2c227` needs two build changes for current tools, kept as a patch series in modshop (`games/kotor/patches/nwn-tools/`): CMake 4 refuses the declared minimum of 2.8.12, and the `string(REGEX MATCH …)` that looks for RedHat fails when `lsb_release` is absent because the unquoted empty variable disappears. Neither touches the compiler.

## nwnrs

nwnrs 0.0.1's compiler accepts KotOR's `nwscript.nss`, including the shipped line 5145, and compiles exactly the scripts nwnnsscomp compiles. Its output's program-size record differs from BioWare's: for `k_act_mispaz` at `--optimization O1`, the same length as BioWare's 85-byte file, the record holds 72.
