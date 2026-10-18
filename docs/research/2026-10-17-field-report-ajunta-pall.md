# Field report: rakata used to trace Ajunta Pall's sword in KotOR 1 (2026-10-17)

From an agent session in modshop with rakata 0.0.0+0119510, the index of the vanilla install, and the game at modshop's `games/kotor/data/game/vanilla/tree`. The task traced every reference to Ajunta Pall's sword (about 20 scripts, 6 dialogues, modules `korr_m35aa`, `korr_m36aa`, `korr_m37aa`).

## 1. refs drops literal arguments when a sibling argument is not a literal

`rakata refs k37_itm_freedont --db …` found 2 scripts; a byte scan of every resource in the three modules found 11. The 9 missing include Uthar's turn-in (`k35_uth_giveswrd`, `k35_uth_hassword`). `nws disasm` of `k35_uth_hassword.ncs` (from `korr_m35aa`) shows `CONSTS "k37_itm_freedont"`, `CPTOPSP -20, 4` (the object copied from a local), `ACTION 30, 2` (`GetItemPossessedBy`); `calls` records `GetItemPossessedBy [null,null]`: the extractor gives up on the whole call because one argument comes off the stack. Calls with `OBJECT_SELF` work (`k_pkor_freattack`'s `GetItemPossessedBy(OBJECT_SELF,"G_W_DBLSWRD005")` is found). Check: `rakata index query --db … "SELECT DISTINCT l.name, c.args FROM calls c JOIN locations l ON l.hash=c.hash WHERE l.name LIKE 'k35_uth_%swrd' OR l.name='k35_uth_hassword'"`. Suggested: record each argument separately (`[null,"k37_itm_freedont"]`), which alone recovers all 9; and a fallback table of every `CONSTS` string per script, so refs reports a weaker match ("string constant ×3, call not resolved") for tags that pass through variables.

## 2. dialog show and index query cost tokens they need not

- `dialog show --scripts` on `kor37_freedonnad.dlg` is 1,861 lines. Every decompiled condition reprints `k_inc_utility`'s globals (`int nGlobal1 = 0;` … `int nGlobal28 = 24;`; K1's includes declare `SW_PLOT_BOOLEAN_01` … and `SW_CONSTANT_*` as plain `int` globals; scripts using `k_inc_generic` carry 145, `k37_ud_freedon` for example).
- `// Recovered from NCS bytecode…` and `// verified: recompiles to the original bytes` on every function; show only what is not verified (as the E0602 on `k37_statue_sword`).
- 89 of 155 voice lines print "no file under streamwaves/": player replies (KotOR 1's protagonist is unvoiced) and the statue. Print nothing when there is no file, or skip the lookup for replies.
- Constants by name: `sub_39d(GetObjectByTag("k37_inv_statue"), nGlobal1)` means something only once `nGlobal1 = 0` is known; the useful form is `UT_GetPlotBooleanFlag(oStatue, SW_PLOT_BOOLEAN_01)`. The decompiler does this for some scripts (`k37_sta_taketrue` folds to `UT_SetPlotBooleanFlag(OBJECT_SELF, SW_PLOT_BOOLEAN_01, …)` with one stray `int nGlobal28 = 24;`) and not for `k37_frd_rightswo`, `k37_frd_wrongswd` and `k_pkor_ajufirst` (all in `kor37_freedonnad.dlg`); cause unknown.
- `index query` text tables pad every column to its widest value: one 300-character string widened all 40 rows of a `strings` query.
- For agents (detected as DataDog pup's `src/useragent.rs` does; `CLAUDECODE=1` is set in Claude Code): unpadded and compact output, TSV or `{columns, rows-as-arrays}` for `index query` rather than arrays of objects that repeat keys, and the existing text for transcripts.
- Hashes in text display could be shortened; low priority.

## 3. Smaller problems

- `cells` column names are lowercase, so `column = 'Name'` silently returns 0 rows: accept either case, or warn when the column does not exist.
- Setup: rakata was not on PATH (found through modshop's `mise.toml`); the game folder took an `ls`. The index records `trees.root`, so `--db` could supply `--game`, or refs hits could print full paths.
- In `dialog show`, the starts list names each condition without its source; the source appears only at its first use in the tree.

## 4. Suggested commands

- `rakata decompile NAME.ncs --module M --game DIR`: look the script up as `cat` does, decompile it as `dialog show` does with the same include folding (today: `rakata cat` to a file, then `nws decompile --game k1 -I …/corpus/nss` from inside modshop). Or let `nws decompile` take a game folder.
- `rakata flags OBJECT_TAG --module M` and `rakata flags GLOBAL`: who writes and who reads each local flag (with `OBJECT_SELF` resolved to the dialogue's owner or the event's object) or global, with values and sites.
- `rakata quest TAG`: the journal entries in order (from `global.jrl`), each with what writes it (dialogue `Quest`/`QuestEntry` fields, scripts' `AddJournalQuestEntry`), what removes the quest, and the globals involved; `--text` for the journal text; lint entries nothing writes and writes to entries the journal lacks (in vanilla, Shaardan's R12 writes no entry).
- `rakata show` for GFF files with names and 2DA values resolved (an item's properties through `itempropdef`, `iprp_*`, `baseitems`; a GIT's placeables, triggers and doors with their event scripts and inventories, and "spawned by script" from `CreateObject` in `calls`).
- `dialog show --brief` (text, conditions and script names only), `--node` (exists now), and constant tables folded away.

## What worked

`dialog show` with conditions decompiled, verified and includes folded made the quest readable and surfaced the designers' comments; the index schema was guessable (`strings` to `gff_fields` to `locations` went from a name to strref 6066 to the `.uti` files without docs); `refs`' indirect hits and one-line output are compact.
