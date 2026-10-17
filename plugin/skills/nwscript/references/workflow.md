# From a change to an installed script

The loop is check, test, compile, install. Each step answers in milliseconds; the game is for what the interpreter does not model.

## 1. Find the script

- A script the game runs has a resref: a dialog node's condition or action, a blueprint's event slot (OnEnter, OnUsed, OnHeartbeat), a trigger's script field. If the user names behaviour rather than a script, find the resref first (with PyKotor, KotOR Tool or the user's knowledge), then its file.
- KotOR 1's global scripts ship with source in `scripts.bif`; module scripts ship as bytecode only, in each module's `*_s.rim`. A checkout of nwscript extracts both from a game install: `cargo xtask corpus extract --game-dir PATH --out DIR` (source in `DIR/nss`, bytecode in `DIR/ncs` and `DIR/modules/NAME/`).
- With only bytecode, recover source: `nws --game k1 --declarations bioware -I DIR decompile FILE.ncs -o FILE.nss`. Work from it only where the comment above a function says `verified`.

## 2. Change and check

Edit the `.nss`, then:

```bash
nws --game k1 -I DIR check FILE.nss
```

Fix every error before going on; `diagnostics.md` covers reading them. `nws fn NAME` and `nws fn --grep WORD` find an engine routine and its parameters; the comment above it in `nwscript.nss` is the game's own documentation.

## 3. Test

Before compiling for real, prove the change: `testing.md`. At the least, compare the old and new script's engine calls:

```bash
nws --game k1 -I DIR diff OLD.ncs NEW.nss
```

`diff` compiles a `.nss` input first and compares bytes, instructions and the trace of engine calls, reporting the first difference at each level. Read it with care:

- `trace: same` does not mean the behaviour is the same. The recording engine answers with defaults, so a change that only matters for other replies (a credit threshold, when gold reads as 0) leaves the trace unchanged. A test that supplies those replies (`testing.md`) is the evidence for a behaviour change.
- Only the first difference per level is shown, at a 0-based offset. To show that nothing else changed, compare the disassemblies: `nws disasm OLD.ncs` and `nws disasm NEW.ncs`, then an ordinary `diff` of the two.

## 4. Compile

```bash
nws --game k1 -I DIR compile FILE.nss -o FILE.ncs
```

`--emit ncs,asm` also writes the assembly, for reading what was produced.

## 5. Install

`nws` has no install command. A changed script usually ships as a loose `.ncs` with the same resref in the game's `Override` folder, which the game loads ahead of its own copies: all of them, in every module.

**Check for a name clash first.** Different scripts can share a resref: `k_pman_misc59` in `scripts.bif` is a credits check, and the `k_pman_misc59` in two Manaan modules is another script entirely. Find every copy (the extracted corpus's `ncs/` and each `modules/NAME/`) and compare them (`nws diff`); if any copy is a different script, a loose file in `Override` replaces it too and breaks it. Install such a script into the module that runs it instead, with an installer such as TSLPatcher or HoloPatcher, or give the new script a new resref and point the caller at it.

Ask before writing into a game folder, and keep the original so the change can be undone.
