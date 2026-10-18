# Working in the Rakata repository

For an agent changing Rakata itself rather than modding with it. The repository's `AGENTS.md` is the onboarding and governs; this is what crosses into the data workflow.

## The clean room

Rakata is written without reading the source of the tools it measures against. Do not open the source of PyKotor (any checkout or installed copy, including a project's virtual environment), HoloPatcher, kpatcher, TSLPatcher's reconstructed source, nwnnsscomp, Beamdog's compiler, xoreos-tools' code or nwnrs-nwscript, and do not query a code index against them. Run them only as black boxes and compare their output. Permitted: the user's own game files and mod archives, BioWare's `nwscript.nss`, the published format documentation (the xoreos documents, the KotOR and NWN wikis' format pages, stoffe's TSLPatcher readme and `changes.ini` syntax pages), `docs/formats/`, and the engine's observable behaviour. A hook refuses a tool call that names a reference project; the refusal is final, so find another way to the fact (measure the tool, read the bytes) instead of rephrasing the call.

## No game bytes

No game file, game text or extract of either is committed, ever: not as a fixture, not quoted in a document or a skill. Tests that need the game read the user's install from `$RAKATA_GAME_K1` (vanilla KotOR 1, the folder holding `chitin.key`), a modded build from `$RAKATA_BUILD`, saves from `$RAKATA_SAVES`, and skip with a message naming the variable when it is unset. Fixtures are made by hand or by the formats' own writers.

A test that writes builds a scratch install over the user's: a reflink copy, or a fresh directory whose `chitin.key`, BIFs and modules are links to the original, with its own empty `Override/`. Never write into the directory a variable names.

## Keeping this skill right

`crates/rakata-cli/tests/plugin.rs` checks that every `rakata` command and subcommand this plugin names exists and that every flag it uses appears in that command's `--help`, and that every name of the Python module the skill cites is in the module's stubs. A change to the command line or the module that breaks the skill fails `cargo test --workspace`.
