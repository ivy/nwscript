# Second pass: rakata used to trace Ajunta Pall's sword in KotOR 1 (2026-10-18)

rakata 0.0.0+2fce656, with a fresh index of the vanilla install (3.2 s, 438 MB). In the commands below `rakata` is the binary modshop's `mise run rakata:local` installs, and every command also takes `--db` naming that index, left out to keep them short. The first pass is [`2026-10-17-field-report-ajunta-pall.md`](2026-10-17-field-report-ajunta-pall.md).

## 1. Now right that the first pass found wrong

1. refs finds all 11 scripts that test the sword's tag, Uthar's turn-in among them (the first pass found 2): `rakata refs k37_itm_freedont`.
2. Calls keep each argument on its own: `[null,"k37_itm_freedont"]` for `k35_uth_hassword`: `rakata index query "SELECT DISTINCT l.name, c.args FROM calls c JOIN locations l ON l.hash=c.hash WHERE l.name IN ('k35_uth_hassword','k35_uth_giveswrd') AND c.routine='GetItemPossessedBy'"`.
3. refs adds weaker string-constant matches when a string reaches a call through a variable: `rakata refs kor37_shaardan` (`k_pkor_spn_shaar.ncs string constant ×1 @1151`).
4. `decompile` exists, with includes folded and every function verified: `rakata decompile k35_uth_hassword --module korr_m35aa`.
5. `flags OBJECT` exists. It resolves OBJECT_SELF through the dialogue's owner (`k37_statue.dlg` belongs to `k37_inv_statue`) and names indices by constant: `rakata flags k37_inv_statue --module korr_m37aa`.
6. `flags GLOBAL` exists, with values, sites and what runs each script: `rakata flags KOR_SWORD_PLACED`, `rakata flags KOR_SHAARDAN_SWORD`.
7. `quest` exists. It lists the entries in order, with writers, set globals, the remover and `--text`, and lints Shaardan's R12 as RW2103: `rakata quest kor37_ajuntapall --text`.
8. `show` resolves items through the 2DAs, shows a blueprint's scripts and inventory, and lists an area's instances and what scripts spawn: `rakata show k37_itm_ajunta.uti --module korr_m37aa`, `rakata show m37aa.git --module korr_m37aa`.
9. `dialog show --scripts` on `kor37_freedonnad.dlg` is down from 1,861 lines to 1,173. The per-function `// Recovered from…` lines are gone, and so is "no file under streamwaves/" (0 occurrences): `rakata dialog show kor37_freedonnad.dlg --module korr_m37aa --scripts | grep -c "no file under"` (0).
10. `k37_frd_rightswo`, `k37_frd_wrongswd` and `k_pkor_ajufirst` now fold to `UT_GetPlotBooleanFlag(oLocal1, SW_PLOT_BOOLEAN_01)` / `UT_GetTalkedToBooleanFlag(OBJECT_SELF)`: `rakata decompile k37_frd_rightswo --module korr_m37aa`.
11. `k37_statue_sword` verifies (it was E0602): `rakata decompile k37_statue_sword --module korr_m37aa` (`[verified, 9 of 9 functions]`).
12. The starts list prints each condition's source: `rakata dialog show kor37_freedonnad.dlg --module korr_m37aa | head -75`.
13. `dialog show --brief` exists (294 lines for the same dialogue): `rakata dialog show kor37_freedonnad.dlg --module korr_m37aa --brief`.
14. When an agent runs it, `index query` prints TSV, and its JSON is `{columns, rows}` with rows as arrays: `rakata --format json index query "SELECT name, type FROM locations LIMIT 3"`.
15. `cells.column` matching ignores case (`'Name'` and `'name'` both give 2811): `rakata index query "SELECT count(*) FROM cells WHERE column = 'Name'"`.
16. `--db` supplies the game root, so no command needed `--game`: any command above.

## 2. Still wrong or missing

1. **Folding uses the wrong include library.** `k35_trg_shaaswrd`, the Korriban academy trigger that moves Shaardan when he holds a fake sword, decompiles as `#include "k_inc_kas"` and `GetUllerBerriesLocal() == 0`. That function is Kashyyyk's, a byte-identical wrapper of `UT_GetPlotBooleanFlag(OBJECT_SELF, SW_PLOT_BOOLEAN_01)`. The output verifies but tells the reader something false about the script. When several libraries supply identical bytes, the fold should prefer the library the script's other functions came from, or the most basic one (`k_inc_utility`). Reproduce: `rakata decompile k35_trg_shaaswrd --module korr_m35aa`.
2. **Constant tables stay as `nGlobalN` when no include function folds.** This is the first pass's main readability complaint, still there in the scripts that matter:
   - `k37_ud_freedon` writes journal entry 20. It prints 146 `int nGlobalN` lines, and its guard reads `sub_ea5(oLocal1, nGlobal119)`, an unfolded `UT_GetPlotBooleanFlag(…, SW_PLOT_BOOLEAN_01)`.
   - `k_def_death01` is `ExecuteScript("k_ai_master", OBJECT_SELF, nGlobal7)` above 27 declarations, where `nGlobal7 = 1007`.
   - `k37_sha_goodswrd`, `k37_sha_fakeswrd` and `k37_sha_fakeswr2` print 28 lines each.
   - Uthar's E0 `--node` view spends 146 lines on `k_pkor_keldoubt1`, a one-line condition.

   Scripts that do fold still carry a stray `int nGlobal28 = 24;` (or `string sGlobal146 = "k35_itm_sithrwrd";`). The table itself identifies the include, so it could be folded even when no function is. Reproduce: `rakata decompile k37_ud_freedon --module korr_m37aa`, `rakata decompile k_def_death01`, `rakata dialog show kor35_utharwynn.dlg --module korr_m35aa --node E0`.
3. **refs and show ignore `resolved_calls`.** The index resolves Shaardan's spawn through `UT_CreateObject` to `CreateObject [1,"kor37_shaardan",…]`, path `[1175,2267]`. refs reports it only as a weaker string-constant match, and `show m37aa.git` says "a template the index does not know: computed, or passed in a variable" for `k_pkor_spn_shaar`. `flags` does use `resolved_calls`, so the three commands disagree. Reproduce: `rakata refs kor37_shaardan --direct`, `rakata show m37aa.git --module korr_m37aa | tail -6`, `rakata index query "SELECT r.routine, r.args, r.path FROM resolved_calls r JOIN locations l ON l.hash=r.hash WHERE l.name='k_pkor_spn_shaar' AND r.routine='CreateObject'"`.
4. **refs output is noisy and partly misleading.**
   - When a blueprint's resref equals its tag, the section "through the tag k37_itm_freedont of k37_itm_freedont.uti" repeats all 28 script hits already listed as direct.
   - `refs k37_itm_ajunta` lists the dialogues of the scripts it reached through the tag (`k35_uth_hassword`'s 18 Uthar replies) under "through the script" headings, before the "through the tag" header. They read as if those scripts named `k37_itm_ajunta`.
   - Each script's three party-member checks print as three lines, so "32 direct" is 11 scripts.

   Grouping hits by resource with their offsets, and dropping tag hits already shown, would roughly halve the output (91 lines here). Reproduce: `rakata refs k37_itm_freedont`, `rakata refs k37_itm_ajunta`.
5. **Commands name dialogue sites in two notations.** refs prints GFF paths (`kor35_utharwynn.dlg EntryList[0].RepliesList[6].Active`), which name the link index, not the reply. flags and quest print `E0->R33`. Without a second command you can't tell that `RepliesList[6]` is R33 ("I have the sword of Ajunta Pall."). Reproduce: `rakata refs k35_uth_hassword --direct` against `rakata flags KOR_FREEDON_SWORD`.
6. **quest shows only the conditions on the link into the writing node.**
   - Entry 60 (E33) prints no gate. The gate is `k35_uth_hassword` on `E*->R33`, two links before E33 (R33 → E32 runs `k35_uth_giveswrd` → R34 → E33).
   - Entry 5's E5 and E4 print nothing either.
   - Entry 20 names `k37_ud_freedon @3384` but neither its event (1007, on death) nor its guard (`!SW_PLOT_BOOLEAN_01` on `k37_inv_statue`).
   - "removed by: k_pebn_remove @1754 (m12aa.are OnEnter)" leaves out the module (`ebo_m41aa`), while every other line names its capsule.

   Reproduce: `rakata quest kor37_ajuntapall`.
7. **`dialog show --brief` drops quest writes.** It prints no `quest kor37_ajuntapall, entry N` lines (0 in brief, 2 in the default view), although they are short and are what a quest reader skims for. Reproduce: `rakata dialog show kor37_freedonnad.dlg --module korr_m37aa --brief | grep -c quest`.
8. **`k_pkor_freattack` takes 434 of the 1,173 lines of `--scripts`.** 72 functions fold, but 9 copies of the combat AI (the `GN_*` round logic) don't, for a script whose own body is 12 lines. Reproduce: `rakata dialog show kor37_freedonnad.dlg --module korr_m37aa --scripts | sed -n '88,522p'`.
9. Low: JSON is pretty-printed even in agent mode (`rakata --format json index query "SELECT name, type FROM locations LIMIT 3"`).
10. Low: RW1505 on a strref miss prints twice (the warning and the summary line). Its no-reader list includes formats that cannot hold a strref (tpc, tga, mdl, wok…); `ssf` is the only relevant gap. Reproduce: `rakata refs 6067 --kind strref`.
11. Low: a BIF member is written three ways: `data/templates.bif!#720.uti` (show), `data/templates.bif!g_w_dblswrd005.uti` (refs) and `data/scripts.bif[407]` (decompile). Reproduce: `rakata show g_w_dblswrd005.uti`, `rakata refs g_w_dblswrd005 --direct`, `rakata decompile k_def_death01 | head -1`.
12. Low: `dialog show --node E32 --scripts` decompiles the whole dialogue ("75 scripts decompiled") to show one node.
13. Low, carried over: `which` prints full sha256 hashes (`rakata which k37_statuesword.utp --module korr_m37aa`).

## 3. New

1. `decompile` takes one script per call, and this harness permits no shell loops, so the 15 decompiles needed for the quest took 15 invocations or one long `;` chain. Accepting several names (or `--all-from-dialog DLG`, or `quest TAG --scripts`) would remove that step.
2. Good: `flags`' "OBJECT_SELF unresolved" footer and `refs k37_statue_sword` (0 references) together show that `k37_statue_sword` is dead code. It is an earlier design of the statue (an inventory placeable that tests which sword was put in), and nothing in vanilla runs it.
3. Good: `show kor37_freedon.utc` revealed a third "Ajunta Pall's Blade", which the first pass missed: `g_w_dblswrd005` (templates.bif; Enhancement Bonus 3, Damage Bonus (Fire) cost row 4), in the spirit's own inventory, equipped by `k_pkor_freattack` when he turns hostile. `refs 6066 --kind strref` finds both items that carry that name.

## The story, as the tools now tell it

- **The items.** Three blueprints matter.
  - `k37_itm_freedont`, "Notched Steel Sword", tag `k37_itm_freedont`, no properties, not plot. It is the true sword among the three in the sarcophagus `kor37_sithsarc` (`show m37aa.git`). The decoys are `k37_itm_freednf1` ("Silver Lined Sword") and `k37_itm_freednf2` ("Vibrosword").
  - `k37_itm_ajunta`, "Ajunta Pall's Blade", tag **also** `k37_itm_freedont`, plot, Attack Bonus 5, Damage Bonus (Fire) cost row 2 and (Slashing) cost row 3. The spirit hands it over.
  - `g_w_dblswrd005`, the spirit's own blade (above).

  Every check later in the quest tests only the tag, so the notched sword and the blade are interchangeable to Uthar and to Shaardan.
- **The journal**, quest `kor37_ajuntapall` ("The Sword of Ajunta Pall", global.jrl Categories[94]):
  - **5.** The rumour, from the archaeologist (`kor35_sithfreed1.dlg` E5/E12; `kor36_sithfreed2.dlg` E4/E14). E12 and E14 also set `KOR_KNOW_FREEDON = 1`.
  - **10.** Written by the spirit's `kor37_freedonnad.dlg` R29, which runs `k37_frd_spoken`.
  - **20 (end).** The wrong sword: the spirit's on-death event 1007 in `k37_ud_freedon`, when `k37_inv_statue`'s SW_PLOT_BOOLEAN_01 is not set.
  - **30 (end).** The right sword: E47, gated by `k37_frd_rightswo`. Its next node E48 runs `k37_frd_giveswrd`, which creates `k37_itm_ajunta` on the PC.
  - **40 (end).** Gave Shaardan the real sword: `kor37_shaardan.dlg` R16, running `k37_sha_goodswrd`, sets `KOR_SHAARDAN_SWORD = 1`.
  - **50 (end).** Gave him the Silver Lined fake: R13, `k37_sha_fakeswrd`, sets `KOR_SHAARDAN_SWORD = 2`. R12, the Vibro Blade fake (`k37_sha_fakeswr2`), sets the same global to 2 but writes no entry (RW2103).
  - **60 (end).** Turned in to Uthar: `kor35_utharwynn.dlg` E33, which runs `k_pkor_1prestige` (`KOR_SITH_PRESTIGE + 1`). It is reached from R33 ("I have the sword of Ajunta Pall.", gated by `k35_uth_hassword`: the tag held by any of the party, and `KOR_FREEDON_SWORD == 0`) through E32, whose script `k35_uth_giveswrd` takes the item and sets `KOR_FREEDON_SWORD = 1`.
  - `k_pebn_remove` (`ebo_m41aa`, `m12aa.are` OnEnter) removes the quest.
- **The tomb (`korr_m37aa`).**
  1. Opening the sarcophagus runs `k37_opensarc`. It spawns `kor37_freedon` ("Spirit of Ajunta Pall"), locks `k37_dr_freedon`, and sets the sarcophagus's flag 01.
  2. Using the statue `k37_statue` runs `k37_plc_strtconv`. Once the spirit has spoken, that script makes `k37_inv_statue` start `k37_statue.dlg`.
  3. Each sword reply there (gated by `k37_sta_hastrue`, `k37_sta_hasfake1` or `k37_sta_hasfake2`) runs `k_pkor_swordplcd` (`KOR_SWORD_PLACED = 1`). It then runs `k37_sta_taketrue`, `k37_sta_takefak1` or `k37_sta_takefak2`, which set `k37_inv_statue`'s flag 01 to 1 or 0 and flag 02 to 1, destroy the sword, clear `k37_trg_ajunta`'s flag 01, and swap the statue for `k37_statuesword`.
  4. The spirit's dialogue then starts at E47 (right) or E64 (wrong). E64 runs `k_pkor_freattack`, which equips `G_W_DBLSWRD005` and turns him hostile.
  5. On the way out, `k37_trg_spn_shaa` (`k_pkor_spn_shaar`) spawns Shaardan if the party holds the tag, and `k37_trg_dest_tuk` (`k_pkor_dest_tuka`) clears the tukata. Both are gated by their trigger's flag 01.
- **The academy (`korr_m35aa`).** `KOR_SHAARDAN_SWORD` is read by `kor35_enter`, `k35_trg_shaaswrd`, `k35_trg_shaakill`, `k_pkor_ud_shaard` and `k35_uth_shaaswrd`: Shaardan presents the sword to Uthar (E137/E138), and `k35_uth_killshaa` (E142) sets the global to 3.
