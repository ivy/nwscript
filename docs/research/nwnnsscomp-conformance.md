---
title: nwnnsscomp against BioWare's compiler
date: 2026-10-16
status: measured
---

# nwnnsscomp against BioWare's compiler

The open-source NWScript compiler, built natively on Linux and fed KotOR's own `nwscript.nss`, reproduces BioWare's shipped bytecode for 91% of the vanilla scripts that have source. pykotor's built-in compiler, with the seven patches in modshop's `games/kotor/patches/README.md`, reproduces 15%. This document records how that was measured, what was needed to build the compiler, and what the remaining differences are, so the measurement can be repeated and extended.

## What the compilers are

KotOR shipped no script compiler. The bytecode in the game was produced by BioWare's in-house compiler, and the only record of its output is the 1,754 scripts in `data/scripts.bif` that ship as both NSS source and NCS bytecode (1,774 NSS, 1,784 NCS; module scripts in `modules/*.rim` ship as NCS only). That corpus is the ground truth for any compiler claiming compatibility; the engine's behaviour at runtime is the only other oracle.

`nwnnsscomp` is Edward T. Smith's (Torlack's) compiler for Neverwinter Nights, written in 2002 and 2003 and released under a BSD-style licence carried in each source file's header. Every KotOR build of it that mods ship (`nwnnsscomp.exe` in a `tslpatchdata/` folder, the KotOR Tool and KotOR Scripting Tool builds) is a Windows build of that code with a KotOR `nwscript.nss`. The source is on GitHub at [niv/nwn-tools](https://github.com/niv/nwn-tools), and a fork maintained for modern Linux toolchains at [meaglyn/nwn-tools](https://github.com/meaglyn/nwn-tools) (last commit 2022-09-13). The 2007 `nwnnsscomp-kotor-1.05` tarball on SourceForge's KOTOR ToolSet project, which its name suggests is a KotOR port, differs from the NWN tree only in a bison C++ parser and build files; its compiler library has no KotOR-specific change. The compiler was never modified for KotOR. It reads the game's function table from `nwscript.nss`, and that file is the whole of the game-specific input.

pykotor's compiler (`pykotor.resource.formats.ncs.compiler`) is an independent reimplementation in Python with a PLY-generated parser. Its known defects are in modshop's `plugins/kotor/skills/pykotor/references/scripts.md`.

## Building nwnnsscomp on Linux

From the meaglyn fork, on Fedora 44 with g++, bison 3.8.2 and cmake, three changes were needed. They are build fixes, not compiler changes:

1. `CMakeLists.txt`: remove `-m32` from `CMAKE_CXX_FLAGS`. The 32-bit build needs a multilib glibc that a modern distribution does not have, and the code builds and runs as 64-bit.
2. `CMakeLists.txt`: quote the argument of `string(REGEX MATCH "RedHat" is_redhat ${LSB_RELEASE_ID_SHORT})`. Without `lsb_release` installed the variable is empty and cmake aborts.
3. Build in-source (`cmake .` in the tree). The bison rule names `NscParser.ypp` relative to the current directory, so an out-of-source build cannot find the grammar.

The unmodified niv tree does not build with bison 3.8: its `NscCompat.h` defines `YYEMPTY` as a macro, and bison 3.8 emits `YYEMPTY` as an enumerator. The meaglyn fork handles bison 3.6 and later.

The command line on Linux is `nwnnsscomp -c -g -q -v1.00 -i INCDIR script.nss`; `-g` suppresses the `.ndb` debug file, `-i` names the directory holding `nwscript.nss` and the include files, and the output `.ncs` is written beside the input. `-v1.00` is explained below.

## The corpus and the trap in nwscript.nss

The corpus was extracted from the pristine Steam copy (app 32370, depot 32371, manifest 4122010938027300909) with pykotor: every NSS in the installation into one directory, and every NCS from `scripts.bif` whose name has a matching NSS into another, 1,754 pairs. `nwscript.nss` itself is in the NSS directory, which doubles as the compiler's include directory.

Vanilla `nwscript.nss` carries a BioWare typo at line 5145: `void SetAvailableNPCId(INT nNPC, OBJECT_ID oidNPC);`, with the type names in capitals. nwnnsscomp does not report an error on it; it loops forever after reading the file. The line must be corrected to `int` and `object` in the copy the compiler reads. The `nwscript.nss` files that ship with the Windows tools already carry the correction, which is why the hang is not documented anywhere.

## Method

For each pair: copy the NSS into an empty work directory, run the compiler there with the NSS directory as the include directory and a 60-second timeout, and compare the output bytes to the shipped NCS. Each script lands in one of four classes: identical; same length but different bytes; different length; failed (no output, or a non-zero exit). The harness is a nine-line shell script run through `xargs -P`; the whole corpus takes about a minute. pykotor was scored the same way in-process, with `compile_nss(src, Game.K1)` and `bytes_ncs`, which takes about eight minutes.

```bash
# one.sh COMPILER CORPUS RUNDIR NAME; CORPUS holds nss/ and ncs/
w="$3/work/$4"; mkdir -p "$w"; cp "$2/nss/$4.nss" "$w/"; cd "$w"
if timeout 60 "$1" -c -g -q -v1.00 -i "$2/nss" "$4.nss" </dev/null >log 2>&1 && [ -f "$4.ncs" ]; then
  if cmp -s "$4.ncs" "$2/ncs/$4.ncs"; then echo "identical $4"
  elif [ "$(stat -c %s "$4.ncs")" = "$(stat -c %s "$2/ncs/$4.ncs")" ]; then echo "samelen $4"
  else echo "difflen $4"; fi
else echo "failed $4"; fi
```

## Results

| Compiler | Identical | Same length, different | Different length | Failed |
| --- | --- | --- | --- | --- |
| nwnnsscomp, `-v1.00` | 1,598 | 1 | 132 | 23 |
| nwnnsscomp, default version | 1,204 | 66 | 461 | 23 |
| pykotor at the series head (patches 1 to 7) | 262 | 11 | 1,206 | 275 |

The version flag matters. Torlack's code generator has branches on `m_nVersion >= 130` (the NWN 1.30 compiler changed how empty global blocks and the tails of `if` statements are emitted). KotOR's bytecode was produced by a compiler older than that, and `-v1.00` selects the older forms. Of the 66 same-length differences at the default version, 65 vanish with `-v1.00`.

Five of the 23 failures are `k_inc_*` include files, which have no entry point and are not scripts; they fail in every compiler and are not defects. The real corpus is 1,749 scripts, of which nwnnsscomp reproduces 1,598, 91%.

## The remaining differences

The 18 real failures are stale declarations in BioWare's own `nwscript.nss`, not compiler defects. BioWare's compiler evidently compiled these scripts against a different `nwscript.nss` from the one that shipped:

- Ten `k_act_*rmv` and `k_act_bastadd` scripts: "Type mismatch in parameter 1 in call to RemovePartyMember" (and AddPartyMember). The shipped prototype's parameter type is not the one the scripts pass.
- Five scripts including `k_inc_ebonhawk`: "Function EBO_PlayTakeOff's prototype doesn't match the declaration", a forward declaration in the include disagreeing with its definition, which BioWare's compiler accepted.
- `k_pend_door04`: undeclared `TRASK_DOOR_JAMMED`; `nw_g0_conversat` and `nw_s0_lghtnbolt`: undeclared `StartConversation` and `ResistSpell`, NWN functions KotOR's table does not have. These are NWN leftovers in the BIF.

The 132 different-length outputs are 117 scripts with `#include` and 15 without. The size deltas cluster: 32 scripts are 6 bytes longer, 22 are 142 bytes longer, 20 are 77 bytes shorter, 14 are 3,798 bytes shorter (the `k_act_com*` family, which includes `k_inc_generic`), 10 are 42 bytes longer. Clusters this tight mean a handful of code-generation rules, not a diffuse mismatch; each cluster is one rule to find by disassembling a representative pair. Not yet done.

The one same-length difference at `-v1.00` is `k_ren_visionland`. Not yet inspected.

## What follows

- The build's compile step can use the native nwnnsscomp through pykotor's existing `ExternalNCSCompiler`, which already knows the compiler's argument forms. Until then every script HoloPatcher compiles on Linux goes through a compiler that matches BioWare 15% of the time.
- The harness above is a conformance suite in embryo: run against the corpus on every change to whatever compiler the build uses, with the per-script class committed as a manifest of names and hashes (the corpus itself stays in `data/`, per the repository's first rule).
- The three build fixes belong in a patch series under modshop's `games/kotor/patches/nwn-tools/` if the build adopts the compiler.
- The 132 differences and the `k_ren_visionland` case are the open questions about nwnnsscomp itself; nwnnsscomp's acceptance of the stale declarations BioWare's compiler accepted is a separate question about leniency.
- Whether a new compiler is worth writing is now a question about tooling (diagnostics, disassembly, decompilation, speed in an agent's loop), not about correctness; nwnnsscomp is the oracle any new compiler is measured against, with BioWare's corpus behind it.
