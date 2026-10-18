---
title: "rakata-dialog: the DLG model with stable keys, and `rakata dialog show`"
kind: task
priority: p1
parent: m2-know
blocked_by: [rakata-gff, rakata-tlk, rakata-game]
assignee_kind: agent
labels: [rakata, dialog, cli]
---

## Context

The proposal governs this work: `docs/proposals/rakata.md` in `~/src/github.com/ivy/nwscript`, section "Dialogues". A DLG is a GFF: `EntryList` and `ReplyList` of nodes, `StartingList` of links into entries, each link with `Index`, `Active` (the condition script) and `IsChild`; each node with `Speaker`, `Listener`, `Text` (a localized string), `VO_ResRef`, `Sound`, `Script`, `Delay`, `AnimList` (participant and animation number), `CameraAngle`, `CameraID`, `Quest`, `QuestEntry`, `Comment`; root fields such as `DelayEntry`, `Skippable`, `ConversationType`, `CameraModel`. The format pages are on the KotOR wiki. modshop's Juhani mod wrote its own model (`games/kotor/mods/juhani-clones/dialog.py`: `add_entry`, `add_reply`, `link`, `start`, append-only) and the precedent it needed most was found by parsing every dialogue: in `banter.dlg`, a node's script plays the prone animation and the next node's `AnimList` gets Mission up with animation 10055 (`games/kotor/docs/ideas/asset-index.md`). Conditions are scripts; `nws decompile` recovers them.

## Deliverables

- `crates/rakata-dialog/`: `Dialog` over `rakata_gff::Gff`, lossless (fields the model does not name are kept so write-back is identical); `NodeKey::{Voice(ResRef), StrRef(u32), Text(String), Index(Kind, usize)}` with `resolve(&Dialog) -> Result<NodeRef>` that fails with a diagnostic naming the key when there is no such node or more than one (`Index` resolves but is marked fragile in output).
- The transcript: starts in order, then the tree with indentation, each node as speaker to listener, the text resolved through the TLK, voice resref with the WAV's duration when `streamwaves/<vo>.wav` exists (header only), scripts, each condition's source decompiled through `nwscript-decomp` and marked verified or not, animations as number and participant, camera angle and id, delay, quest; a link to a node already shown prints a reference instead of recursing.
- `rakata dialog show NAME.dlg|FILE --game DIR [--module M] [--format json]`.
- Registration with the conformance registry under `.dlg` is `rakata-gff`'s; a test here writes every vanilla DLG through the model and compares bytes.

## Acceptance criteria

- [ ] `rakata dialog show banter.dlg --game $RAKATA_BUILD` shows a node whose script plays the prone animation followed by a node with animation 10055 on Mission, with the conditions' decompiled source inline.
- [ ] Every vanilla DLG renders without error; the wall time for all of them is in the result (target under a second each with conditions decompiled, conditions cached by hash).
- [ ] Every vanilla DLG written back through the model is byte-identical.
- [ ] `NodeKey::Voice("nm14acjuha07024_")` resolves to exactly one node of the loaded `dan14_cutjuh.dlg` in `$RAKATA_BUILD`; a `Text` key matching two nodes of a fixture fails naming both indices.
- [ ] `--format json` emits the transcript as structured nodes; `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo test` pass.

## Scenarios

```gherkin
Scenario: A key that matches nothing
  Given a dialogue without a node voiced by oaj_missing
  When NodeKey::Voice("oaj_missing") is resolved
  Then the result is a diagnostic with a code, naming the key and the dialogue
```

## Notes

- An empty `Speaker` means the dialogue's owner; print it as such, not as blank.
- `Text` keys compare after TLK resolution and whitespace trimming.
- Engine-only numbers (animations, camera angles) print as numbers with no name until an engine table exists; do not invent names.
- **Clean room** (`docs/proposals/rakata.md`, Clean room). **No game bytes in the repository.**
