# Conformance

1754 scripts with both source and bytecode in `conformance/k1.toml`.

| Compiler | Identical | Same length, different | Different length | Failed |
| --- | --- | --- | --- | --- |
| nws | 1603 | 10 | 133 | 8 |
| nws-bioware | 1652 | 10 | 84 | 8 |
| nwnnsscomp-v100 | 1598 | 1 | 132 | 23 |
| nwnnsscomp | 1204 | 66 | 461 | 23 |
| pykotor | 262 | 11 | 1206 | 275 |
| nwnrs-o0 | 0 | 252 | 1479 | 23 |
| nwnrs-o1 | 0 | 375 | 1356 | 23 |
| nwnrs-o2 | 0 | 375 | 1356 | 23 |
| nwnrs-o3 | 0 | 375 | 1356 | 23 |

## nws

nws compile, this repository's compiler, --game k1 --compat bioware (BioWare's scripts need what its compiler accepted)

Of the 143 outputs that differ from BioWare's bytes, 82 make the same engine calls with the same arguments and return the same result under the recording engine, 61 behave differently, and 0 do not run.

Failed (8):

- **undeclared** (3): uses a name the shipped sources do not declare: a constant BioWare's own build had (TRASK_DOOR_JAMMED), or a Neverwinter Nights function KotOR's table lacks (StartConversation, ResistSpell)
  - `k_pend_door04`: k_pend_door04.nss: error[E0303]: no variable or constant named `TRASK_DOOR_JAMMED`; nothing written
  - `nw_g0_conversat`: nw_g0_conversat.nss: error[E0304]: no function named `StartConversation`; nothing written
  - `nw_s0_lghtnbolt`: nw_s0_lghtnbolt.nss: error[E0304]: no function named `ResistSpell`; nothing written
- **not-a-script** (5): an include file with no entry point; there is nothing to compile
  - `k_inc_generic`: k_inc_generic.nss: error[E0401]: a script without `void main()` or `int StartingConditional()` has nothing to compile; nothing written
  - `k_inc_gensupport`: k_inc_gensupport.nss: error[E0401]: a script without `void main()` or `int StartingConditional()` has nothing to compile; nothing written
  - `k_inc_switch`: k_inc_switch.nss: error[E0401]: a script without `void main()` or `int StartingConditional()` has nothing to compile; nothing written
  - `k_inc_unk`: k_inc_unk.nss: error[E0401]: a script without `void main()` or `int StartingConditional()` has nothing to compile; nothing written
  - `k_inc_walkways`: k_inc_walkways.nss: error[E0401]: a script without `void main()` or `int StartingConditional()` has nothing to compile; nothing written

Different output (143), by cause:

- **stale-prototype** (11): BioWare compiled this against declarations that were not shipped: the shipped nwscript.nss prototype of AddPartyMember/RemovePartyMember, or k_inc_ebonhawk's forward declaration of EBO_PlayTakeOff, disagrees with the call or definition
  `k_act_bastadd`, `k_act_bastrmv`, `k_act_canderrmv`, `k_act_carthrmv`, `k_act_hki47rmv`, `k_act_joleermv`, `k_act_juhanirmv`, `k_act_missionrmv`, `k_act_t3m3rmv`, `k_act_zaalrmv`, `k_sup_galaxmap`
- **stale-include** (73): BioWare's bytecode was compiled before an include it uses last changed: the first difference is in an include's globals or functions (k_inc_utility's SW_CONSTANT_* globals, k_inc_generic's GN_TalentMasterRoutine, among others)
  `k_act_atkonend`, `k_act_caloatt`, `k_act_com31`, `k_act_com32`, `k_act_com33`, `k_act_com33a`, `k_act_com34`, `k_act_com35`, `k_act_com41`, `k_act_com42`, `k_act_com43`, `k_act_com45`, `k_act_comp01`, `k_act_electric`, `k_act_startgas`, `k_act_xoratt`, `k_actq_com33`, `k_actq_com33a`, `k_actq_com34`, `k_amb_enemyflee`, `k_amb_predator01`, `k_com_compart`, `k_com_comrepr`, `k_con_com31`, `k_con_com32`, `k_con_com33`, `k_con_com34`, `k_con_com35`, `k_con_com41`, `k_con_com42`, `k_con_com43`, `k_con_com44`, `k_con_com45`, `k_con_comactive`, `k_con_comhack`, `k_con_comrep`, `k_con_comver31`, `k_con_comver32`, `k_con_comver33`, `k_con_comver33a`, `k_con_comver34`, `k_con_comver41`, `k_con_comver42`, `k_con_comver43`, `k_con_comver44`, `k_con_comver45`, `k_con_tokens`, `k_def_rependd`, `k_pla_actmap`, `k_plc_tresciv`, `k_plc_trescorhig`, `k_plc_trescorlow`, `k_plc_trescormid`, `k_plc_tresdrdhig`, `k_plc_tresdrdlow`, `k_plc_tresdrdmid`, `k_plc_tresmilhig`, `k_plc_tresmillow`, `k_plc_tresmilmid`, `k_plc_tresrakat`, `k_plc_tresshahig`, `k_plc_tresshalow`, `k_plc_tresshamid`, `k_plc_tressndppl`, `k_ptat17aa_enter`, `k_ptat_gammenemy`, `k_sp1_generic`, `k_sup_star_map`, `k_trg_messengers`, `k_trg_partyheart`, `k_trg_stealth`, `k_zon_catalog`, `k_zon_control`
- **stale-bytecode** (8): BioWare's bytecode was compiled from an earlier version of this script's own source: the first difference is in the script, and it is a different call, constant or condition
  `k_ptat_exitkrayt`, `k_qa`, `k_sup_droid`, `k_sup_gohawk`, `k_sup_grenade`, `k_trg_senni`, `k_trg_transfail1`, `k_trp_generic`
- **conversation-eleven** (51): BioWare compiled this when ActionStartConversation took eleven arguments; the shipped declaration has twelve. Identical with --declarations bioware
  `k_bant_trig`, `k_def_bandspn`, `k_def_calodef`, `k_hjol_userdef01`, `k_pdan_elise_d`, `k_pdan_rapid01`, `k_pdan_rapid02`, `k_pdan_rapid03`, `k_pdan_rapid04`, `k_pdan_rapid05`, `k_pend_door01`, `k_pend_door02`, `k_pend_door03`, `k_pend_door08`, `k_pkas_freyyuser`, `k_pman_26a_area`, `k_pman_28ac_init`, `k_pman_at01`, `k_pman_at02`, `k_pman_door03`, `k_pman_door06`, `k_pman_door07`, `k_pman_door08`, `k_pman_door10`, `k_pman_door11`, `k_pman_door13`, `k_pman_door14`, `k_pman_door20`, `k_pman_door30`, `k_pman_init01`, `k_pman_init02`, `k_pman_init03`, `k_pman_init10`, `k_pman_judge1_d`, `k_pman_obj_asc`, `k_pman_obj_asc2`, `k_pman_obj_ascc`, `k_pman_obj_ascc2`, `k_pman_obj_ascn`, `k_pman_osc`, `k_pman_rapid01`, `k_pman_rapid02`, `k_pman_rapid03`, `k_pman_rapid04`, `k_pman_rapid05`, `k_pman_sitspeak`, `k_pman_survive01`, `k_ptar_sitharm`, `k_ptat_krayttran`, `k_swg_xor_user`, `k_trg_partyinit`

Size differences, largest clusters first:

| Bytes | Scripts | Examples |
| --- | --- | --- |
| +6 | 32 | `k_def_bandspn`, `k_hjol_userdef01`, `k_pdan_elise_d` |
| +142 | 22 | `k_act_com41`, `k_act_comp01`, `k_act_electric` |
| -77 | 20 | `k_act_com32`, `k_act_com33`, `k_act_com33a` |
| -3798 | 14 | `k_plc_tresciv`, `k_plc_trescorhig`, `k_plc_trescorlow` |
| +0 | 10 | `k_act_bastadd`, `k_act_bastrmv`, `k_act_canderrmv` |
| +42 | 10 | `k_pdan_rapid01`, `k_pdan_rapid02`, `k_pdan_rapid03` |
| +12 | 8 | `k_def_calodef`, `k_pend_door01`, `k_pman_26a_area` |
| +334 | 5 | `k_act_atkonend`, `k_act_caloatt`, `k_act_xoratt` |
| -58 | 2 | `k_act_com31`, `k_con_com31` |
| -27 | 2 | `k_sup_gohawk`, `k_trp_generic` |
| +44 | 2 | `k_zon_catalog`, `k_zon_control` |
| -203 | 1 | `k_sp1_generic` |
| -87 | 1 | `k_ptat_exitkrayt` |
| -4 | 1 | `k_sup_droid` |
| +7 | 1 | `k_pla_actmap` |
| +18 | 1 | `k_sup_grenade` |
| +30 | 1 | `k_trg_senni` |
| +66 | 1 | `k_bant_trig` |
| +70 | 1 | `k_trg_transfail1` |
| +77 | 1 | `k_act_com45` |
| +161 | 1 | `k_con_comhack` |
| +274 | 1 | `k_trg_partyheart` |
| +341 | 1 | `k_trg_messengers` |
| +353 | 1 | `k_trg_stealth` |
| +785 | 1 | `k_qa` |
| +1143 | 1 | `k_sup_galaxmap` |
| +2177 | 1 | `k_ptat17aa_enter` |

## nws-bioware

nws compile --declarations bioware --compat bioware: the routines BioWare's scripts were compiled against

Of the 94 outputs that differ from BioWare's bytes, 52 make the same engine calls with the same arguments and return the same result under the recording engine, 42 behave differently, and 0 do not run.

Failed (8):

- **undeclared** (3): uses a name the shipped sources do not declare: a constant BioWare's own build had (TRASK_DOOR_JAMMED), or a Neverwinter Nights function KotOR's table lacks (StartConversation, ResistSpell)
  - `k_pend_door04`: k_pend_door04.nss: error[E0303]: no variable or constant named `TRASK_DOOR_JAMMED`; nothing written
  - `nw_g0_conversat`: nw_g0_conversat.nss: error[E0304]: no function named `StartConversation`; nothing written
  - `nw_s0_lghtnbolt`: nw_s0_lghtnbolt.nss: error[E0304]: no function named `ResistSpell`; nothing written
- **not-a-script** (5): an include file with no entry point; there is nothing to compile
  - `k_inc_generic`: k_inc_generic.nss: error[E0401]: a script without `void main()` or `int StartingConditional()` has nothing to compile; nothing written
  - `k_inc_gensupport`: k_inc_gensupport.nss: error[E0401]: a script without `void main()` or `int StartingConditional()` has nothing to compile; nothing written
  - `k_inc_switch`: k_inc_switch.nss: error[E0401]: a script without `void main()` or `int StartingConditional()` has nothing to compile; nothing written
  - `k_inc_unk`: k_inc_unk.nss: error[E0401]: a script without `void main()` or `int StartingConditional()` has nothing to compile; nothing written
  - `k_inc_walkways`: k_inc_walkways.nss: error[E0401]: a script without `void main()` or `int StartingConditional()` has nothing to compile; nothing written

Different output (94), by cause:

- **stale-prototype** (11): BioWare compiled this against declarations that were not shipped: the shipped nwscript.nss prototype of AddPartyMember/RemovePartyMember, or k_inc_ebonhawk's forward declaration of EBO_PlayTakeOff, disagrees with the call or definition
  `k_act_bastadd`, `k_act_bastrmv`, `k_act_canderrmv`, `k_act_carthrmv`, `k_act_hki47rmv`, `k_act_joleermv`, `k_act_juhanirmv`, `k_act_missionrmv`, `k_act_t3m3rmv`, `k_act_zaalrmv`, `k_sup_galaxmap`
- **stale-include** (73): BioWare's bytecode was compiled before an include it uses last changed: the first difference is in an include's globals or functions (k_inc_utility's SW_CONSTANT_* globals, k_inc_generic's GN_TalentMasterRoutine, among others)
  `k_act_atkonend`, `k_act_caloatt`, `k_act_com31`, `k_act_com32`, `k_act_com33`, `k_act_com33a`, `k_act_com34`, `k_act_com35`, `k_act_com41`, `k_act_com42`, `k_act_com43`, `k_act_com45`, `k_act_comp01`, `k_act_electric`, `k_act_startgas`, `k_act_xoratt`, `k_actq_com33`, `k_actq_com33a`, `k_actq_com34`, `k_amb_enemyflee`, `k_amb_predator01`, `k_com_compart`, `k_com_comrepr`, `k_con_com31`, `k_con_com32`, `k_con_com33`, `k_con_com34`, `k_con_com35`, `k_con_com41`, `k_con_com42`, `k_con_com43`, `k_con_com44`, `k_con_com45`, `k_con_comactive`, `k_con_comhack`, `k_con_comrep`, `k_con_comver31`, `k_con_comver32`, `k_con_comver33`, `k_con_comver33a`, `k_con_comver34`, `k_con_comver41`, `k_con_comver42`, `k_con_comver43`, `k_con_comver44`, `k_con_comver45`, `k_con_tokens`, `k_def_rependd`, `k_pla_actmap`, `k_plc_tresciv`, `k_plc_trescorhig`, `k_plc_trescorlow`, `k_plc_trescormid`, `k_plc_tresdrdhig`, `k_plc_tresdrdlow`, `k_plc_tresdrdmid`, `k_plc_tresmilhig`, `k_plc_tresmillow`, `k_plc_tresmilmid`, `k_plc_tresrakat`, `k_plc_tresshahig`, `k_plc_tresshalow`, `k_plc_tresshamid`, `k_plc_tressndppl`, `k_ptat17aa_enter`, `k_ptat_gammenemy`, `k_sp1_generic`, `k_sup_star_map`, `k_trg_messengers`, `k_trg_partyheart`, `k_trg_stealth`, `k_zon_catalog`, `k_zon_control`
- **stale-bytecode** (8): BioWare's bytecode was compiled from an earlier version of this script's own source: the first difference is in the script, and it is a different call, constant or condition
  `k_ptat_exitkrayt`, `k_qa`, `k_sup_droid`, `k_sup_gohawk`, `k_sup_grenade`, `k_trg_senni`, `k_trg_transfail1`, `k_trp_generic`
- **conversation-twelve** (2): BioWare compiled this against the shipped twelve-argument ActionStartConversation, unlike most of its scripts. Identical with the shipped declarations
  `k_ai_master`, `k_pman_27_init01`

Size differences, largest clusters first:

| Bytes | Scripts | Examples |
| --- | --- | --- |
| +142 | 22 | `k_act_com41`, `k_act_comp01`, `k_act_electric` |
| -77 | 20 | `k_act_com32`, `k_act_com33`, `k_act_com33a` |
| -3798 | 14 | `k_plc_tresciv`, `k_plc_trescorhig`, `k_plc_trescorlow` |
| +0 | 10 | `k_act_bastadd`, `k_act_bastrmv`, `k_act_canderrmv` |
| +334 | 5 | `k_act_atkonend`, `k_act_caloatt`, `k_act_xoratt` |
| -58 | 2 | `k_act_com31`, `k_con_com31` |
| -27 | 2 | `k_sup_gohawk`, `k_trp_generic` |
| +44 | 2 | `k_zon_catalog`, `k_zon_control` |
| -203 | 1 | `k_sp1_generic` |
| -87 | 1 | `k_ptat_exitkrayt` |
| -12 | 1 | `k_pman_27_init01` |
| -6 | 1 | `k_ai_master` |
| -4 | 1 | `k_sup_droid` |
| +7 | 1 | `k_pla_actmap` |
| +18 | 1 | `k_sup_grenade` |
| +24 | 1 | `k_trg_senni` |
| +70 | 1 | `k_trg_transfail1` |
| +77 | 1 | `k_act_com45` |
| +161 | 1 | `k_con_comhack` |
| +274 | 1 | `k_trg_partyheart` |
| +305 | 1 | `k_trg_messengers` |
| +353 | 1 | `k_trg_stealth` |
| +743 | 1 | `k_qa` |
| +1143 | 1 | `k_sup_galaxmap` |
| +2171 | 1 | `k_ptat17aa_enter` |

## nwnnsscomp-v100

Edward T. Smith's nwnnsscomp, native build (modshop's mise run nwnnsscomp:build), -v1.00

Of the 133 outputs that differ from BioWare's bytes, 83 make the same engine calls with the same arguments and return the same result under the recording engine, 50 behave differently, and 0 do not run.

Failed (23):

- **undeclared** (3): uses a name the shipped sources do not declare: a constant BioWare's own build had (TRASK_DOOR_JAMMED), or a Neverwinter Nights function KotOR's table lacks (StartConversation, ResistSpell)
  - `k_pend_door04`: k_pend_door04.nss(5): Error: Undeclared identifier "TRASK_DOOR_JAMMED"
  - `nw_g0_conversat`: nw_g0_conversat.nss(14): Error: Undeclared identifier "StartConversation"
  - `nw_s0_lghtnbolt`: nw_s0_lghtnbolt.nss(58): Error: Undeclared identifier "ResistSpell"
- **stale-prototype** (15): BioWare compiled this against declarations that were not shipped: the shipped nwscript.nss prototype of AddPartyMember/RemovePartyMember, or k_inc_ebonhawk's forward declaration of EBO_PlayTakeOff, disagrees with the call or definition
  - `k_act_bastadd`: k_act_bastadd.nss(12): Error: Type mismatch in parameter 1 in call to "AddPartyMember"
  - `k_act_bastrmv`: k_act_bastrmv.nss(12): Error: Type mismatch in parameter 1 in call to "RemovePartyMember"
  - `k_act_canderrmv`: k_act_canderrmv.nss(12): Error: Type mismatch in parameter 1 in call to "RemovePartyMember"
  - `k_act_carthrmv`: k_act_carthrmv.nss(12): Error: Type mismatch in parameter 1 in call to "RemovePartyMember"
  - `k_act_hki47rmv`: k_act_hki47rmv.nss(12): Error: Type mismatch in parameter 1 in call to "RemovePartyMember"
  - `k_act_joleermv`: k_act_joleermv.nss(12): Error: Type mismatch in parameter 1 in call to "RemovePartyMember"
  - `k_act_juhanirmv`: k_act_juhanirmv.nss(12): Error: Type mismatch in parameter 1 in call to "RemovePartyMember"
  - `k_act_makeitem`: k_inc_ebonhawk.nss(209): Error: Function "EBO_PlayTakeOff"'s prototype doesn't match the declaration
  - `k_act_missionrmv`: k_act_missionrmv.nss(12): Error: Type mismatch in parameter 1 in call to "RemovePartyMember"
  - `k_act_t3m3rmv`: k_act_t3m3rmv.nss(12): Error: Type mismatch in parameter 1 in call to "RemovePartyMember"
  - `k_act_zaalrmv`: k_act_zaalrmv.nss(12): Error: Type mismatch in parameter 1 in call to "RemovePartyMember"
  - `k_con_makeitem`: k_inc_ebonhawk.nss(209): Error: Function "EBO_PlayTakeOff"'s prototype doesn't match the declaration
  - `k_pebn_galaxy`: k_inc_ebonhawk.nss(209): Error: Function "EBO_PlayTakeOff"'s prototype doesn't match the declaration
  - `k_sup_galaxmap`: k_inc_ebonhawk.nss(209): Error: Function "EBO_PlayTakeOff"'s prototype doesn't match the declaration
  - `k_sup_galaxymap`: k_inc_ebonhawk.nss(209): Error: Function "EBO_PlayTakeOff"'s prototype doesn't match the declaration
- **not-a-script** (5): an include file with no entry point; there is nothing to compile
  - `k_inc_generic`: exited 0 without writing k_inc_generic.ncs
  - `k_inc_gensupport`: exited 0 without writing k_inc_gensupport.ncs
  - `k_inc_switch`: exited 0 without writing k_inc_switch.ncs
  - `k_inc_unk`: exited 0 without writing k_inc_unk.ncs
  - `k_inc_walkways`: exited 0 without writing k_inc_walkways.ncs

Different output (133), by cause:

- **stale-include** (73): BioWare's bytecode was compiled before an include it uses last changed: the first difference is in an include's globals or functions (k_inc_utility's SW_CONSTANT_* globals, k_inc_generic's GN_TalentMasterRoutine, among others)
  `k_act_atkonend`, `k_act_caloatt`, `k_act_com31`, `k_act_com32`, `k_act_com33`, `k_act_com33a`, `k_act_com34`, `k_act_com35`, `k_act_com41`, `k_act_com42`, `k_act_com43`, `k_act_com45`, `k_act_comp01`, `k_act_electric`, `k_act_startgas`, `k_act_xoratt`, `k_actq_com33`, `k_actq_com33a`, `k_actq_com34`, `k_amb_enemyflee`, `k_amb_predator01`, `k_com_compart`, `k_com_comrepr`, `k_con_com31`, `k_con_com32`, `k_con_com33`, `k_con_com34`, `k_con_com35`, `k_con_com41`, `k_con_com42`, `k_con_com43`, `k_con_com44`, `k_con_com45`, `k_con_comactive`, `k_con_comhack`, `k_con_comrep`, `k_con_comver31`, `k_con_comver32`, `k_con_comver33`, `k_con_comver33a`, `k_con_comver34`, `k_con_comver41`, `k_con_comver42`, `k_con_comver43`, `k_con_comver44`, `k_con_comver45`, `k_con_tokens`, `k_def_rependd`, `k_pla_actmap`, `k_plc_tresciv`, `k_plc_trescorhig`, `k_plc_trescorlow`, `k_plc_trescormid`, `k_plc_tresdrdhig`, `k_plc_tresdrdlow`, `k_plc_tresdrdmid`, `k_plc_tresmilhig`, `k_plc_tresmillow`, `k_plc_tresmilmid`, `k_plc_tresrakat`, `k_plc_tresshahig`, `k_plc_tresshalow`, `k_plc_tresshamid`, `k_plc_tressndppl`, `k_ptat17aa_enter`, `k_ptat_gammenemy`, `k_sp1_generic`, `k_sup_star_map`, `k_trg_messengers`, `k_trg_partyheart`, `k_trg_stealth`, `k_zon_catalog`, `k_zon_control`
- **stale-bytecode** (8): BioWare's bytecode was compiled from an earlier version of this script's own source: the first difference is in the script, and it is a different call, constant or condition
  `k_ptat_exitkrayt`, `k_qa`, `k_sup_droid`, `k_sup_gohawk`, `k_sup_grenade`, `k_trg_senni`, `k_trg_transfail1`, `k_trp_generic`
- **conversation-eleven** (51): BioWare compiled this when ActionStartConversation took eleven arguments; the shipped declaration has twelve. Identical with --declarations bioware
  `k_bant_trig`, `k_def_bandspn`, `k_def_calodef`, `k_hjol_userdef01`, `k_pdan_elise_d`, `k_pdan_rapid01`, `k_pdan_rapid02`, `k_pdan_rapid03`, `k_pdan_rapid04`, `k_pdan_rapid05`, `k_pend_door01`, `k_pend_door02`, `k_pend_door03`, `k_pend_door08`, `k_pkas_freyyuser`, `k_pman_26a_area`, `k_pman_28ac_init`, `k_pman_at01`, `k_pman_at02`, `k_pman_door03`, `k_pman_door06`, `k_pman_door07`, `k_pman_door08`, `k_pman_door10`, `k_pman_door11`, `k_pman_door13`, `k_pman_door14`, `k_pman_door20`, `k_pman_door30`, `k_pman_init01`, `k_pman_init02`, `k_pman_init03`, `k_pman_init10`, `k_pman_judge1_d`, `k_pman_obj_asc`, `k_pman_obj_asc2`, `k_pman_obj_ascc`, `k_pman_obj_ascc2`, `k_pman_obj_ascn`, `k_pman_osc`, `k_pman_rapid01`, `k_pman_rapid02`, `k_pman_rapid03`, `k_pman_rapid04`, `k_pman_rapid05`, `k_pman_sitspeak`, `k_pman_survive01`, `k_ptar_sitharm`, `k_ptat_krayttran`, `k_swg_xor_user`, `k_trg_partyinit`
- **no known cause** (1): `k_ren_visionland`

Size differences, largest clusters first:

| Bytes | Scripts | Examples |
| --- | --- | --- |
| +6 | 32 | `k_def_bandspn`, `k_hjol_userdef01`, `k_pdan_elise_d` |
| +142 | 22 | `k_act_com41`, `k_act_comp01`, `k_act_electric` |
| -77 | 20 | `k_act_com32`, `k_act_com33`, `k_act_com33a` |
| -3798 | 14 | `k_plc_tresciv`, `k_plc_trescorhig`, `k_plc_trescorlow` |
| +42 | 10 | `k_pdan_rapid01`, `k_pdan_rapid02`, `k_pdan_rapid03` |
| +12 | 8 | `k_def_calodef`, `k_pend_door01`, `k_pman_26a_area` |
| +334 | 5 | `k_act_atkonend`, `k_act_caloatt`, `k_act_xoratt` |
| -58 | 2 | `k_act_com31`, `k_con_com31` |
| -27 | 2 | `k_sup_gohawk`, `k_trp_generic` |
| +44 | 2 | `k_zon_catalog`, `k_zon_control` |
| -203 | 1 | `k_sp1_generic` |
| -87 | 1 | `k_ptat_exitkrayt` |
| -4 | 1 | `k_sup_droid` |
| +0 | 1 | `k_ren_visionland` |
| +7 | 1 | `k_pla_actmap` |
| +18 | 1 | `k_sup_grenade` |
| +30 | 1 | `k_trg_senni` |
| +66 | 1 | `k_bant_trig` |
| +70 | 1 | `k_trg_transfail1` |
| +77 | 1 | `k_act_com45` |
| +161 | 1 | `k_con_comhack` |
| +274 | 1 | `k_trg_partyheart` |
| +341 | 1 | `k_trg_messengers` |
| +353 | 1 | `k_trg_stealth` |
| +785 | 1 | `k_qa` |
| +2177 | 1 | `k_ptat17aa_enter` |

## nwnnsscomp

Edward T. Smith's nwnnsscomp, native build, default version

Of the 527 outputs that differ from BioWare's bytes, 455 make the same engine calls with the same arguments and return the same result under the recording engine, 72 behave differently, and 0 do not run.

Failed (23):

- **undeclared** (3): uses a name the shipped sources do not declare: a constant BioWare's own build had (TRASK_DOOR_JAMMED), or a Neverwinter Nights function KotOR's table lacks (StartConversation, ResistSpell)
  - `k_pend_door04`: k_pend_door04.nss(5): Error: Undeclared identifier "TRASK_DOOR_JAMMED"
  - `nw_g0_conversat`: nw_g0_conversat.nss(14): Error: Undeclared identifier "StartConversation"
  - `nw_s0_lghtnbolt`: nw_s0_lghtnbolt.nss(58): Error: Undeclared identifier "ResistSpell"
- **stale-prototype** (15): BioWare compiled this against declarations that were not shipped: the shipped nwscript.nss prototype of AddPartyMember/RemovePartyMember, or k_inc_ebonhawk's forward declaration of EBO_PlayTakeOff, disagrees with the call or definition
  - `k_act_bastadd`: k_act_bastadd.nss(12): Error: Type mismatch in parameter 1 in call to "AddPartyMember"
  - `k_act_bastrmv`: k_act_bastrmv.nss(12): Error: Type mismatch in parameter 1 in call to "RemovePartyMember"
  - `k_act_canderrmv`: k_act_canderrmv.nss(12): Error: Type mismatch in parameter 1 in call to "RemovePartyMember"
  - `k_act_carthrmv`: k_act_carthrmv.nss(12): Error: Type mismatch in parameter 1 in call to "RemovePartyMember"
  - `k_act_hki47rmv`: k_act_hki47rmv.nss(12): Error: Type mismatch in parameter 1 in call to "RemovePartyMember"
  - `k_act_joleermv`: k_act_joleermv.nss(12): Error: Type mismatch in parameter 1 in call to "RemovePartyMember"
  - `k_act_juhanirmv`: k_act_juhanirmv.nss(12): Error: Type mismatch in parameter 1 in call to "RemovePartyMember"
  - `k_act_makeitem`: k_inc_ebonhawk.nss(209): Error: Function "EBO_PlayTakeOff"'s prototype doesn't match the declaration
  - `k_act_missionrmv`: k_act_missionrmv.nss(12): Error: Type mismatch in parameter 1 in call to "RemovePartyMember"
  - `k_act_t3m3rmv`: k_act_t3m3rmv.nss(12): Error: Type mismatch in parameter 1 in call to "RemovePartyMember"
  - `k_act_zaalrmv`: k_act_zaalrmv.nss(12): Error: Type mismatch in parameter 1 in call to "RemovePartyMember"
  - `k_con_makeitem`: k_inc_ebonhawk.nss(209): Error: Function "EBO_PlayTakeOff"'s prototype doesn't match the declaration
  - `k_pebn_galaxy`: k_inc_ebonhawk.nss(209): Error: Function "EBO_PlayTakeOff"'s prototype doesn't match the declaration
  - `k_sup_galaxmap`: k_inc_ebonhawk.nss(209): Error: Function "EBO_PlayTakeOff"'s prototype doesn't match the declaration
  - `k_sup_galaxymap`: k_inc_ebonhawk.nss(209): Error: Function "EBO_PlayTakeOff"'s prototype doesn't match the declaration
- **not-a-script** (5): an include file with no entry point; there is nothing to compile
  - `k_inc_generic`: exited 0 without writing k_inc_generic.ncs
  - `k_inc_gensupport`: exited 0 without writing k_inc_gensupport.ncs
  - `k_inc_switch`: exited 0 without writing k_inc_switch.ncs
  - `k_inc_unk`: exited 0 without writing k_inc_unk.ncs
  - `k_inc_walkways`: exited 0 without writing k_inc_walkways.ncs

Different output (527), by cause:

- **stale-include** (73): BioWare's bytecode was compiled before an include it uses last changed: the first difference is in an include's globals or functions (k_inc_utility's SW_CONSTANT_* globals, k_inc_generic's GN_TalentMasterRoutine, among others)
  `k_act_atkonend`, `k_act_caloatt`, `k_act_com31`, `k_act_com32`, `k_act_com33`, `k_act_com33a`, `k_act_com34`, `k_act_com35`, `k_act_com41`, `k_act_com42`, `k_act_com43`, `k_act_com45`, `k_act_comp01`, `k_act_electric`, `k_act_startgas`, `k_act_xoratt`, `k_actq_com33`, `k_actq_com33a`, `k_actq_com34`, `k_amb_enemyflee`, `k_amb_predator01`, `k_com_compart`, `k_com_comrepr`, `k_con_com31`, `k_con_com32`, `k_con_com33`, `k_con_com34`, `k_con_com35`, `k_con_com41`, `k_con_com42`, `k_con_com43`, `k_con_com44`, `k_con_com45`, `k_con_comactive`, `k_con_comhack`, `k_con_comrep`, `k_con_comver31`, `k_con_comver32`, `k_con_comver33`, `k_con_comver33a`, `k_con_comver34`, `k_con_comver41`, `k_con_comver42`, `k_con_comver43`, `k_con_comver44`, `k_con_comver45`, `k_con_tokens`, `k_def_rependd`, `k_pla_actmap`, `k_plc_tresciv`, `k_plc_trescorhig`, `k_plc_trescorlow`, `k_plc_trescormid`, `k_plc_tresdrdhig`, `k_plc_tresdrdlow`, `k_plc_tresdrdmid`, `k_plc_tresmilhig`, `k_plc_tresmillow`, `k_plc_tresmilmid`, `k_plc_tresrakat`, `k_plc_tresshahig`, `k_plc_tresshalow`, `k_plc_tresshamid`, `k_plc_tressndppl`, `k_ptat17aa_enter`, `k_ptat_gammenemy`, `k_sp1_generic`, `k_sup_star_map`, `k_trg_messengers`, `k_trg_partyheart`, `k_trg_stealth`, `k_zon_catalog`, `k_zon_control`
- **stale-bytecode** (8): BioWare's bytecode was compiled from an earlier version of this script's own source: the first difference is in the script, and it is a different call, constant or condition
  `k_ptat_exitkrayt`, `k_qa`, `k_sup_droid`, `k_sup_gohawk`, `k_sup_grenade`, `k_trg_senni`, `k_trg_transfail1`, `k_trp_generic`
- **conversation-twelve** (2): BioWare compiled this against the shipped twelve-argument ActionStartConversation, unlike most of its scripts. Identical with the shipped declarations
  `k_ai_master`, `k_pman_27_init01`
- **conversation-eleven** (51): BioWare compiled this when ActionStartConversation took eleven arguments; the shipped declaration has twelve. Identical with --declarations bioware
  `k_bant_trig`, `k_def_bandspn`, `k_def_calodef`, `k_hjol_userdef01`, `k_pdan_elise_d`, `k_pdan_rapid01`, `k_pdan_rapid02`, `k_pdan_rapid03`, `k_pdan_rapid04`, `k_pdan_rapid05`, `k_pend_door01`, `k_pend_door02`, `k_pend_door03`, `k_pend_door08`, `k_pkas_freyyuser`, `k_pman_26a_area`, `k_pman_28ac_init`, `k_pman_at01`, `k_pman_at02`, `k_pman_door03`, `k_pman_door06`, `k_pman_door07`, `k_pman_door08`, `k_pman_door10`, `k_pman_door11`, `k_pman_door13`, `k_pman_door14`, `k_pman_door20`, `k_pman_door30`, `k_pman_init01`, `k_pman_init02`, `k_pman_init03`, `k_pman_init10`, `k_pman_judge1_d`, `k_pman_obj_asc`, `k_pman_obj_asc2`, `k_pman_obj_ascc`, `k_pman_obj_ascc2`, `k_pman_obj_ascn`, `k_pman_osc`, `k_pman_rapid01`, `k_pman_rapid02`, `k_pman_rapid03`, `k_pman_rapid04`, `k_pman_rapid05`, `k_pman_sitspeak`, `k_pman_survive01`, `k_ptar_sitharm`, `k_ptat_krayttran`, `k_swg_xor_user`, `k_trg_partyinit`
- **no known cause** (393): `k_act_bandatt`, `k_act_com44`, `k_act_darkhigh`, `k_act_darkmed`, `k_act_darksml`, `k_act_exitarea`, `k_act_exitarea2`, `k_act_lighthigh`, `k_act_lightmed`, `k_act_lightsml`, `k_act_talktrue`, `k_act_walkways`, `k_act_walkways2`, `k_amb_prey_spawn`, `k_amb_prey_ude`, `k_cheat_01`, `k_cht_n_bast`, `k_cht_n_cand`, `k_cht_n_carth`, `k_cht_n_hk47`, `k_cht_n_jolee`, `k_cht_n_juhani`, `k_cht_n_mission`, `k_cht_n_t3m4`, `k_cht_n_zaalbar`, `k_cht_p_dan`, `k_cht_p_end`, `k_cht_p_kas`, `k_cht_p_kor`, `k_cht_p_lev`, `k_cht_p_man`, `k_cht_p_sta`, `k_cht_p_tar`, `k_cht_p_tat`, `k_cht_p_unk`, `k_con_chrhigh`, `k_con_chrlow`, `k_con_chrmiddle`, `k_con_chrnormal`, `k_con_dark`, `k_con_darkhigh`, `k_con_darklow`, `k_con_flirt`, `k_con_flirtno`, `k_con_fperslow`, `k_con_inrange`, `k_con_inthigh`, `k_con_intlow`, `k_con_intnormal`, `k_con_light`, `k_con_lighthigh`, `k_con_lightlow`, `k_con_neutral`, `k_con_perseasy`, `k_con_pershigh`, `k_con_persmed`, `k_def_ambient`, `k_def_ambmob`, `k_def_buff`, `k_def_calospn`, `k_def_grenspn`, `k_def_repairsp`, `k_def_repuser`, `k_def_spawn01`, `k_def_userdef01`, `k_hcan_state22`, `k_hcan_state23`, `k_hen_candspwn`, `k_hen_canduser`, `k_hen_spawn01`, `k_hjol_spawn01`, `k_hjuh_h12`, `k_hjuh_p07`, `k_hjuh_p07d`, `k_hjuh_p07l`, `k_hjuh_p15`, `k_hjuh_p21`, `k_hjuh_p22`, `k_hjuh_p23`, `k_hjuh_p28`, `k_hmis_talk6-8`, `k_hzal_atkash2`, `k_hzal_beenkashy`, `k_hzal_chuundead`, `k_hzal_freyydead`, `k_hzal_notkash2`, `k_pdan_belaya02`, `k_pdan_droid50`, `k_pdan_iriaz_sp`, `k_pdan_iriaz_ud`, `k_pdan_jedi21`, `k_pdan_kath02`, `k_pdan_kath03`, `k_pdan_kath04`, `k_pdan_mand04`, `k_pdan_mand52`, `k_pdan_stand02`, `k_pebn_loc01`, `k_pebn_loc02`, `k_pebn_loc03`, `k_pebn_loc04`, `k_pebn_loc05`, `k_pebn_loc06`, `k_pebn_loc07`, `k_pebn_loc08`, `k_pebn_loc09`, `k_pebn_swoop`, `k_pend_door06`, `k_pend_explode`, `k_pkas_chuundead`, `k_pkas_freyydead`, `k_pkas_freyyhelp`, `k_pkas_freyyspwn`, `k_pkas_hashilt`, `k_pkas_hassword`, `k_pkas_heart_on`, `k_pkas_jaarakblt`, `k_pkas_jaaraksus`, `k_pkas_jaardead`, `k_pkas_jaarfreed`, `k_pkas_nohilt`, `k_pkas_nosword`, `k_pkas_randwalk`, `k_pman_26a_aread`, `k_pman_26d_area`, `k_pman_26d_triga`, `k_pman_26d_trigb`, `k_pman_26d_trigc`, `k_pman_27_area`, `k_pman_28d_cmp01`, `k_pman_28d_cmp02`, `k_pman_28d_triga`, `k_pman_28d_trigb`, `k_pman_airlock02`, `k_pman_amb_9`, `k_pman_ambient05`, `k_pman_area_28a`, `k_pman_argrep_d`, `k_pman_argsith_d`, `k_pman_beast_9`, `k_pman_casamad_s`, `k_pman_casamad_y`, `k_pman_casname_n`, `k_pman_casname_s`, `k_pman_champ_f01`, `k_pman_champ_f02`, `k_pman_champ_f03`, `k_pman_champ_f04`, `k_pman_champ_m01`, `k_pman_champ_m02`, `k_pman_champ_m03`, `k_pman_champ_m04`, `k_pman_champ_qui`, `k_pman_comp04`, `k_pman_comp09`, `k_pman_door32`, `k_pman_hukname_n`, `k_pman_hukname_s`, `k_pman_huktmad_1`, `k_pman_huktmad_y`, `k_pman_locked02`, `k_pman_lostset_n`, `k_pman_lostset_q`, `k_pman_lostset_y`, `k_pman_misc07`, `k_pman_miss01`, `k_pman_missing40`, `k_pman_murder22`, `k_pman_murder55`, `k_pman_murder75`, `k_pman_murder95`, `k_pman_murder97`, `k_pman_murder98`, `k_pman_murder99`, `k_pman_murder9a`, `k_pman_murder9b`, `k_pman_murder9c`, `k_pman_murder9r`, `k_pman_murder9u`, `k_pman_no_threat`, `k_pman_notpaid_c`, `k_pman_notpaid_n`, `k_pman_notpaid_s`, `k_pman_payrace_1`, `k_pman_payrace_2`, `k_pman_payrace_3`, `k_pman_persuad_f`, `k_pman_persuad_s`, `k_pman_persuad_y`, `k_pman_planet57`, `k_pman_planet58`, `k_pman_planet59`, `k_pman_planet64`, `k_pman_planet65`, `k_pman_planet66`, `k_pman_planet67`, `k_pman_planet68`, `k_pman_planet69`, `k_pman_planet70`, `k_pman_planet71`, `k_pman_planet72`, `k_pman_puzzle06`, `k_pman_puzzle07`, `k_pman_puzzle08`, `k_pman_puzzle09`, `k_pman_puzzle10`, `k_pman_puzzle11`, `k_pman_qreturn_n`, `k_pman_qreturn_s`, `k_pman_queecham2`, `k_pman_queechamp`, `k_pman_queelostp`, `k_pman_queemad_s`, `k_pman_queemad_y`, `k_pman_queenotpc`, `k_pman_queeorpcy`, `k_pman_queequit`, `k_pman_quename_n`, `k_pman_quename_s`, `k_pman_quepaid_n`, `k_pman_quepaid_s`, `k_pman_quepaid_y`, `k_pman_quetell_n`, `k_pman_quetell_s`, `k_pman_quetell_y`, `k_pman_quleave_s`, `k_pman_quleave_y`, `k_pman_race_0_02`, `k_pman_race_0_03`, `k_pman_race_0_04`, `k_pman_race_0_q1`, `k_pman_race_0_q2`, `k_pman_race_0_q3`, `k_pman_race_0_q4`, `k_pman_race_1_02`, `k_pman_race_1_03`, `k_pman_race_1_04`, `k_pman_race_2_02`, `k_pman_race_2_03`, `k_pman_race_2_04`, `k_pman_race_3_02`, `k_pman_race_3_03`, `k_pman_race_3_04`, `k_pman_race_s_01`, `k_pman_race_s_02`, `k_pman_race_s_03`, `k_pman_race_s_04`, `k_pman_race_x_02`, `k_pman_race_x_03`, `k_pman_race_x_04`, `k_pman_racecha_n`, `k_pman_racechamp`, `k_pman_racelosty`, `k_pman_racenone`, `k_pman_raceone`, `k_pman_raceonex`, `k_pman_raceset_1`, `k_pman_raceset_2`, `k_pman_raceset_3`, `k_pman_raceset_4`, `k_pman_racetwo`, `k_pman_racetwox`, `k_pman_random02`, `k_pman_random03`, `k_pman_random04`, `k_pman_random05`, `k_pman_random17`, `k_pman_random18`, `k_pman_random19`, `k_pman_random20`, `k_pman_random29`, `k_pman_random30`, `k_pman_shield_9`, `k_pman_sonic`, `k_pman_suituse`, `k_pman_survive02`, `k_pman_tatracern`, `k_pman_tatracery`, `k_pman_ww`, `k_pman_ye_threat`, `k_ptat_1gamewon`, `k_ptat_2gamewon`, `k_ptat_3gamewon`, `k_ptat_amb1`, `k_ptat_angrgurke`, `k_ptat_baddeal`, `k_ptat_beenkash`, `k_ptat_bounty_n`, `k_ptat_bounty_y`, `k_ptat_bountyset`, `k_ptat_bullynico`, `k_ptat_celisdeal`, `k_ptat_checkmad`, `k_ptat_dockpaid`, `k_ptat_dockpay`, `k_ptat_dorakname`, `k_ptat_dorakwho`, `k_ptat_gaffiwrit`, `k_ptat_gammbad`, `k_ptat_gammbribe`, `k_ptat_gammgone`, `k_ptat_gammhunt`, `k_ptat_gammleave`, `k_ptat_gammmean`, `k_ptat_gammwarn`, `k_ptat_gandmad`, `k_ptat_gandname`, `k_ptat_garmname`, `k_ptat_garmwho`, `k_ptat_gateperm`, `k_ptat_gizkaspwn`, `k_ptat_gizkauser`, `k_ptat_gooddeal`, `k_ptat_gurkemad`, `k_ptat_has_k_map`, `k_ptat_hasgaffi`, `k_ptat_haspearl`, `k_ptat_haspermit`, `k_ptat_haswrit`, `k_ptat_huntask_n`, `k_ptat_huntask_y`, `k_ptat_koname_ch`, `k_ptat_konameset`, `k_ptat_losscheck`, `k_ptat_m_office`, `k_ptat_namecheck`, `k_ptat_nicenico`, `k_ptat_nicofree`, `k_ptat_nicohappy`, `k_ptat_nicoinfo`, `k_ptat_niconews`, `k_ptat_nicotalk`, `k_ptat_nicowait`, `k_ptat_no_k_map`, `k_ptat_nohkinfo`, `k_ptat_nonicotlk`, `k_ptat_nopermit`, `k_ptat_nosalehk`, `k_ptat_notracer`, `k_ptat_nowrit`, `k_ptat_officemad`, `k_ptat_pazzloss`, `k_ptat_pazzwin1`, `k_ptat_pazzwin2`, `k_ptat_pazzwin3`, `k_ptat_persyuka`, `k_ptat_racechamp`, `k_ptat_racenone`, `k_ptat_raceone`, `k_ptat_racer`, `k_ptat_racer_id`, `k_ptat_racetrial`, `k_ptat_racetwo`, `k_ptat_salehk`, `k_ptat_sandper`, `k_ptat_sethunt`, `k_ptat_shariwait`, `k_ptat_shartrust`, `k_ptat_soldhk`, `k_ptat_soldpearl`, `k_ptat_talktanis`, `k_ptat_taname_ch`, `k_ptat_tanameset`, `k_ptat_tanisdead`, `k_ptat_tanisgave`, `k_ptat_tanismad`, `k_ptat_tatinfo_c`, `k_ptat_tatinfo_s`, `k_ptat_toldofhk`, `k_ptat_tuskpdone`, `k_ptat_yukanpers`, `k_ptat_yukarname`, `k_ptat_yukarwho`, `k_ptat_zoriiname`, `k_ptat_zoriiwho`, `k_ren_leavlev`, `k_ren_nextplanet`, `k_ren_turret`, `k_ren_visionland`, `k_sup_bands`, `k_sup_comshots`, `k_sup_guiopen`, `k_sup_healing`, `k_sup_static`, `k_swg_bastila22`, `k_swg_carth13`, `k_swg_hk4713`, `k_swg_hk47upg01`, `k_swg_jolee13`, `k_swg_jolee15`, `k_swg_jolee18`, `k_swg_jolee19`, `k_swg_xor_def`

Size differences, largest clusters first:

| Bytes | Scripts | Examples |
| --- | --- | --- |
| +2 | 163 | `k_act_exitarea`, `k_act_exitarea2`, `k_con_chrhigh` |
| +0 | 66 | `k_act_talktrue`, `k_con_flirtno`, `k_con_fperslow` |
| +4 | 57 | `k_con_chrmiddle`, `k_hjol_spawn01`, `k_pend_explode` |
| +6 | 37 | `k_con_dark`, `k_con_light`, `k_hjuh_p15` |
| +64 | 16 | `k_amb_prey_spawn`, `k_def_ambient`, `k_def_ambmob` |
| +8 | 15 | `k_act_com44`, `k_con_perseasy`, `k_con_pershigh` |
| +142 | 15 | `k_act_com41`, `k_act_electric`, `k_amb_enemyflee` |
| -3790 | 14 | `k_plc_tresciv`, `k_plc_trescorhig`, `k_plc_trescorlow` |
| +18 | 14 | `k_cht_p_dan`, `k_cht_p_end`, `k_cht_p_kas` |
| +16 | 13 | `k_cht_n_bast`, `k_cht_n_cand`, `k_cht_n_carth` |
| +10 | 12 | `k_pman_28ac_init`, `k_pman_door06`, `k_pman_door07` |
| -73 | 10 | `k_act_com35`, `k_con_com32`, `k_con_com33` |
| +42 | 10 | `k_pdan_rapid01`, `k_pdan_rapid02`, `k_pdan_rapid03` |
| -69 | 9 | `k_act_com32`, `k_act_com33`, `k_act_com33a` |
| +14 | 9 | `k_con_neutral`, `k_def_repairsp`, `k_hen_spawn01` |
| +12 | 6 | `k_act_darkhigh`, `k_act_darkmed`, `k_act_darksml` |
| +22 | 5 | `k_def_buff`, `k_def_userdef01`, `k_pdan_kath04` |
| +30 | 5 | `k_ptat_gizkauser`, `k_ren_nextplanet`, `k_ren_turret` |
| +852 | 4 | `k_act_atkonend`, `k_act_caloatt`, `k_act_xoratt` |
| +50 | 3 | `k_act_walkways`, `k_act_walkways2`, `k_pman_ww` |
| +144 | 3 | `k_act_comp01`, `k_act_startgas`, `k_com_comrepr` |
| +20 | 2 | `k_amb_prey_ude`, `k_hen_canduser` |
| +28 | 2 | `k_pkas_randwalk`, `k_sup_bands` |
| +32 | 2 | `k_pkas_freyyuser`, `k_ren_visionland` |
| +34 | 2 | `k_cheat_01`, `k_hjol_userdef01` |
| +36 | 2 | `k_def_calodef`, `k_ren_leavlev` |
| +44 | 2 | `k_zon_catalog`, `k_zon_control` |
| +48 | 2 | `k_sup_grenade`, `k_trg_partyinit` |
| +70 | 2 | `k_def_bandspn`, `k_trg_transfail1` |
| -113 | 1 | `k_sp1_generic` |
| -87 | 1 | `k_ptat_exitkrayt` |
| -75 | 1 | `k_con_tokens` |
| -54 | 1 | `k_con_com31` |
| -50 | 1 | `k_act_com31` |
| -21 | 1 | `k_trp_generic` |
| +1 | 1 | `k_sup_gohawk` |
| +25 | 1 | `k_pla_actmap` |
| +66 | 1 | `k_bant_trig` |
| +72 | 1 | `k_pdan_iriaz_ud` |
| +78 | 1 | `k_pebn_swoop` |
| +85 | 1 | `k_act_com45` |
| +146 | 1 | `k_amb_predator01` |
| +148 | 1 | `k_com_compart` |
| +150 | 1 | `k_plc_tresdrdhig` |
| +154 | 1 | `k_sup_star_map` |
| +163 | 1 | `k_con_comhack` |
| +274 | 1 | `k_trg_partyheart` |
| +341 | 1 | `k_trg_messengers` |
| +357 | 1 | `k_trg_stealth` |
| +518 | 1 | `k_act_bandatt` |
| +606 | 1 | `k_ai_master` |
| +785 | 1 | `k_qa` |
| +854 | 1 | `k_def_rependd` |
| +2177 | 1 | `k_ptat17aa_enter` |

## pykotor

pykotor's built-in compiler, compile_nss(source, Game.K1), from modshop's environment (uv sync there first)

Of the 1217 outputs that differ from BioWare's bytes, 753 make the same engine calls with the same arguments and return the same result under the recording engine, 440 behave differently, and 24 do not run.

Failed (275):

- **undeclared** (3): uses a name the shipped sources do not declare: a constant BioWare's own build had (TRASK_DOOR_JAMMED), or a Neverwinter Nights function KotOR's table lacks (StartConversation, ResistSpell)
  - `k_pend_door04`: error: CompileError: Undefined variable 'TRASK_DOOR_JAMMED'
  - `nw_g0_conversat`: error: CompileError: Undefined function 'StartConversation'
  - `nw_s0_lghtnbolt`: error: CompileError: Undefined function 'ResistSpell'
- **stale-prototype** (15): BioWare compiled this against declarations that were not shipped: the shipped nwscript.nss prototype of AddPartyMember/RemovePartyMember, or k_inc_ebonhawk's forward declaration of EBO_PlayTakeOff, disagrees with the call or definition
  - `k_act_bastadd`: error: CompileError: Missing required arguments for 'AddPartyMember'
  - `k_act_bastrmv`: error: CompileError: Type mismatch for parameter 'nNPC' in call to 'RemovePartyMember'
  - `k_act_canderrmv`: error: CompileError: Type mismatch for parameter 'nNPC' in call to 'RemovePartyMember'
  - `k_act_carthrmv`: error: CompileError: Type mismatch for parameter 'nNPC' in call to 'RemovePartyMember'
  - `k_act_hki47rmv`: error: CompileError: Type mismatch for parameter 'nNPC' in call to 'RemovePartyMember'
  - `k_act_joleermv`: error: CompileError: Type mismatch for parameter 'nNPC' in call to 'RemovePartyMember'
  - `k_act_juhanirmv`: error: CompileError: Type mismatch for parameter 'nNPC' in call to 'RemovePartyMember'
  - `k_act_makeitem`: error: CompileError: Function 'EBO_BastilaStartConversation2' already has a prototype or been defined.
  - `k_act_missionrmv`: error: CompileError: Type mismatch for parameter 'nNPC' in call to 'RemovePartyMember'
  - `k_act_t3m3rmv`: error: CompileError: Type mismatch for parameter 'nNPC' in call to 'RemovePartyMember'
  - `k_act_zaalrmv`: error: CompileError: Type mismatch for parameter 'nNPC' in call to 'RemovePartyMember'
  - `k_con_makeitem`: error: CompileError: Function 'EBO_BastilaStartConversation2' already has a prototype or been defined.
  - `k_pebn_galaxy`: error: CompileError: Function 'EBO_BastilaStartConversation2' already has a prototype or been defined.
  - `k_sup_galaxmap`: error: CompileError: Function 'EBO_BastilaStartConversation2' already has a prototype or been defined.
  - `k_sup_galaxymap`: error: CompileError: Function 'EBO_BastilaStartConversation2' already has a prototype or been defined.
- **stale-include** (2): BioWare's bytecode was compiled before an include it uses last changed: the first difference is in an include's globals or functions (k_inc_utility's SW_CONSTANT_* globals, k_inc_generic's GN_TalentMasterRoutine, among others)
  - `k_ptat17aa_enter`: error: CompileError: Syntax error at line 1840, position 51873, token='nOp'
  - `k_ptat_gammenemy`: error: CompileError: Syntax error at line 1843, position 51873, token='nOp'
- **not-a-script** (5): an include file with no entry point; there is nothing to compile
  - `k_inc_generic`: error: EntryPointError: This file has no entry point and cannot be compiled (Most likely an include file).
  - `k_inc_gensupport`: error: EntryPointError: This file has no entry point and cannot be compiled (Most likely an include file).
  - `k_inc_switch`: error: EntryPointError: This file has no entry point and cannot be compiled (Most likely an include file).
  - `k_inc_unk`: error: EntryPointError: This file has no entry point and cannot be compiled (Most likely an include file).
  - `k_inc_walkways`: error: EntryPointError: This file has no entry point and cannot be compiled (Most likely an include file).
- **conversation-twelve** (1): BioWare compiled this against the shipped twelve-argument ActionStartConversation, unlike most of its scripts. Identical with the shipped declarations
  - `k_pman_27_init01`: error: CompileError: Syntax error at line 57, position 1254, token='nOp'
- **conversation-eleven** (22): BioWare compiled this when ActionStartConversation took eleven arguments; the shipped declaration has twelve. Identical with --declarations bioware
  - `k_pdan_rapid05`: error: CompileError: Syntax error at line 53, position 1254, token='nOp'
  - `k_pman_26a_area`: error: CompileError: Syntax error at line 74, position 1254, token='nOp'
  - `k_pman_28ac_init`: error: CompileError: Syntax error at line 52, position 1254, token='nOp'
  - `k_pman_at01`: error: CompileError: Syntax error at line 55, position 1254, token='nOp'
  - `k_pman_at02`: error: CompileError: Syntax error at line 56, position 1254, token='nOp'
  - `k_pman_door06`: error: CompileError: Syntax error at line 54, position 1254, token='nOp'
  - `k_pman_door07`: error: CompileError: Syntax error at line 54, position 1254, token='nOp'
  - `k_pman_door10`: error: CompileError: Syntax error at line 51, position 1254, token='nOp'
  - `k_pman_door11`: error: CompileError: Syntax error at line 62, position 1254, token='nOp'
  - `k_pman_door13`: error: CompileError: Syntax error at line 53, position 1254, token='nOp'
  - `k_pman_door14`: error: CompileError: Syntax error at line 53, position 1254, token='nOp'
  - `k_pman_door30`: error: CompileError: Syntax error at line 74, position 1254, token='nOp'
  - `k_pman_init01`: error: CompileError: Syntax error at line 55, position 1254, token='nOp'
  - `k_pman_init02`: error: CompileError: Syntax error at line 53, position 1254, token='nOp'
  - `k_pman_init03`: error: CompileError: Syntax error at line 54, position 1254, token='nOp'
  - `k_pman_init10`: error: CompileError: Syntax error at line 70, position 1254, token='nOp'
  - `k_pman_rapid01`: error: CompileError: Syntax error at line 53, position 1254, token='nOp'
  - `k_pman_rapid02`: error: CompileError: Syntax error at line 53, position 1254, token='nOp'
  - `k_pman_rapid03`: error: CompileError: Syntax error at line 53, position 1254, token='nOp'
  - `k_pman_rapid04`: error: CompileError: Syntax error at line 53, position 1254, token='nOp'
  - `k_pman_rapid05`: error: CompileError: Syntax error at line 53, position 1254, token='nOp'
  - `k_ptat_krayttran`: error: CompileError: Syntax error at line 1848, position 51873, token='nOp'
- **no known cause** (227)
  - `k_act_mispaz`: error: CompileError: Type mismatch for parameter 'nMaxWager' in call to 'PlayPazaak'
  - `k_pdan_mand51`: error: CompileError: Syntax error at line 67, position 1254, token='nOp'
  - `k_pman_26a_aread`: error: CompileError: Syntax error at line 128, position 1254, token='nOp'
  - `k_pman_26d_area`: error: CompileError: Syntax error at line 124, position 1254, token='nOp'
  - `k_pman_26d_triga`: error: CompileError: Syntax error at line 61, position 1254, token='nOp'
  - `k_pman_26d_trigb`: error: CompileError: Syntax error at line 61, position 1254, token='nOp'
  - `k_pman_26d_trigc`: error: CompileError: Syntax error at line 66, position 1254, token='nOp'
  - `k_pman_27_area`: error: CompileError: Syntax error at line 76, position 1254, token='nOp'
  - `k_pman_28b_at01`: error: CompileError: Syntax error at line 47, position 1254, token='nOp'
  - `k_pman_28b_at02`: error: CompileError: Syntax error at line 47, position 1254, token='nOp'
  - `k_pman_28d_cmp01`: error: CompileError: Syntax error at line 63, position 1254, token='nOp'
  - `k_pman_28d_cmp02`: error: CompileError: Syntax error at line 75, position 1254, token='nOp'
  - `k_pman_28d_triga`: error: CompileError: Syntax error at line 64, position 1254, token='nOp'
  - `k_pman_28d_trigb`: error: CompileError: Syntax error at line 57, position 1254, token='nOp'
  - `k_pman_aioff`: error: CompileError: Syntax error at line 48, position 1254, token='nOp'
  - `k_pman_aion`: error: CompileError: Syntax error at line 48, position 1254, token='nOp'
  - `k_pman_airlock02`: error: CompileError: Syntax error at line 91, position 1254, token='nOp'
  - `k_pman_airlock03`: error: CompileError: Syntax error at line 52, position 1254, token='nOp'
  - `k_pman_area_28a`: error: CompileError: Syntax error at line 79, position 1254, token='nOp'
  - `k_pman_beast_9`: error: CompileError: Syntax error at line 75, position 1254, token='nOp'
  - `k_pman_casamad_s`: error: CompileError: Syntax error at line 53, position 1254, token='nOp'
  - `k_pman_casamad_y`: error: CompileError: Syntax error at line 60, position 1254, token='nOp'
  - `k_pman_casname_n`: error: CompileError: Syntax error at line 62, position 1254, token='nOp'
  - `k_pman_casname_s`: error: CompileError: Syntax error at line 55, position 1254, token='nOp'
  - `k_pman_champ_f01`: error: CompileError: Syntax error at line 67, position 1254, token='nOp'
  - `k_pman_champ_f02`: error: CompileError: Syntax error at line 68, position 1254, token='nOp'
  - `k_pman_champ_f03`: error: CompileError: Syntax error at line 67, position 1254, token='nOp'
  - `k_pman_champ_f04`: error: CompileError: Syntax error at line 66, position 1254, token='nOp'
  - `k_pman_champ_m01`: error: CompileError: Syntax error at line 66, position 1254, token='nOp'
  - `k_pman_champ_m02`: error: CompileError: Syntax error at line 69, position 1254, token='nOp'
  - `k_pman_champ_m03`: error: CompileError: Syntax error at line 65, position 1254, token='nOp'
  - `k_pman_champ_m04`: error: CompileError: Syntax error at line 65, position 1254, token='nOp'
  - `k_pman_champ_qui`: error: CompileError: Syntax error at line 65, position 1254, token='nOp'
  - `k_pman_comp02`: error: CompileError: Syntax error at line 46, position 1254, token='nOp'
  - `k_pman_comp04`: error: CompileError: Syntax error at line 61, position 1254, token='nOp'
  - `k_pman_comp05`: error: CompileError: Syntax error at line 48, position 1254, token='nOp'
  - `k_pman_comp09`: error: CompileError: Syntax error at line 46, position 1254, token='nOp'
  - `k_pman_comp10`: error: CompileError: Syntax error at line 46, position 1254, token='nOp'
  - `k_pman_comp11`: error: CompileError: Syntax error at line 46, position 1254, token='nOp'
  - `k_pman_comp31`: error: CompileError: Syntax error at line 46, position 1254, token='nOp'
  - `k_pman_comp32`: error: CompileError: Syntax error at line 46, position 1254, token='nOp'
  - `k_pman_door40`: error: CompileError: Syntax error at line 64, position 1254, token='nOp'
  - `k_pman_firixa_d`: error: CompileError: Syntax error at line 56, position 1254, token='nOp'
  - `k_pman_hukname_n`: error: CompileError: Syntax error at line 63, position 1254, token='nOp'
  - `k_pman_hukname_s`: error: CompileError: Syntax error at line 55, position 1254, token='nOp'
  - `k_pman_huktmad_1`: error: CompileError: Syntax error at line 70, position 1254, token='nOp'
  - `k_pman_huktmad_y`: error: CompileError: Syntax error at line 62, position 1254, token='nOp'
  - `k_pman_lostset_n`: error: CompileError: Syntax error at line 63, position 1254, token='nOp'
  - `k_pman_lostset_q`: error: CompileError: Syntax error at line 63, position 1254, token='nOp'
  - `k_pman_lostset_y`: error: CompileError: Syntax error at line 62, position 1254, token='nOp'
  - `k_pman_misc07`: error: CompileError: Syntax error at line 48, position 1254, token='nOp'
  - `k_pman_miss01`: error: CompileError: Syntax error at line 52, position 1254, token='nOp'
  - `k_pman_missing24`: error: CompileError: Syntax error at line 149, position 1254, token='nOp'
  - `k_pman_missing25`: error: CompileError: Syntax error at line 51, position 1254, token='nOp'
  - `k_pman_missing29`: error: CompileError: Syntax error at line 51, position 1254, token='nOp'
  - `k_pman_missing40`: error: CompileError: Syntax error at line 53, position 1254, token='nOp'
  - `k_pman_missing42`: error: CompileError: Syntax error at line 51, position 1254, token='nOp'
  - `k_pman_no_threat`: error: CompileError: Syntax error at line 71, position 1254, token='nOp'
  - `k_pman_notpaid_c`: error: CompileError: Syntax error at line 66, position 1254, token='nOp'
  - `k_pman_notpaid_n`: error: CompileError: Syntax error at line 68, position 1254, token='nOp'
  - `k_pman_notpaid_s`: error: CompileError: Syntax error at line 63, position 1254, token='nOp'
  - `k_pman_npcstart`: error: CompileError: Syntax error at line 46, position 1254, token='nOp'
  - `k_pman_npcstop`: error: CompileError: Syntax error at line 48, position 1254, token='nOp'
  - `k_pman_payrace_1`: error: CompileError: Syntax error at line 70, position 1254, token='nOp'
  - `k_pman_payrace_2`: error: CompileError: Syntax error at line 69, position 1254, token='nOp'
  - `k_pman_payrace_3`: error: CompileError: Syntax error at line 68, position 1254, token='nOp'
  - `k_pman_persuad_f`: error: CompileError: Syntax error at line 71, position 1254, token='nOp'
  - `k_pman_persuad_s`: error: CompileError: Syntax error at line 61, position 1254, token='nOp'
  - `k_pman_persuad_y`: error: CompileError: Syntax error at line 62, position 1254, token='nOp'
  - `k_pman_planet22`: error: CompileError: Syntax error at line 47, position 1254, token='nOp'
  - `k_pman_puzzle06`: error: CompileError: Syntax error at line 48, position 1254, token='nOp'
  - `k_pman_puzzle07`: error: CompileError: Syntax error at line 48, position 1254, token='nOp'
  - `k_pman_puzzle08`: error: CompileError: Syntax error at line 48, position 1254, token='nOp'
  - `k_pman_puzzle09`: error: CompileError: Syntax error at line 60, position 1254, token='nOp'
  - `k_pman_puzzle10`: error: CompileError: Syntax error at line 61, position 1254, token='nOp'
  - `k_pman_puzzle11`: error: CompileError: Syntax error at line 48, position 1254, token='nOp'
  - `k_pman_qreturn_n`: error: CompileError: Syntax error at line 68, position 1254, token='nOp'
  - `k_pman_qreturn_s`: error: CompileError: Syntax error at line 59, position 1254, token='nOp'
  - `k_pman_queecham2`: error: CompileError: Syntax error at line 63, position 1254, token='nOp'
  - `k_pman_queechamp`: error: CompileError: Syntax error at line 64, position 1254, token='nOp'
  - `k_pman_queelostp`: error: CompileError: Syntax error at line 65, position 1254, token='nOp'
  - `k_pman_queemad_s`: error: CompileError: Syntax error at line 54, position 1254, token='nOp'
  - `k_pman_queemad_y`: error: CompileError: Syntax error at line 61, position 1254, token='nOp'
  - `k_pman_queenotpc`: error: CompileError: Syntax error at line 64, position 1254, token='nOp'
  - `k_pman_queeorpcy`: error: CompileError: Syntax error at line 65, position 1254, token='nOp'
  - `k_pman_queequit`: error: CompileError: Syntax error at line 68, position 1254, token='nOp'
  - `k_pman_quename_n`: error: CompileError: Syntax error at line 63, position 1254, token='nOp'
  - `k_pman_quename_s`: error: CompileError: Syntax error at line 56, position 1254, token='nOp'
  - `k_pman_quepaid_n`: error: CompileError: Syntax error at line 67, position 1254, token='nOp'
  - `k_pman_quepaid_s`: error: CompileError: Syntax error at line 59, position 1254, token='nOp'
  - `k_pman_quepaid_y`: error: CompileError: Syntax error at line 68, position 1254, token='nOp'
  - `k_pman_quetell_n`: error: CompileError: Syntax error at line 67, position 1254, token='nOp'
  - `k_pman_quetell_s`: error: CompileError: Syntax error at line 57, position 1254, token='nOp'
  - `k_pman_quetell_y`: error: CompileError: Syntax error at line 68, position 1254, token='nOp'
  - `k_pman_quleave_s`: error: CompileError: Syntax error at line 61, position 1254, token='nOp'
  - `k_pman_quleave_y`: error: CompileError: Syntax error at line 69, position 1254, token='nOp'
  - `k_pman_race_0_02`: error: CompileError: Syntax error at line 66, position 1254, token='nOp'
  - `k_pman_race_0_03`: error: CompileError: Syntax error at line 65, position 1254, token='nOp'
  - `k_pman_race_0_04`: error: CompileError: Syntax error at line 64, position 1254, token='nOp'
  - `k_pman_race_0_q1`: error: CompileError: Syntax error at line 68, position 1254, token='nOp'
  - `k_pman_race_0_q2`: error: CompileError: Syntax error at line 69, position 1254, token='nOp'
  - `k_pman_race_0_q3`: error: CompileError: Syntax error at line 68, position 1254, token='nOp'
  - `k_pman_race_0_q4`: error: CompileError: Syntax error at line 67, position 1254, token='nOp'
  - `k_pman_race_1_02`: error: CompileError: Syntax error at line 62, position 1254, token='nOp'
  - `k_pman_race_1_03`: error: CompileError: Syntax error at line 66, position 1254, token='nOp'
  - `k_pman_race_1_04`: error: CompileError: Syntax error at line 65, position 1254, token='nOp'
  - `k_pman_race_2_02`: error: CompileError: Syntax error at line 68, position 1254, token='nOp'
  - `k_pman_race_2_03`: error: CompileError: Syntax error at line 67, position 1254, token='nOp'
  - `k_pman_race_2_04`: error: CompileError: Syntax error at line 66, position 1254, token='nOp'
  - `k_pman_race_3_02`: error: CompileError: Syntax error at line 65, position 1254, token='nOp'
  - `k_pman_race_3_03`: error: CompileError: Syntax error at line 67, position 1254, token='nOp'
  - `k_pman_race_3_04`: error: CompileError: Syntax error at line 66, position 1254, token='nOp'
  - `k_pman_race_s_01`: error: CompileError: Syntax error at line 67, position 1254, token='nOp'
  - `k_pman_race_s_02`: error: CompileError: Syntax error at line 68, position 1254, token='nOp'
  - `k_pman_race_s_03`: error: CompileError: Syntax error at line 67, position 1254, token='nOp'
  - `k_pman_race_s_04`: error: CompileError: Syntax error at line 66, position 1254, token='nOp'
  - `k_pman_race_x_02`: error: CompileError: Syntax error at line 72, position 1254, token='nOp'
  - `k_pman_race_x_03`: error: CompileError: Syntax error at line 71, position 1254, token='nOp'
  - `k_pman_race_x_04`: error: CompileError: Syntax error at line 70, position 1254, token='nOp'
  - `k_pman_racecha_n`: error: CompileError: Syntax error at line 62, position 1254, token='nOp'
  - `k_pman_racechamp`: error: CompileError: Syntax error at line 61, position 1254, token='nOp'
  - `k_pman_racelosty`: error: CompileError: Syntax error at line 67, position 1254, token='nOp'
  - `k_pman_racenone`: error: CompileError: Syntax error at line 61, position 1254, token='nOp'
  - `k_pman_raceone`: error: CompileError: Syntax error at line 61, position 1254, token='nOp'
  - `k_pman_raceonex`: error: CompileError: Syntax error at line 70, position 1254, token='nOp'
  - `k_pman_raceset_1`: error: CompileError: Syntax error at line 65, position 1254, token='nOp'
  - `k_pman_raceset_2`: error: CompileError: Syntax error at line 70, position 1254, token='nOp'
  - `k_pman_raceset_3`: error: CompileError: Syntax error at line 72, position 1254, token='nOp'
  - `k_pman_raceset_4`: error: CompileError: Syntax error at line 65, position 1254, token='nOp'
  - `k_pman_racetwo`: error: CompileError: Syntax error at line 61, position 1254, token='nOp'
  - `k_pman_racetwox`: error: CompileError: Syntax error at line 70, position 1254, token='nOp'
  - `k_pman_selapr5_d`: error: CompileError: Syntax error at line 64, position 1254, token='nOp'
  - `k_pman_shasa01`: error: CompileError: Syntax error at line 57, position 1254, token='nOp'
  - `k_pman_sith01`: error: CompileError: Syntax error at line 53, position 1254, token='nOp'
  - `k_pman_sith02`: error: CompileError: Syntax error at line 59, position 1254, token='nOp'
  - `k_pman_sithmas01`: error: CompileError: Syntax error at line 57, position 1254, token='nOp'
  - `k_pman_survive02`: error: CompileError: Syntax error at line 52, position 1254, token='nOp'
  - `k_pman_trial02`: error: CompileError: Syntax error at line 150, position 1254, token='nOp'
  - `k_pman_ww`: error: CompileError: Syntax error at line 46, position 1254, token='nOp'
  - `k_pman_ye_threat`: error: CompileError: Syntax error at line 61, position 1254, token='nOp'
  - `k_ptat_1gamewon`: error: CompileError: Syntax error at line 1847, position 51873, token='nOp'
  - `k_ptat_2gamewon`: error: CompileError: Syntax error at line 1847, position 51873, token='nOp'
  - `k_ptat_3gamewon`: error: CompileError: Syntax error at line 1848, position 51873, token='nOp'
  - `k_ptat_angrgurke`: error: CompileError: Syntax error at line 1832, position 51873, token='nOp'
  - `k_ptat_baddeal`: error: CompileError: Syntax error at line 1832, position 51873, token='nOp'
  - `k_ptat_bounty_n`: error: CompileError: Syntax error at line 1848, position 51873, token='nOp'
  - `k_ptat_bounty_y`: error: CompileError: Syntax error at line 1849, position 51873, token='nOp'
  - `k_ptat_bountyset`: error: CompileError: Syntax error at line 1841, position 51873, token='nOp'
  - `k_ptat_bullynico`: error: CompileError: Syntax error at line 1839, position 51873, token='nOp'
  - `k_ptat_celisdeal`: error: CompileError: Syntax error at line 1839, position 51873, token='nOp'
  - `k_ptat_checkmad`: error: CompileError: Syntax error at line 1846, position 51873, token='nOp'
  - `k_ptat_dockpaid`: error: CompileError: Syntax error at line 1840, position 51873, token='nOp'
  - `k_ptat_dockpay`: error: CompileError: Syntax error at line 1837, position 51873, token='nOp'
  - `k_ptat_dorakname`: error: CompileError: Syntax error at line 1833, position 51873, token='nOp'
  - `k_ptat_dorakwho`: error: CompileError: Syntax error at line 1840, position 51873, token='nOp'
  - `k_ptat_dunesea`: error: CompileError: Syntax error at line 1832, position 51873, token='nOp'
  - `k_ptat_gammbad`: error: CompileError: Syntax error at line 1840, position 51873, token='nOp'
  - `k_ptat_gammbribe`: error: CompileError: Syntax error at line 1840, position 51873, token='nOp'
  - `k_ptat_gammgone`: error: CompileError: Syntax error at line 1840, position 51873, token='nOp'
  - `k_ptat_gammhunt`: error: CompileError: Syntax error at line 1839, position 51873, token='nOp'
  - `k_ptat_gammleave`: error: CompileError: Syntax error at line 1852, position 51873, token='nOp'
  - `k_ptat_gammmean`: error: CompileError: Syntax error at line 1842, position 51873, token='nOp'
  - `k_ptat_gammwarn`: error: CompileError: Syntax error at line 1833, position 51873, token='nOp'
  - `k_ptat_gandmad`: error: CompileError: Syntax error at line 1839, position 51873, token='nOp'
  - `k_ptat_gandname`: error: CompileError: Syntax error at line 1841, position 51873, token='nOp'
  - `k_ptat_garmleave`: error: CompileError: Syntax error at line 1832, position 51873, token='nOp'
  - `k_ptat_garmname`: error: CompileError: Syntax error at line 1832, position 51873, token='nOp'
  - `k_ptat_garmwho`: error: CompileError: Syntax error at line 1839, position 51873, token='nOp'
  - `k_ptat_gooddeal`: error: CompileError: Syntax error at line 1832, position 51873, token='nOp'
  - `k_ptat_gurkemad`: error: CompileError: Syntax error at line 1839, position 51873, token='nOp'
  - `k_ptat_has_k_map`: error: CompileError: Syntax error at line 1840, position 51873, token='nOp'
  - `k_ptat_huntask_n`: error: CompileError: Syntax error at line 1848, position 51873, token='nOp'
  - `k_ptat_huntask_y`: error: CompileError: Syntax error at line 1847, position 51873, token='nOp'
  - `k_ptat_koname_ch`: error: CompileError: Syntax error at line 1847, position 51873, token='nOp'
  - `k_ptat_konameset`: error: CompileError: Syntax error at line 1840, position 51873, token='nOp'
  - `k_ptat_losscheck`: error: CompileError: Syntax error at line 1848, position 51873, token='nOp'
  - `k_ptat_m_office`: error: CompileError: Syntax error at line 1845, position 51873, token='nOp'
  - `k_ptat_namecheck`: error: CompileError: Syntax error at line 1846, position 51873, token='nOp'
  - `k_ptat_nicenico`: error: CompileError: Syntax error at line 1839, position 51873, token='nOp'
  - `k_ptat_nicofree`: error: CompileError: Syntax error at line 1840, position 51873, token='nOp'
  - `k_ptat_nicohappy`: error: CompileError: Syntax error at line 1832, position 51873, token='nOp'
  - `k_ptat_nicoinfo`: error: CompileError: Syntax error at line 1832, position 51873, token='nOp'
  - `k_ptat_niconews`: error: CompileError: Syntax error at line 1840, position 51873, token='nOp'
  - `k_ptat_nicotalk`: error: CompileError: Syntax error at line 1839, position 51873, token='nOp'
  - `k_ptat_nicowait`: error: CompileError: Syntax error at line 1841, position 51873, token='nOp'
  - `k_ptat_no_k_map`: error: CompileError: Syntax error at line 1840, position 51873, token='nOp'
  - `k_ptat_nohkinfo`: error: CompileError: Syntax error at line 1839, position 51873, token='nOp'
  - `k_ptat_nonicotlk`: error: CompileError: Syntax error at line 1839, position 51873, token='nOp'
  - `k_ptat_norespon`: error: CompileError: Syntax error at line 1833, position 51873, token='nOp'
  - `k_ptat_nosalehk`: error: CompileError: Syntax error at line 1839, position 51873, token='nOp'
  - `k_ptat_notracer`: error: CompileError: Syntax error at line 1840, position 51873, token='nOp'
  - `k_ptat_officemad`: error: CompileError: Syntax error at line 1843, position 51873, token='nOp'
  - `k_ptat_pazzloss`: error: CompileError: Syntax error at line 1840, position 51873, token='nOp'
  - `k_ptat_pazzwin1`: error: CompileError: Syntax error at line 1841, position 51873, token='nOp'
  - `k_ptat_pazzwin2`: error: CompileError: Syntax error at line 1841, position 51873, token='nOp'
  - `k_ptat_pazzwin3`: error: CompileError: Syntax error at line 1841, position 51873, token='nOp'
  - `k_ptat_persyuka`: error: CompileError: Syntax error at line 1832, position 51873, token='nOp'
  - `k_ptat_racechamp`: error: CompileError: Syntax error at line 1839, position 51873, token='nOp'
  - `k_ptat_racenone`: error: CompileError: Syntax error at line 1839, position 51873, token='nOp'
  - `k_ptat_raceone`: error: CompileError: Syntax error at line 1839, position 51873, token='nOp'
  - `k_ptat_racer`: error: CompileError: Syntax error at line 1840, position 51873, token='nOp'
  - `k_ptat_racer_id`: error: CompileError: Syntax error at line 1832, position 51873, token='nOp'
  - `k_ptat_racetrial`: error: CompileError: Syntax error at line 1839, position 51873, token='nOp'
  - `k_ptat_racetwo`: error: CompileError: Syntax error at line 1839, position 51873, token='nOp'
  - `k_ptat_salehk`: error: CompileError: Syntax error at line 1839, position 51873, token='nOp'
  - `k_ptat_sethunt`: error: CompileError: Syntax error at line 1839, position 51873, token='nOp'
  - `k_ptat_shariwait`: error: CompileError: Syntax error at line 1833, position 51873, token='nOp'
  - `k_ptat_shartrust`: error: CompileError: Syntax error at line 1839, position 51873, token='nOp'
  - `k_ptat_soldhk`: error: CompileError: Syntax error at line 1832, position 51873, token='nOp'
  - `k_ptat_talktanis`: error: CompileError: Syntax error at line 1834, position 51873, token='nOp'
  - `k_ptat_taname_ch`: error: CompileError: Syntax error at line 1848, position 51873, token='nOp'
  - `k_ptat_tanameset`: error: CompileError: Syntax error at line 1841, position 51873, token='nOp'
  - `k_ptat_tanileave`: error: CompileError: Syntax error at line 1837, position 51873, token='nOp'
  - `k_ptat_tanisdead`: error: CompileError: Syntax error at line 1842, position 51873, token='nOp'
  - `k_ptat_tanisgave`: error: CompileError: Syntax error at line 1840, position 51873, token='nOp'
  - `k_ptat_tanismad`: error: CompileError: Syntax error at line 1832, position 51873, token='nOp'
  - `k_ptat_tatinfo_c`: error: CompileError: Syntax error at line 1847, position 51873, token='nOp'
  - `k_ptat_tatinfo_s`: error: CompileError: Syntax error at line 1840, position 51873, token='nOp'
  - `k_ptat_toldofhk`: error: CompileError: Syntax error at line 1832, position 51873, token='nOp'
  - `k_ptat_yukaleave`: error: CompileError: Syntax error at line 1832, position 51873, token='nOp'
  - `k_ptat_yukanpers`: error: CompileError: Syntax error at line 1839, position 51873, token='nOp'
  - `k_ptat_yukarname`: error: CompileError: Syntax error at line 1833, position 51873, token='nOp'
  - `k_ptat_yukarwho`: error: CompileError: Syntax error at line 1840, position 51873, token='nOp'
  - `k_ptat_zoriiname`: error: CompileError: Syntax error at line 1833, position 51873, token='nOp'
  - `k_ptat_zoriiwho`: error: CompileError: Syntax error at line 1840, position 51873, token='nOp'
  - `k_ptat_zorileave`: error: CompileError: Syntax error at line 1831, position 51873, token='nOp'
  - `k_swg_hk47fix`: error: MissingIncludeError: Could not find included script 'K_inc_utility.nss'

Different output (1217), by cause:

- **stale-include** (71): BioWare's bytecode was compiled before an include it uses last changed: the first difference is in an include's globals or functions (k_inc_utility's SW_CONSTANT_* globals, k_inc_generic's GN_TalentMasterRoutine, among others)
  `k_act_atkonend`, `k_act_caloatt`, `k_act_com31`, `k_act_com32`, `k_act_com33`, `k_act_com33a`, `k_act_com34`, `k_act_com35`, `k_act_com41`, `k_act_com42`, `k_act_com43`, `k_act_com45`, `k_act_comp01`, `k_act_electric`, `k_act_startgas`, `k_act_xoratt`, `k_actq_com33`, `k_actq_com33a`, `k_actq_com34`, `k_amb_enemyflee`, `k_amb_predator01`, `k_com_compart`, `k_com_comrepr`, `k_con_com31`, `k_con_com32`, `k_con_com33`, `k_con_com34`, `k_con_com35`, `k_con_com41`, `k_con_com42`, `k_con_com43`, `k_con_com44`, `k_con_com45`, `k_con_comactive`, `k_con_comhack`, `k_con_comrep`, `k_con_comver31`, `k_con_comver32`, `k_con_comver33`, `k_con_comver33a`, `k_con_comver34`, `k_con_comver41`, `k_con_comver42`, `k_con_comver43`, `k_con_comver44`, `k_con_comver45`, `k_con_tokens`, `k_def_rependd`, `k_pla_actmap`, `k_plc_tresciv`, `k_plc_trescorhig`, `k_plc_trescorlow`, `k_plc_trescormid`, `k_plc_tresdrdhig`, `k_plc_tresdrdlow`, `k_plc_tresdrdmid`, `k_plc_tresmilhig`, `k_plc_tresmillow`, `k_plc_tresmilmid`, `k_plc_tresrakat`, `k_plc_tresshahig`, `k_plc_tresshalow`, `k_plc_tresshamid`, `k_plc_tressndppl`, `k_sp1_generic`, `k_sup_star_map`, `k_trg_messengers`, `k_trg_partyheart`, `k_trg_stealth`, `k_zon_catalog`, `k_zon_control`
- **stale-bytecode** (8): BioWare's bytecode was compiled from an earlier version of this script's own source: the first difference is in the script, and it is a different call, constant or condition
  `k_ptat_exitkrayt`, `k_qa`, `k_sup_droid`, `k_sup_gohawk`, `k_sup_grenade`, `k_trg_senni`, `k_trg_transfail1`, `k_trp_generic`
- **conversation-twelve** (1): BioWare compiled this against the shipped twelve-argument ActionStartConversation, unlike most of its scripts. Identical with the shipped declarations
  `k_ai_master`
- **conversation-eleven** (29): BioWare compiled this when ActionStartConversation took eleven arguments; the shipped declaration has twelve. Identical with --declarations bioware
  `k_bant_trig`, `k_def_bandspn`, `k_def_calodef`, `k_hjol_userdef01`, `k_pdan_elise_d`, `k_pdan_rapid01`, `k_pdan_rapid02`, `k_pdan_rapid03`, `k_pdan_rapid04`, `k_pend_door01`, `k_pend_door02`, `k_pend_door03`, `k_pend_door08`, `k_pkas_freyyuser`, `k_pman_door03`, `k_pman_door08`, `k_pman_door20`, `k_pman_judge1_d`, `k_pman_obj_asc`, `k_pman_obj_asc2`, `k_pman_obj_ascc`, `k_pman_obj_ascc2`, `k_pman_obj_ascn`, `k_pman_osc`, `k_pman_sitspeak`, `k_pman_survive01`, `k_ptar_sitharm`, `k_swg_xor_user`, `k_trg_partyinit`
- **no known cause** (1108): `k_act_bandatt`, `k_act_canderadd`, `k_act_candstim01`, `k_act_candstim02`, `k_act_candstim03`, `k_act_carthadd`, `k_act_com44`, `k_act_darkhigh`, `k_act_darkmed`, `k_act_darksml`, `k_act_exitarea`, `k_act_exitarea2`, `k_act_foeinsight`, `k_act_heartoff`, `k_act_hearton`, `k_act_hk47add`, `k_act_hk47simul`, `k_act_joleeadd`, `k_act_juhaniadd`, `k_act_leaverun`, `k_act_lighthigh`, `k_act_lightmed`, `k_act_lightsml`, `k_act_missionadd`, `k_act_nospeak`, `k_act_noswitch`, `k_act_npctalk`, `k_act_t3m3add`, `k_act_talktrue`, `k_act_walkways`, `k_act_walkways2`, `k_act_xordie`, `k_act_zaaladd`, `k_amb_prey_spawn`, `k_amb_prey_ude`, `k_cheat_01`, `k_cht_n_bast`, `k_cht_n_cand`, `k_cht_n_carth`, `k_cht_n_hk47`, `k_cht_n_jolee`, `k_cht_n_juhani`, `k_cht_n_mission`, `k_cht_n_t3m4`, `k_cht_n_zaalbar`, `k_cht_p_dan`, `k_cht_p_end`, `k_cht_p_kas`, `k_cht_p_kor`, `k_cht_p_lev`, `k_cht_p_man`, `k_cht_p_sta`, `k_cht_p_tar`, `k_cht_p_tat`, `k_cht_p_unk`, `k_cht_v_levi`, `k_cht_v_mast`, `k_cht_v_star`, `k_con_at_ebon`, `k_con_atdant`, `k_con_atforge`, `k_con_atkash`, `k_con_atkorr`, `k_con_atlev`, `k_con_atmann`, `k_con_atspire`, `k_con_attaris`, `k_con_attat`, `k_con_atunk`, `k_con_bastnpm`, `k_con_bastpm`, `k_con_candernpm`, `k_con_canderpm`, `k_con_candstim01`, `k_con_carthnpm`, `k_con_carthpm`, `k_con_chrhigh`, `k_con_chrlow`, `k_con_chrmiddle`, `k_con_chrnormal`, `k_con_classdiff`, `k_con_classsame`, `k_con_dark`, `k_con_darkhigh`, `k_con_darklow`, `k_con_flirt`, `k_con_flirtno`, `k_con_fpershigh`, `k_con_fperslow`, `k_con_heal`, `k_con_hk47npm`, `k_con_hk47pm`, `k_con_inrange`, `k_con_inthigh`, `k_con_intlow`, `k_con_intnormal`, `k_con_isfemale`, `k_con_isjedi`, `k_con_ismale`, `k_con_isscout`, `k_con_issmug`, `k_con_issoldier`, `k_con_joleejoin`, `k_con_joleenpm`, `k_con_joleepm`, `k_con_juhanilive`, `k_con_juhaninpm`, `k_con_juhanipm`, `k_con_light`, `k_con_lighthigh`, `k_con_lightlow`, `k_con_missionnpm`, `k_con_missionpm`, `k_con_neutral`, `k_con_npctalk`, `k_con_perseasy`, `k_con_pershigh`, `k_con_persmed`, `k_con_repair`, `k_con_t3m3npm`, `k_con_t3m3pm`, `k_con_talkedto`, `k_con_zaalnpm`, `k_con_zaalpm`, `k_def_ambient`, `k_def_ambmob`, `k_def_attacked01`, `k_def_blocked01`, `k_def_buff`, `k_def_calospn`, `k_def_combend01`, `k_def_damage01`, `k_def_death01`, `k_def_dialogue01`, `k_def_disturb01`, `k_def_endconv`, `k_def_grenspn`, `k_def_heartbt01`, `k_def_pathfail01`, `k_def_percept01`, `k_def_repairsp`, `k_def_repuser`, `k_def_spawn01`, `k_def_spellat01`, `k_def_userdef01`, `k_hbanter1`, `k_hbanter10`, `k_hbanter10x`, `k_hbanter11`, `k_hbanter11x`, `k_hbanter1x`, `k_hbanter2`, `k_hbanter2x`, `k_hbanter3`, `k_hbanter3x`, `k_hbanter4`, `k_hbanter4x`, `k_hbanter5`, `k_hbanter5x`, `k_hbanter6`, `k_hbanter6x`, `k_hbanter7`, `k_hbanter7x`, `k_hbanter8`, `k_hbanter8x`, `k_hbanter9`, `k_hbanter9x`, `k_hbas_final`, `k_hbas_playkash`, `k_hbas_playkorr`, `k_hbas_playman`, `k_hbas_playtat`, `k_hcan_state01`, `k_hcan_state02`, `k_hcan_state03`, `k_hcan_state04`, `k_hcan_state05`, `k_hcan_state06`, `k_hcan_state07`, `k_hcan_state08`, `k_hcan_state09`, `k_hcan_state10`, `k_hcan_state20`, `k_hcan_state21`, `k_hcan_state22`, `k_hcan_state23`, `k_hcan_state28`, `k_hcan_state29`, `k_hcan_state30`, `k_hcan_state31`, `k_hcan_state32`, `k_hcan_state33`, `k_hcan_state34`, `k_hcan_state35`, `k_hcan_state36`, `k_hcan_state37`, `k_hcan_state38`, `k_hcan_state39`, `k_hcan_state40`, `k_hcan_state41`, `k_hcan_state42`, `k_hcan_state43`, `k_hen_attacked01`, `k_hen_blocked01`, `k_hen_candspwn`, `k_hen_canduser`, `k_hen_combend01`, `k_hen_damage01`, `k_hen_dialogue01`, `k_hen_enter5m`, `k_hen_exit5m`, `k_hen_heartbt01`, `k_hen_leadchng`, `k_hen_percept01`, `k_hen_retreat`, `k_hen_spawn01`, `k_hjol_spawn01`, `k_hjuh_d01`, `k_hjuh_d02`, `k_hjuh_h01`, `k_hjuh_h01x`, `k_hjuh_h02`, `k_hjuh_h03`, `k_hjuh_h04`, `k_hjuh_h05`, `k_hjuh_h06`, `k_hjuh_h07`, `k_hjuh_h12`, `k_hjuh_p01`, `k_hjuh_p02`, `k_hjuh_p03`, `k_hjuh_p04`, `k_hjuh_p05`, `k_hjuh_p06`, `k_hjuh_p07`, `k_hjuh_p07d`, `k_hjuh_p07l`, `k_hjuh_p08`, `k_hjuh_p09`, `k_hjuh_p10`, `k_hjuh_p10x`, `k_hjuh_p11`, `k_hjuh_p12`, `k_hjuh_p13`, `k_hjuh_p15`, `k_hjuh_p16`, `k_hjuh_p19`, `k_hjuh_p20`, `k_hjuh_p21`, `k_hjuh_p22`, `k_hjuh_p23`, `k_hjuh_p25`, `k_hjuh_p26`, `k_hjuh_p27`, `k_hjuh_p28`, `k_hjuh_s04`, `k_hjuh_s05`, `k_hjuh_s06`, `k_hjuh_w01`, `k_hjuh_w02`, `k_hjuh_w03`, `k_hjuh_w04`, `k_hjuh_w05`, `k_hjuh_w06`, `k_hjuh_w07`, `k_hjuh_w08`, `k_hjuh_w09`, `k_hjuh_w10`, `k_hmis_misangry`, `k_hmis_misangryx`, `k_hmis_paz`, `k_hmis_planet`, `k_hmis_talk0`, `k_hmis_talk0+`, `k_hmis_talk1`, `k_hmis_talk10`, `k_hmis_talk10lev`, `k_hmis_talk10x`, `k_hmis_talk11`, `k_hmis_talk11lev`, `k_hmis_talk11x`, `k_hmis_talk12`, `k_hmis_talk12x`, `k_hmis_talk13x`, `k_hmis_talk14`, `k_hmis_talk14x`, `k_hmis_talk15`, `k_hmis_talk15x`, `k_hmis_talk1lev`, `k_hmis_talk1x`, `k_hmis_talk2x`, `k_hmis_talk3`, `k_hmis_talk3lev`, `k_hmis_talk3x`, `k_hmis_talk4x`, `k_hmis_talk5`, `k_hmis_talk5x`, `k_hmis_talk6`, `k_hmis_talk6-8`, `k_hmis_talk6lev`, `k_hmis_talk6x`, `k_hmis_talk7`, `k_hmis_talk7lev`, `k_hmis_talk7x`, `k_hmis_talk8`, `k_hmis_talk8x`, `k_hmis_talk9`, `k_hmis_talk99`, `k_hmis_talk99x`, `k_hmis_talk9x`, `k_hmis_talkrev`, `k_hmis_tardest`, `k_hmis_tardestx`, `k_hmis_zaalbar`, `k_hxor_state01`, `k_hxor_state02`, `k_hxor_state03`, `k_hzal_atkash1`, `k_hzal_atkash2`, `k_hzal_beenkashy`, `k_hzal_chuundead`, `k_hzal_freyydead`, `k_hzal_notkash1`, `k_hzal_notkash2`, `k_hzal_talk0`, `k_hzal_talk_1up`, `k_hzal_talkmad_c`, `k_hzal_talkmad_s`, `k_hzal_talkset_1`, `k_messenger_plot`, `k_pdan_anger06`, `k_pdan_anger07`, `k_pdan_anger08`, `k_pdan_anger09`, `k_pdan_anger10`, `k_pdan_anger11`, `k_pdan_anger12`, `k_pdan_anger13`, `k_pdan_anger14`, `k_pdan_anger15`, `k_pdan_anger16`, `k_pdan_anger17`, `k_pdan_anger18`, `k_pdan_anger19`, `k_pdan_anger20`, `k_pdan_bastial05`, `k_pdan_bastila01`, `k_pdan_bastila02`, `k_pdan_bastila03`, `k_pdan_bastila04`, `k_pdan_bastila05`, `k_pdan_bastila06`, `k_pdan_bastila07`, `k_pdan_bastila08`, `k_pdan_bastila09`, `k_pdan_bastila10`, `k_pdan_bastila11`, `k_pdan_belaya01`, `k_pdan_belaya02`, `k_pdan_cander01`, `k_pdan_cander02`, `k_pdan_cander03`, `k_pdan_carth01`, `k_pdan_carth02`, `k_pdan_carth03`, `k_pdan_carth04`, `k_pdan_casus01`, `k_pdan_casus02`, `k_pdan_casus03`, `k_pdan_casus04`, `k_pdan_casus05`, `k_pdan_drandom02`, `k_pdan_drandom03`, `k_pdan_drandom04`, `k_pdan_drandom05`, `k_pdan_drandom06`, `k_pdan_drandom07`, `k_pdan_drandom08`, `k_pdan_droid01`, `k_pdan_droid02`, `k_pdan_droid05`, `k_pdan_droid06`, `k_pdan_droid07`, `k_pdan_droid08`, `k_pdan_droid09`, `k_pdan_droid11`, `k_pdan_droid12`, `k_pdan_droid13`, `k_pdan_droid15`, `k_pdan_droid50`, `k_pdan_iriaz_sp`, `k_pdan_iriaz_ud`, `k_pdan_jedi01`, `k_pdan_jedi02`, `k_pdan_jedi03`, `k_pdan_jedi04`, `k_pdan_jedi05`, `k_pdan_jedi06`, `k_pdan_jedi07`, `k_pdan_jedi08`, `k_pdan_jedi09`, `k_pdan_jedi10`, `k_pdan_jedi11`, `k_pdan_jedi12`, `k_pdan_jedi13`, `k_pdan_jedi14`, `k_pdan_jedi15`, `k_pdan_jedi16`, `k_pdan_jedi17`, `k_pdan_jedi18`, `k_pdan_jedi19`, `k_pdan_jedi20`, `k_pdan_jedi21`, `k_pdan_jedi22`, `k_pdan_jedi23`, `k_pdan_jedi24`, `k_pdan_jedi25`, `k_pdan_jedi26`, `k_pdan_jedi27`, `k_pdan_jedi28`, `k_pdan_jedi29`, `k_pdan_jedi30`, `k_pdan_jedi31`, `k_pdan_jedi32`, `k_pdan_jedi33`, `k_pdan_jedi34`, `k_pdan_jedi35`, `k_pdan_jedi36`, `k_pdan_jedi37`, `k_pdan_jedi38`, `k_pdan_jedi39`, `k_pdan_juhani01`, `k_pdan_juhani02`, `k_pdan_juhani03`, `k_pdan_juhani04`, `k_pdan_juhani05`, `k_pdan_juhani06`, `k_pdan_juhani07`, `k_pdan_juhani08`, `k_pdan_juhani09`, `k_pdan_juhani10`, `k_pdan_juhani11`, `k_pdan_juhani12`, `k_pdan_juhani15`, `k_pdan_juhani16`, `k_pdan_juhani17`, `k_pdan_juhani18`, `k_pdan_juhani19`, `k_pdan_juhani20`, `k_pdan_kath02`, `k_pdan_kath03`, `k_pdan_kath04`, `k_pdan_mand01`, `k_pdan_mand02`, `k_pdan_mand04`, `k_pdan_mand06`, `k_pdan_mand07`, `k_pdan_mand08`, `k_pdan_mand10`, `k_pdan_mand11`, `k_pdan_mand52`, `k_pdan_mission01`, `k_pdan_mission02`, `k_pdan_murder01`, `k_pdan_murder02`, `k_pdan_murder03`, `k_pdan_murder04`, `k_pdan_murder12`, `k_pdan_murder13`, `k_pdan_murder14`, `k_pdan_murder15`, `k_pdan_murder16`, `k_pdan_murder17`, `k_pdan_murder18`, `k_pdan_murder19`, `k_pdan_murder20`, `k_pdan_murder21`, `k_pdan_murder22`, `k_pdan_murder23`, `k_pdan_murder24`, `k_pdan_murder25`, `k_pdan_murder26`, `k_pdan_murder27`, `k_pdan_murder28`, `k_pdan_murder29`, `k_pdan_murder36`, `k_pdan_murder37`, `k_pdan_murder38`, `k_pdan_murder39`, `k_pdan_murder40`, `k_pdan_murder41`, `k_pdan_murder42`, `k_pdan_murder43`, `k_pdan_murder44`, `k_pdan_murder45`, `k_pdan_murder46`, `k_pdan_murder53`, `k_pdan_murder55`, `k_pdan_murder56`, `k_pdan_murder57`, `k_pdan_murder58`, `k_pdan_murder59`, `k_pdan_murder60`, `k_pdan_murder61`, `k_pdan_murder62`, `k_pdan_murder64`, `k_pdan_murder65`, `k_pdan_murder66`, `k_pdan_murder67`, `k_pdan_murder68`, `k_pdan_murder69`, `k_pdan_murder70`, `k_pdan_murder71`, `k_pdan_murder76`, `k_pdan_planet01`, `k_pdan_planet02`, `k_pdan_planet03`, `k_pdan_planet04`, `k_pdan_planet05`, `k_pdan_planet06`, `k_pdan_planet08`, `k_pdan_planet12`, `k_pdan_planet14`, `k_pdan_planet15`, `k_pdan_planet16`, `k_pdan_romance01`, `k_pdan_romance02`, `k_pdan_romance03`, `k_pdan_romance04`, `k_pdan_romance05`, `k_pdan_romance06`, `k_pdan_romance07`, `k_pdan_romance08`, `k_pdan_romance09`, `k_pdan_romance10`, `k_pdan_romance11`, `k_pdan_romance12`, `k_pdan_romance13`, `k_pdan_romance14`, `k_pdan_romance15`, `k_pdan_romance16`, `k_pdan_romance17`, `k_pdan_romance18`, `k_pdan_romance19`, `k_pdan_romance20`, `k_pdan_romance21`, `k_pdan_romance22`, `k_pdan_romance23`, `k_pdan_romance24`, `k_pdan_romance25`, `k_pdan_romance26`, `k_pdan_romance27`, `k_pdan_romance29`, `k_pdan_romance30`, `k_pdan_romance31`, `k_pdan_romance32`, `k_pdan_romance33`, `k_pdan_romance34`, `k_pdan_romance36`, `k_pdan_romance37`, `k_pdan_romance38`, `k_pdan_romance39`, `k_pdan_romance40`, `k_pdan_romance41`, `k_pdan_stand01`, `k_pdan_stand02`, `k_pebn_ctaris01`, `k_pebn_ctaris02`, `k_pebn_ctaris03`, `k_pebn_ctaris04`, `k_pebn_ctaris05`, `k_pebn_ctaris06`, `k_pebn_ctaris07`, `k_pebn_ctaris08`, `k_pebn_ctaris09`, `k_pebn_gotaris`, `k_pebn_loc01`, `k_pebn_loc02`, `k_pebn_loc03`, `k_pebn_loc04`, `k_pebn_loc05`, `k_pebn_loc06`, `k_pebn_loc07`, `k_pebn_loc08`, `k_pebn_loc09`, `k_pebn_swoop`, `k_pend_door05`, `k_pend_door06`, `k_pend_door09`, `k_pend_explode`, `k_phkd_fighter`, `k_pkas_chuundead`, `k_pkas_freyydead`, `k_pkas_freyyhelp`, `k_pkas_freyyspwn`, `k_pkas_hashilt`, `k_pkas_hassword`, `k_pkas_heart_on`, `k_pkas_jaarakblt`, `k_pkas_jaaraksus`, `k_pkas_jaardead`, `k_pkas_jaarfreed`, `k_pkas_nohilt`, `k_pkas_nosword`, `k_pkas_randwalk`, `k_pman_28c_sur01`, `k_pman_airlock01`, `k_pman_amb_9`, `k_pman_arg01`, `k_pman_argrep_d`, `k_pman_argsith_d`, `k_pman_beast_d`, `k_pman_comp01`, `k_pman_comp06`, `k_pman_comp07`, `k_pman_comp34`, `k_pman_comp35`, `k_pman_door32`, `k_pman_false`, `k_pman_locked01`, `k_pman_locked02`, `k_pman_main01`, `k_pman_misc01`, `k_pman_misc02`, `k_pman_misc03`, `k_pman_misc04`, `k_pman_misc05`, `k_pman_misc06`, `k_pman_misc08`, `k_pman_misc09`, `k_pman_misc10`, `k_pman_misc11`, `k_pman_misc12`, `k_pman_misc13`, `k_pman_misc14`, `k_pman_misc15`, `k_pman_misc16`, `k_pman_misc17`, `k_pman_misc18`, `k_pman_misc19`, `k_pman_misc24`, `k_pman_misc25`, `k_pman_misc28`, `k_pman_misc30`, `k_pman_misc32`, `k_pman_misc35`, `k_pman_misc38`, `k_pman_misc39`, `k_pman_misc41`, `k_pman_misc42`, `k_pman_misc46`, `k_pman_misc47`, `k_pman_misc53`, `k_pman_misc55`, `k_pman_misc59`, `k_pman_misc60`, `k_pman_misc61`, `k_pman_misc62`, `k_pman_misc70`, `k_pman_missing01`, `k_pman_missing02`, `k_pman_missing03`, `k_pman_missing04`, `k_pman_missing05`, `k_pman_missing06`, `k_pman_missing07`, `k_pman_missing08`, `k_pman_missing09`, `k_pman_missing10`, `k_pman_missing11`, `k_pman_missing12`, `k_pman_missing13`, `k_pman_missing15`, `k_pman_missing16`, `k_pman_missing23`, `k_pman_missing27`, `k_pman_missing28`, `k_pman_missing30`, `k_pman_missing31`, `k_pman_missing33`, `k_pman_missing34`, `k_pman_missing37`, `k_pman_missing38`, `k_pman_missing43`, `k_pman_murder01`, `k_pman_murder02`, `k_pman_murder03`, `k_pman_murder04`, `k_pman_murder05`, `k_pman_murder06`, `k_pman_murder07`, `k_pman_murder08`, `k_pman_murder09`, `k_pman_murder10`, `k_pman_murder11`, `k_pman_murder12`, `k_pman_murder13`, `k_pman_murder14`, `k_pman_murder15`, `k_pman_murder16`, `k_pman_murder17`, `k_pman_murder18`, `k_pman_murder19`, `k_pman_murder20`, `k_pman_murder21`, `k_pman_murder22`, `k_pman_murder23`, `k_pman_murder24`, `k_pman_murder25`, `k_pman_murder26`, `k_pman_murder27`, `k_pman_murder28`, `k_pman_murder29`, `k_pman_murder31`, `k_pman_murder33`, `k_pman_murder34`, `k_pman_murder35`, `k_pman_murder37`, `k_pman_murder38`, `k_pman_murder40`, `k_pman_murder42`, `k_pman_murder43`, `k_pman_murder44`, `k_pman_murder45`, `k_pman_murder46`, `k_pman_murder47`, `k_pman_murder50`, `k_pman_murder55`, `k_pman_murder56`, `k_pman_murder57`, `k_pman_murder61`, `k_pman_murder64`, `k_pman_murder65`, `k_pman_murder67`, `k_pman_murder68`, `k_pman_murder69`, `k_pman_murder71`, `k_pman_murder75`, `k_pman_murder76`, `k_pman_murder77`, `k_pman_murder78`, `k_pman_murder79`, `k_pman_murder80`, `k_pman_murder95`, `k_pman_murder97`, `k_pman_murder98`, `k_pman_murder99`, `k_pman_murder9a`, `k_pman_murder9b`, `k_pman_murder9c`, `k_pman_murder9h`, `k_pman_murder9i`, `k_pman_murder9j`, `k_pman_murder9k`, `k_pman_murder9l`, `k_pman_murder9m`, `k_pman_murder9n`, `k_pman_murder9o`, `k_pman_murder9p`, `k_pman_murder9q`, `k_pman_murder9r`, `k_pman_murder9u`, `k_pman_murder9v`, `k_pman_murder9w`, `k_pman_murder9x`, `k_pman_murder9y`, `k_pman_murder9z`, `k_pman_murdera07`, `k_pman_murdera08`, `k_pman_murdera09`, `k_pman_murdera11`, `k_pman_patrol_d`, `k_pman_pazaak01`, `k_pman_pazaak02`, `k_pman_pazaak03`, `k_pman_pazaak04`, `k_pman_pazaak05`, `k_pman_pazaak06`, `k_pman_pazaak07`, `k_pman_pazaak08`, `k_pman_pazaak09`, `k_pman_pazaak10`, `k_pman_pazaak11`, `k_pman_planet01`, `k_pman_planet02`, `k_pman_planet03`, `k_pman_planet04`, `k_pman_planet05`, `k_pman_planet06`, `k_pman_planet07`, `k_pman_planet08`, `k_pman_planet09`, `k_pman_planet10`, `k_pman_planet11`, `k_pman_planet12`, `k_pman_planet13`, `k_pman_planet14`, `k_pman_planet15`, `k_pman_planet16`, `k_pman_planet17`, `k_pman_planet18`, `k_pman_planet19`, `k_pman_planet20`, `k_pman_planet21`, `k_pman_planet24`, `k_pman_planet27`, `k_pman_planet28`, `k_pman_planet29`, `k_pman_planet31`, `k_pman_planet32`, `k_pman_planet33`, `k_pman_planet34`, `k_pman_planet36`, `k_pman_planet37`, `k_pman_planet38`, `k_pman_planet44`, `k_pman_planet45`, `k_pman_planet46`, `k_pman_planet48`, `k_pman_planet49`, `k_pman_planet50`, `k_pman_planet51`, `k_pman_planet57`, `k_pman_planet58`, `k_pman_planet59`, `k_pman_planet60`, `k_pman_planet61`, `k_pman_planet63`, `k_pman_planet64`, `k_pman_planet65`, `k_pman_planet66`, `k_pman_planet67`, `k_pman_planet68`, `k_pman_planet69`, `k_pman_planet70`, `k_pman_planet71`, `k_pman_planet72`, `k_pman_planet73`, `k_pman_planet83`, `k_pman_planet84`, `k_pman_planet85`, `k_pman_planet87`, `k_pman_puzzle01`, `k_pman_puzzle02`, `k_pman_puzzle03`, `k_pman_puzzle04`, `k_pman_puzzle05`, `k_pman_puzzle12`, `k_pman_puzzle13`, `k_pman_puzzle14`, `k_pman_random01`, `k_pman_random02`, `k_pman_random03`, `k_pman_random04`, `k_pman_random05`, `k_pman_random06`, `k_pman_random07`, `k_pman_random08`, `k_pman_random09`, `k_pman_random10`, `k_pman_random11`, `k_pman_random12`, `k_pman_random13`, `k_pman_random15`, `k_pman_random16`, `k_pman_random17`, `k_pman_random18`, `k_pman_random19`, `k_pman_random20`, `k_pman_random21`, `k_pman_random22`, `k_pman_random23`, `k_pman_random24`, `k_pman_random25`, `k_pman_random26`, `k_pman_random27`, `k_pman_random28`, `k_pman_random29`, `k_pman_random30`, `k_pman_random31`, `k_pman_random32`, `k_pman_random33`, `k_pman_rodian01`, `k_pman_rodian02`, `k_pman_rodian03`, `k_pman_rodian04`, `k_pman_rodian06`, `k_pman_rodian07`, `k_pman_selapr_d`, `k_pman_shield_9`, `k_pman_shield_d`, `k_pman_sonic`, `k_pman_suituse`, `k_pman_swoop01`, `k_pman_swoop02`, `k_pman_swoop03`, `k_pman_swoop04`, `k_pman_swoop05`, `k_pman_swoop06`, `k_pman_swoop07`, `k_pman_swoop08`, `k_pman_swoop09`, `k_pman_swoop10`, `k_pman_swoop11`, `k_pman_swoop12`, `k_pman_swoop13`, `k_pman_swoop14`, `k_pman_swoop15`, `k_pman_swoop16`, `k_pman_swoop17`, `k_pman_swoop18`, `k_pman_swoop19`, `k_pman_swoop20`, `k_pman_swoop21`, `k_pman_swoop22`, `k_pman_swoop23`, `k_pman_swoop24`, `k_pman_swoop25`, `k_pman_swoop26`, `k_pman_swoop27`, `k_pman_swoop28`, `k_pman_swoop29`, `k_pman_swoop30`, `k_pman_swoop31`, `k_pman_swoop34`, `k_pman_swoop35`, `k_pman_swoop36`, `k_pman_swoop37`, `k_pman_swoop38`, `k_pman_swoop39`, `k_pman_swoop40`, `k_pman_tatracern`, `k_pman_tatracery`, `k_ptar_sithdis`, `k_ptar_sithpaper`, `k_ptat_amb1`, `k_ptat_beenkash`, `k_ptat_drandom09`, `k_ptat_drandom10`, `k_ptat_drandom11`, `k_ptat_drandom12`, `k_ptat_gaffiwrit`, `k_ptat_gateperm`, `k_ptat_gizkaspwn`, `k_ptat_gizkauser`, `k_ptat_hasgaffi`, `k_ptat_haspearl`, `k_ptat_haspermit`, `k_ptat_haswrit`, `k_ptat_nopermit`, `k_ptat_nowrit`, `k_ptat_sandper`, `k_ptat_soldpearl`, `k_ptat_tuskpdone`, `k_punk_cold`, `k_ren_leavlev`, `k_ren_levcapture`, `k_ren_levescape`, `k_ren_nextplanet`, `k_ren_starforge`, `k_ren_starland`, `k_ren_taris01`, `k_ren_taris02`, `k_ren_taris03`, `k_ren_turret`, `k_ren_turretld02`, `k_ren_turretload`, `k_ren_unkcrash`, `k_ren_unkturret`, `k_ren_visionland`, `k_sup_actmap`, `k_sup_bands`, `k_sup_comshots`, `k_sup_fear`, `k_sup_guiopen`, `k_sup_healing`, `k_sup_static`, `k_swg_bastdef`, `k_swg_bastevil`, `k_swg_bastforce1`, `k_swg_bastforce3`, `k_swg_bastforcex`, `k_swg_bastforcey`, `k_swg_bastila01`, `k_swg_bastila02`, `k_swg_bastila03`, `k_swg_bastila04`, `k_swg_bastila05`, `k_swg_bastila06`, `k_swg_bastila07`, `k_swg_bastila08`, `k_swg_bastila09`, `k_swg_bastila10`, `k_swg_bastila11`, `k_swg_bastila12`, `k_swg_bastila12a`, `k_swg_bastila13`, `k_swg_bastila14`, `k_swg_bastila15`, `k_swg_bastila16`, `k_swg_bastila17`, `k_swg_bastila18`, `k_swg_bastila19`, `k_swg_bastila20`, `k_swg_bastila20x`, `k_swg_bastila21`, `k_swg_bastila22`, `k_swg_bastila23`, `k_swg_bastila24`, `k_swg_bastila25`, `k_swg_bastila26`, `k_swg_bastila27`, `k_swg_bastila28`, `k_swg_bastila29`, `k_swg_bastila30`, `k_swg_bastila31`, `k_swg_bastila32`, `k_swg_bastila99`, `k_swg_bastile17`, `k_swg_bastmovie`, `k_swg_carth01`, `k_swg_carth02`, `k_swg_carth03`, `k_swg_carth04`, `k_swg_carth05`, `k_swg_carth06`, `k_swg_carth07`, `k_swg_carth08`, `k_swg_carth09`, `k_swg_carth10`, `k_swg_carth11`, `k_swg_carth12`, `k_swg_carth13`, `k_swg_carth14`, `k_swg_carth15`, `k_swg_carth16`, `k_swg_carth17`, `k_swg_carth18`, `k_swg_carth19`, `k_swg_carth20`, `k_swg_carth21`, `k_swg_carth22`, `k_swg_carth23`, `k_swg_carth24`, `k_swg_carth25`, `k_swg_carth25x`, `k_swg_carth26`, `k_swg_carth99`, `k_swg_helena07`, `k_swg_helena08`, `k_swg_hk4700`, `k_swg_hk4701`, `k_swg_hk4702`, `k_swg_hk4703`, `k_swg_hk4704`, `k_swg_hk4705`, `k_swg_hk4706`, `k_swg_hk4707`, `k_swg_hk4708`, `k_swg_hk4709`, `k_swg_hk4710`, `k_swg_hk4711`, `k_swg_hk4712`, `k_swg_hk4712x`, `k_swg_hk4712y`, `k_swg_hk4713`, `k_swg_hk47upg01`, `k_swg_hk47upg02`, `k_swg_hk47upg03`, `k_swg_hk47upg04`, `k_swg_jolee01`, `k_swg_jolee02`, `k_swg_jolee03`, `k_swg_jolee04`, `k_swg_jolee05`, `k_swg_jolee06`, `k_swg_jolee07`, `k_swg_jolee08`, `k_swg_jolee09`, `k_swg_jolee10`, `k_swg_jolee11`, `k_swg_jolee12`, `k_swg_jolee13`, `k_swg_jolee14`, `k_swg_jolee15`, `k_swg_jolee16`, `k_swg_jolee17`, `k_swg_jolee18`, `k_swg_jolee19`, `k_swg_jolee20`, `k_swg_jolee99`, `k_swg_joleerev`, `k_swg_replaykas`, `k_swg_replaykor`, `k_swg_replayman`, `k_swg_replaytat`, `k_swg_sasha01`, `k_swg_xor_def`, `k_trg_bandon`, `k_trg_calonord`, `k_trg_xorattack`, `k_unk_rancor`, `qa_calo`, `qa_lev`, `qa_map`, `qa_master`, `qa_planet`, `qa_setanim`

Size differences, largest clusters first:

| Bytes | Scripts | Examples |
| --- | --- | --- |
| -6 | 342 | `k_hbas_final`, `k_hcan_state01`, `k_hcan_state20` |
| +277 | 129 | `k_con_atdant`, `k_con_atforge`, `k_con_atkash` |
| +283 | 116 | `k_act_canderadd`, `k_act_candstim01`, `k_act_candstim02` |
| +263 | 94 | `k_con_bastpm`, `k_con_canderpm`, `k_con_carthpm` |
| -20 | 84 | `k_hcan_state02`, `k_hcan_state03`, `k_hcan_state04` |
| +249 | 36 | `k_hbanter1`, `k_hbanter10`, `k_hbanter11` |
| -34 | 31 | `k_hcan_state07`, `k_hcan_state08`, `k_hcan_state09` |
| +267 | 19 | `k_def_attacked01`, `k_def_blocked01`, `k_def_combend01` |
| +6 | 15 | `k_pend_door03`, `k_pend_door08`, `k_pman_door03` |
| +23759 | 14 | `k_plc_tresciv`, `k_plc_trescorhig`, `k_plc_trescorlow` |
| +28404 | 13 | `k_pebn_loc01`, `k_pebn_loc02`, `k_pebn_loc03` |
| +73413 | 13 | `k_amb_prey_spawn`, `k_def_ambient`, `k_def_ambmob` |
| +271 | 12 | `k_con_at_ebon`, `k_con_inrange`, `k_pebn_ctaris01` |
| +0 | 11 | `k_pdan_stand01`, `k_pkas_hashilt`, `k_pkas_hassword` |
| +28837 | 11 | `k_hbanter10x`, `k_hbanter11x`, `k_hbanter1x` |
| -48 | 10 | `k_hjuh_p05`, `k_hjuh_p22`, `k_pman_murder09` |
| +800 | 10 | `k_cht_p_dan`, `k_cht_p_end`, `k_cht_p_kas` |
| -90 | 9 | `k_pman_planet64`, `k_pman_planet65`, `k_pman_planet66` |
| +881 | 9 | `k_cht_n_bast`, `k_cht_n_cand`, `k_cht_n_carth` |
| +28482 | 9 | `k_con_com32`, `k_con_com33`, `k_con_com34` |
| +29002 | 9 | `k_con_comactive`, `k_con_comrep`, `k_con_comver31` |
| +28178 | 6 | `k_act_darkhigh`, `k_act_darkmed`, `k_act_darksml` |
| -202 | 5 | `k_pman_murder97`, `k_pman_murder98`, `k_pman_murder99` |
| -12 | 5 | `k_hzal_beenkashy`, `k_hzal_chuundead`, `k_hzal_freyydead` |
| -216 | 4 | `k_pman_murder95`, `k_pman_murder9b`, `k_pman_murder9c` |
| +219 | 4 | `k_cht_v_levi`, `k_cht_v_mast`, `k_cht_v_star` |
| +235 | 4 | `k_hzal_atkash2`, `k_hzal_notkash2`, `k_swg_bastila02` |
| +27755 | 4 | `k_act_com32`, `k_act_com33`, `k_act_com33a` |
| +28996 | 4 | `k_con_chrhigh`, `k_con_chrnormal`, `k_con_inthigh` |
| +111262 | 4 | `k_pdan_rapid01`, `k_pdan_rapid02`, `k_pdan_rapid03` |
| +114814 | 4 | `k_pdan_droid06`, `k_pdan_droid07`, `k_pdan_droid08` |
| +243 | 3 | `k_con_flirtno`, `k_con_fperslow`, `k_swg_carth25x` |
| +8421 | 3 | `k_ren_turretld02`, `k_ren_turretload`, `k_ren_unkturret` |
| +27615 | 3 | `k_con_perseasy`, `k_con_pershigh`, `k_con_persmed` |
| +27637 | 3 | `k_pman_planet57`, `k_pman_planet58`, `k_pman_planet59` |
| +28940 | 3 | `k_con_darkhigh`, `k_con_darklow`, `k_con_lightlow` |
| +114853 | 3 | `k_pdan_bastila11`, `k_pman_28c_sur01`, `k_pman_arg01` |
| +124273 | 3 | `k_pkas_freyydead`, `k_pkas_jaardead`, `k_pkas_jaarfreed` |
| -28 | 2 | `k_pman_suituse`, `k_ptat_gateperm` |
| +12 | 2 | `k_pman_argrep_d`, `k_pman_argsith_d` |
| +255 | 2 | `k_pman_sonic`, `k_ptat_sandper` |
| +257 | 2 | `k_hmis_paz`, `k_phkd_fighter` |
| +269 | 2 | `k_swg_bastila28`, `k_unk_rancor` |
| +8419 | 2 | `k_ren_starforge`, `k_ren_taris01` |
| +11199 | 2 | `k_act_atkonend`, `k_act_xoratt` |
| +27538 | 2 | `k_actq_com33`, `k_actq_com33a` |
| +27758 | 2 | `k_act_com42`, `k_act_com43` |
| +28695 | 2 | `k_act_exitarea`, `k_act_exitarea2` |
| +28852 | 2 | `k_swg_hk4712`, `k_swg_jolee14` |
| +28958 | 2 | `k_con_chrlow`, `k_con_intlow` |
| +28974 | 2 | `k_con_comver33a`, `k_con_comver34` |
| +79040 | 2 | `k_act_walkways`, `k_act_walkways2` |
| +82566 | 2 | `k_def_repuser`, `k_hen_retreat` |
| +111232 | 2 | `k_amb_prey_ude`, `k_def_userdef01` |
| +113305 | 2 | `k_pend_door05`, `k_pend_door09` |
| +124271 | 2 | `k_pkas_chuundead`, `k_pkas_freyyhelp` |
| -728 | 1 | `k_sp1_generic` |
| -146 | 1 | `k_hcan_state22` |
| -132 | 1 | `k_hjuh_h12` |
| -112 | 1 | `k_def_buff` |
| -104 | 1 | `k_hjuh_p21` |
| -87 | 1 | `k_ptat_exitkrayt` |
| -76 | 1 | `k_hcan_state23` |
| -74 | 1 | `k_trp_generic` |
| -62 | 1 | `k_hjuh_p23` |
| -59 | 1 | `k_sup_comshots` |
| -42 | 1 | `k_ptat_gaffiwrit` |
| -40 | 1 | `k_pman_tatracery` |
| -26 | 1 | `k_pman_tatracern` |
| +18 | 1 | `k_pman_judge1_d` |
| +70 | 1 | `k_trg_transfail1` |
| +91 | 1 | `k_sup_droid` |
| +183 | 1 | `k_ptar_sitharm` |
| +193 | 1 | `k_swg_bastila22` |
| +289 | 1 | `k_pman_beast_d` |
| +299 | 1 | `k_trg_senni` |
| +1220 | 1 | `k_qa` |
| +5972 | 1 | `k_ren_leavlev` |
| +6332 | 1 | `k_ren_nextplanet` |
| +6522 | 1 | `k_ren_turret` |
| +6875 | 1 | `k_ren_visionland` |
| +8384 | 1 | `k_ren_unkcrash` |
| +8387 | 1 | `k_ren_starland` |
| +8388 | 1 | `k_ren_levescape` |
| +8391 | 1 | `k_ren_taris03` |
| +8413 | 1 | `k_ren_taris02` |
| +8418 | 1 | `k_ren_levcapture` |
| +10717 | 1 | `k_act_bandatt` |
| +11171 | 1 | `k_act_caloatt` |
| +27047 | 1 | `k_pla_actmap` |
| +27454 | 1 | `k_actq_com34` |
| +27699 | 1 | `k_plc_tresdrdhig` |
| +27734 | 1 | `k_act_com31` |
| +27835 | 1 | `k_act_com44` |
| +27906 | 1 | `k_messenger_plot` |
| +28011 | 1 | `k_amb_predator01` |
| +28028 | 1 | `k_bant_trig` |
| +28044 | 1 | `k_act_electric` |
| +28136 | 1 | `k_con_neutral` |
| +28166 | 1 | `k_pebn_swoop` |
| +28203 | 1 | `k_sup_gohawk` |
| +28235 | 1 | `k_sup_star_map` |
| +28237 | 1 | `k_com_compart` |
| +28271 | 1 | `k_act_startgas` |
| +28347 | 1 | `k_hjuh_p07d` |
| +28375 | 1 | `k_hjuh_p15` |
| +28379 | 1 | `k_hjuh_p07l` |
| +28411 | 1 | `k_act_nospeak` |
| +28461 | 1 | `k_con_com31` |
| +28471 | 1 | `k_act_com35` |
| +28609 | 1 | `k_sup_guiopen` |
| +28658 | 1 | `k_con_dark` |
| +28690 | 1 | `k_con_light` |
| +28761 | 1 | `k_con_tokens` |
| +28796 | 1 | `k_swg_jolee18` |
| +28838 | 1 | `k_swg_jolee12` |
| +28880 | 1 | `k_con_chrmiddle` |
| +28923 | 1 | `k_amb_enemyflee` |
| +28924 | 1 | `k_con_talkedto` |
| +28934 | 1 | `k_con_comhack` |
| +28950 | 1 | `k_act_comp01` |
| +28955 | 1 | `k_com_comrepr` |
| +28972 | 1 | `k_con_lighthigh` |
| +28984 | 1 | `k_act_talktrue` |
| +29084 | 1 | `k_act_foeinsight` |
| +29098 | 1 | `k_con_repair` |
| +29120 | 1 | `k_def_pathfail01` |
| +29150 | 1 | `k_trg_messengers` |
| +29240 | 1 | `k_con_comver41` |
| +29262 | 1 | `k_act_com41` |
| +32435 | 1 | `k_ai_master` |
| +35355 | 1 | `k_ptar_sithdis` |
| +36437 | 1 | `k_sup_healing` |
| +36566 | 1 | `k_sup_grenade` |
| +39994 | 1 | `k_def_rependd` |
| +73130 | 1 | `k_pdan_mand52` |
| +73385 | 1 | `k_def_calospn` |
| +73419 | 1 | `k_def_bandspn` |
| +76480 | 1 | `k_pkas_freyyspwn` |
| +76705 | 1 | `k_def_repairsp` |
| +76763 | 1 | `k_pdan_kath03` |
| +76798 | 1 | `k_pdan_stand02` |
| +76806 | 1 | `k_hen_spawn01` |
| +82341 | 1 | `k_hen_leadchng` |
| +82564 | 1 | `k_swg_xor_user` |
| +83502 | 1 | `k_zon_catalog` |
| +107706 | 1 | `k_pdan_iriaz_ud` |
| +109538 | 1 | `k_trg_partyinit` |
| +110211 | 1 | `k_act_com45` |
| +111056 | 1 | `k_hen_canduser` |
| +111117 | 1 | `k_def_calodef` |
| +111148 | 1 | `k_ptat_gizkauser` |
| +111182 | 1 | `k_pkas_randwalk` |
| +111286 | 1 | `k_pend_door06` |
| +111690 | 1 | `k_zon_control` |
| +111693 | 1 | `k_trg_partyheart` |
| +112135 | 1 | `k_pend_explode` |
| +113087 | 1 | `k_pend_door02` |
| +113308 | 1 | `k_pend_door01` |
| +113773 | 1 | `k_pman_door32` |
| +113779 | 1 | `k_pman_door20` |
| +114112 | 1 | `k_trg_stealth` |
| +114638 | 1 | `k_pdan_kath04` |
| +114650 | 1 | `k_pdan_kath02` |
| +114707 | 1 | `k_pdan_droid50` |
| +114717 | 1 | `k_pdan_belaya02` |
| +114721 | 1 | `k_pdan_droid09` |
| +114731 | 1 | `k_pdan_murder55` |
| +114755 | 1 | `k_pdan_droid02` |
| +114770 | 1 | `k_pdan_droid15` |
| +114877 | 1 | `k_pdan_elise_d` |
| +123083 | 1 | `k_pkas_freyyuser` |
| +123127 | 1 | `k_hjol_userdef01` |
| +123470 | 1 | `k_hjol_spawn01` |
| +124270 | 1 | `k_pkas_jaaraksus` |
| +124272 | 1 | `k_pkas_jaarakblt` |

## nwnrs-o0

nwnrs 0.0.1, nwnrs pack, --optimization O0

Of the 1731 outputs that differ from BioWare's bytes, 688 make the same engine calls with the same arguments and return the same result under the recording engine, 1007 behave differently, and 36 do not run.

Failed (23):

- **undeclared** (3): uses a name the shipped sources do not declare: a constant BioWare's own build had (TRASK_DOOR_JAMMED), or a Neverwinter Nights function KotOR's table lacks (StartConversation, ResistSpell)
  - `k_pend_door04`: ERROR nwnrs::pack: error=failed to compile k_pend_door04.nss: undefined identifier "TRASK_DOOR_JAMMED" (-622)
  - `nw_g0_conversat`: ERROR nwnrs::pack: error=failed to compile nw_g0_conversat.nss: undefined function "StartConversation" (-622)
  - `nw_s0_lghtnbolt`: ERROR nwnrs::pack: error=failed to compile nw_s0_lghtnbolt.nss: undefined function "ResistSpell" (-622)
- **stale-prototype** (15): BioWare compiled this against declarations that were not shipped: the shipped nwscript.nss prototype of AddPartyMember/RemovePartyMember, or k_inc_ebonhawk's forward declaration of EBO_PlayTakeOff, disagrees with the call or definition
  - `k_act_bastadd`: ERROR nwnrs::pack: error=failed to compile k_act_bastadd.nss: call to "AddPartyMember" did not supply enough parameters: 1 < 2 (-587)
  - `k_act_bastrmv`: ERROR nwnrs::pack: error=failed to compile k_act_bastrmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_canderrmv`: ERROR nwnrs::pack: error=failed to compile k_act_canderrmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_carthrmv`: ERROR nwnrs::pack: error=failed to compile k_act_carthrmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_hki47rmv`: ERROR nwnrs::pack: error=failed to compile k_act_hki47rmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_joleermv`: ERROR nwnrs::pack: error=failed to compile k_act_joleermv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_juhanirmv`: ERROR nwnrs::pack: error=failed to compile k_act_juhanirmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_makeitem`: ERROR nwnrs::pack: error=failed to compile k_act_makeitem.nss: function "EBO_PlayTakeOff" declaration differs from previous declaration (-617)
  - `k_act_missionrmv`: ERROR nwnrs::pack: error=failed to compile k_act_missionrmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_t3m3rmv`: ERROR nwnrs::pack: error=failed to compile k_act_t3m3rmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_zaalrmv`: ERROR nwnrs::pack: error=failed to compile k_act_zaalrmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_con_makeitem`: ERROR nwnrs::pack: error=failed to compile k_con_makeitem.nss: function "EBO_PlayTakeOff" declaration differs from previous declaration (-617)
  - `k_pebn_galaxy`: ERROR nwnrs::pack: error=failed to compile k_pebn_galaxy.nss: function "EBO_PlayTakeOff" declaration differs from previous declaration (-617)
  - `k_sup_galaxmap`: ERROR nwnrs::pack: error=failed to compile k_sup_galaxmap.nss: function "EBO_PlayTakeOff" declaration differs from previous declaration (-617)
  - `k_sup_galaxymap`: ERROR nwnrs::pack: error=failed to compile k_sup_galaxymap.nss: function "EBO_PlayTakeOff" declaration differs from previous declaration (-617)
- **not-a-script** (5): an include file with no entry point; there is nothing to compile
  - `k_inc_generic`: ERROR nwnrs::pack: error=failed to compile k_inc_generic.nss: script must define StartingConditional (-5182)
  - `k_inc_gensupport`: ERROR nwnrs::pack: error=failed to compile k_inc_gensupport.nss: script must define StartingConditional (-5182)
  - `k_inc_switch`: ERROR nwnrs::pack: error=failed to compile k_inc_switch.nss: script must define StartingConditional (-5182)
  - `k_inc_unk`: ERROR nwnrs::pack: error=failed to compile k_inc_unk.nss: script must define StartingConditional (-5182)
  - `k_inc_walkways`: ERROR nwnrs::pack: error=failed to compile k_inc_walkways.nss: script must define StartingConditional (-5182)

Different output (1731), by cause:

- **stale-include** (73): BioWare's bytecode was compiled before an include it uses last changed: the first difference is in an include's globals or functions (k_inc_utility's SW_CONSTANT_* globals, k_inc_generic's GN_TalentMasterRoutine, among others)
  `k_act_atkonend`, `k_act_caloatt`, `k_act_com31`, `k_act_com32`, `k_act_com33`, `k_act_com33a`, `k_act_com34`, `k_act_com35`, `k_act_com41`, `k_act_com42`, `k_act_com43`, `k_act_com45`, `k_act_comp01`, `k_act_electric`, `k_act_startgas`, `k_act_xoratt`, `k_actq_com33`, `k_actq_com33a`, `k_actq_com34`, `k_amb_enemyflee`, `k_amb_predator01`, `k_com_compart`, `k_com_comrepr`, `k_con_com31`, `k_con_com32`, `k_con_com33`, `k_con_com34`, `k_con_com35`, `k_con_com41`, `k_con_com42`, `k_con_com43`, `k_con_com44`, `k_con_com45`, `k_con_comactive`, `k_con_comhack`, `k_con_comrep`, `k_con_comver31`, `k_con_comver32`, `k_con_comver33`, `k_con_comver33a`, `k_con_comver34`, `k_con_comver41`, `k_con_comver42`, `k_con_comver43`, `k_con_comver44`, `k_con_comver45`, `k_con_tokens`, `k_def_rependd`, `k_pla_actmap`, `k_plc_tresciv`, `k_plc_trescorhig`, `k_plc_trescorlow`, `k_plc_trescormid`, `k_plc_tresdrdhig`, `k_plc_tresdrdlow`, `k_plc_tresdrdmid`, `k_plc_tresmilhig`, `k_plc_tresmillow`, `k_plc_tresmilmid`, `k_plc_tresrakat`, `k_plc_tresshahig`, `k_plc_tresshalow`, `k_plc_tresshamid`, `k_plc_tressndppl`, `k_ptat17aa_enter`, `k_ptat_gammenemy`, `k_sp1_generic`, `k_sup_star_map`, `k_trg_messengers`, `k_trg_partyheart`, `k_trg_stealth`, `k_zon_catalog`, `k_zon_control`
- **stale-bytecode** (8): BioWare's bytecode was compiled from an earlier version of this script's own source: the first difference is in the script, and it is a different call, constant or condition
  `k_ptat_exitkrayt`, `k_qa`, `k_sup_droid`, `k_sup_gohawk`, `k_sup_grenade`, `k_trg_senni`, `k_trg_transfail1`, `k_trp_generic`
- **conversation-twelve** (3): BioWare compiled this against the shipped twelve-argument ActionStartConversation, unlike most of its scripts. Identical with the shipped declarations
  `k_ai_master`, `k_pman_27_init01`, `k_trg_transfail`
- **conversation-eleven** (51): BioWare compiled this when ActionStartConversation took eleven arguments; the shipped declaration has twelve. Identical with --declarations bioware
  `k_bant_trig`, `k_def_bandspn`, `k_def_calodef`, `k_hjol_userdef01`, `k_pdan_elise_d`, `k_pdan_rapid01`, `k_pdan_rapid02`, `k_pdan_rapid03`, `k_pdan_rapid04`, `k_pdan_rapid05`, `k_pend_door01`, `k_pend_door02`, `k_pend_door03`, `k_pend_door08`, `k_pkas_freyyuser`, `k_pman_26a_area`, `k_pman_28ac_init`, `k_pman_at01`, `k_pman_at02`, `k_pman_door03`, `k_pman_door06`, `k_pman_door07`, `k_pman_door08`, `k_pman_door10`, `k_pman_door11`, `k_pman_door13`, `k_pman_door14`, `k_pman_door20`, `k_pman_door30`, `k_pman_init01`, `k_pman_init02`, `k_pman_init03`, `k_pman_init10`, `k_pman_judge1_d`, `k_pman_obj_asc`, `k_pman_obj_asc2`, `k_pman_obj_ascc`, `k_pman_obj_ascc2`, `k_pman_obj_ascn`, `k_pman_osc`, `k_pman_rapid01`, `k_pman_rapid02`, `k_pman_rapid03`, `k_pman_rapid04`, `k_pman_rapid05`, `k_pman_sitspeak`, `k_pman_survive01`, `k_ptar_sitharm`, `k_ptat_krayttran`, `k_swg_xor_user`, `k_trg_partyinit`
- **no known cause** (1596): `k_act_bandatt`, `k_act_canderadd`, `k_act_candstim01`, `k_act_candstim02`, `k_act_candstim03`, `k_act_carthadd`, `k_act_cmusic_off`, `k_act_cmusic_on`, `k_act_com44`, `k_act_darkhigh`, `k_act_darkmed`, `k_act_darksml`, `k_act_exitarea`, `k_act_exitarea2`, `k_act_foeinsight`, `k_act_heartoff`, `k_act_hearton`, `k_act_hk47add`, `k_act_hk47simul`, `k_act_joleeadd`, `k_act_juhaniadd`, `k_act_leaverun`, `k_act_lighthigh`, `k_act_lightmed`, `k_act_lightsml`, `k_act_mispaz`, `k_act_missionadd`, `k_act_nospeak`, `k_act_noswitch`, `k_act_npctalk`, `k_act_t3m3add`, `k_act_talktrue`, `k_act_walkways`, `k_act_walkways2`, `k_act_xordie`, `k_act_zaaladd`, `k_amb_prey_spawn`, `k_amb_prey_ude`, `k_cheat02`, `k_cheat_01`, `k_cht_n_bast`, `k_cht_n_cand`, `k_cht_n_carth`, `k_cht_n_hk47`, `k_cht_n_jolee`, `k_cht_n_juhani`, `k_cht_n_mission`, `k_cht_n_screen`, `k_cht_n_t3m4`, `k_cht_n_zaalbar`, `k_cht_p_dan`, `k_cht_p_end`, `k_cht_p_kas`, `k_cht_p_kor`, `k_cht_p_lev`, `k_cht_p_man`, `k_cht_p_sta`, `k_cht_p_tar`, `k_cht_p_tat`, `k_cht_p_unk`, `k_cht_v_levi`, `k_cht_v_mast`, `k_cht_v_star`, `k_con_at_ebon`, `k_con_atdant`, `k_con_atforge`, `k_con_atkash`, `k_con_atkorr`, `k_con_atlev`, `k_con_atmann`, `k_con_atspire`, `k_con_attaris`, `k_con_attat`, `k_con_atunk`, `k_con_bastnpm`, `k_con_bastpm`, `k_con_candernpm`, `k_con_canderpm`, `k_con_candstim01`, `k_con_carthnpm`, `k_con_carthpm`, `k_con_chrhigh`, `k_con_chrlow`, `k_con_chrmiddle`, `k_con_chrnormal`, `k_con_classdiff`, `k_con_classsame`, `k_con_dark`, `k_con_darkhigh`, `k_con_darklow`, `k_con_flirt`, `k_con_flirtno`, `k_con_fpershigh`, `k_con_fperslow`, `k_con_heal`, `k_con_hk47npm`, `k_con_hk47pm`, `k_con_inrange`, `k_con_inthigh`, `k_con_intlow`, `k_con_intnormal`, `k_con_isfemale`, `k_con_isjedi`, `k_con_ismale`, `k_con_isscout`, `k_con_issmug`, `k_con_issoldier`, `k_con_joleejoin`, `k_con_joleenpm`, `k_con_joleepm`, `k_con_juhanilive`, `k_con_juhaninpm`, `k_con_juhanipm`, `k_con_light`, `k_con_lighthigh`, `k_con_lightlow`, `k_con_missionnpm`, `k_con_missionpm`, `k_con_neutral`, `k_con_npctalk`, `k_con_perseasy`, `k_con_pershigh`, `k_con_persmed`, `k_con_repair`, `k_con_t3m3npm`, `k_con_t3m3pm`, `k_con_talkedto`, `k_con_zaalnpm`, `k_con_zaalpm`, `k_def_ambient`, `k_def_ambmob`, `k_def_attacked01`, `k_def_blocked01`, `k_def_buff`, `k_def_calospn`, `k_def_combend01`, `k_def_damage01`, `k_def_death01`, `k_def_dialogue01`, `k_def_disturb01`, `k_def_endconv`, `k_def_grenspn`, `k_def_heartbt01`, `k_def_pathfail01`, `k_def_percept01`, `k_def_repairsp`, `k_def_repuser`, `k_def_spawn01`, `k_def_spawndead`, `k_def_spellat01`, `k_def_userdef01`, `k_hbanter1`, `k_hbanter10`, `k_hbanter10x`, `k_hbanter11`, `k_hbanter11x`, `k_hbanter1x`, `k_hbanter2`, `k_hbanter2x`, `k_hbanter3`, `k_hbanter3x`, `k_hbanter4`, `k_hbanter4x`, `k_hbanter5`, `k_hbanter5x`, `k_hbanter6`, `k_hbanter6x`, `k_hbanter7`, `k_hbanter7x`, `k_hbanter8`, `k_hbanter8x`, `k_hbanter9`, `k_hbanter9x`, `k_hbas_final`, `k_hbas_playkash`, `k_hbas_playkorr`, `k_hbas_playman`, `k_hbas_playtat`, `k_hcan_state01`, `k_hcan_state02`, `k_hcan_state03`, `k_hcan_state04`, `k_hcan_state05`, `k_hcan_state06`, `k_hcan_state07`, `k_hcan_state08`, `k_hcan_state09`, `k_hcan_state10`, `k_hcan_state11`, `k_hcan_state12`, `k_hcan_state13`, `k_hcan_state14`, `k_hcan_state15`, `k_hcan_state16`, `k_hcan_state17`, `k_hcan_state18`, `k_hcan_state19`, `k_hcan_state20`, `k_hcan_state21`, `k_hcan_state22`, `k_hcan_state23`, `k_hcan_state24`, `k_hcan_state25`, `k_hcan_state26`, `k_hcan_state27`, `k_hcan_state28`, `k_hcan_state29`, `k_hcan_state30`, `k_hcan_state31`, `k_hcan_state32`, `k_hcan_state33`, `k_hcan_state34`, `k_hcan_state35`, `k_hcan_state36`, `k_hcan_state37`, `k_hcan_state38`, `k_hcan_state39`, `k_hcan_state40`, `k_hcan_state41`, `k_hcan_state42`, `k_hcan_state43`, `k_hcan_state44`, `k_hcan_state45`, `k_hen_attacked01`, `k_hen_blocked01`, `k_hen_candspwn`, `k_hen_canduser`, `k_hen_combend01`, `k_hen_damage01`, `k_hen_dialogue01`, `k_hen_enter5m`, `k_hen_exit5m`, `k_hen_heartbt01`, `k_hen_leadchng`, `k_hen_percept01`, `k_hen_retreat`, `k_hen_spawn01`, `k_hjol_spawn01`, `k_hjuh_com01`, `k_hjuh_d01`, `k_hjuh_d02`, `k_hjuh_h01`, `k_hjuh_h01x`, `k_hjuh_h02`, `k_hjuh_h03`, `k_hjuh_h04`, `k_hjuh_h05`, `k_hjuh_h06`, `k_hjuh_h07`, `k_hjuh_h08`, `k_hjuh_h09`, `k_hjuh_h10`, `k_hjuh_h10x`, `k_hjuh_h11`, `k_hjuh_h12`, `k_hjuh_h13`, `k_hjuh_p01`, `k_hjuh_p02`, `k_hjuh_p03`, `k_hjuh_p04`, `k_hjuh_p05`, `k_hjuh_p06`, `k_hjuh_p07`, `k_hjuh_p07d`, `k_hjuh_p07l`, `k_hjuh_p08`, `k_hjuh_p09`, `k_hjuh_p10`, `k_hjuh_p10x`, `k_hjuh_p10y`, `k_hjuh_p11`, `k_hjuh_p12`, `k_hjuh_p13`, `k_hjuh_p14`, `k_hjuh_p15`, `k_hjuh_p16`, `k_hjuh_p17`, `k_hjuh_p18`, `k_hjuh_p19`, `k_hjuh_p20`, `k_hjuh_p21`, `k_hjuh_p22`, `k_hjuh_p23`, `k_hjuh_p24`, `k_hjuh_p25`, `k_hjuh_p26`, `k_hjuh_p27`, `k_hjuh_p28`, `k_hjuh_s01`, `k_hjuh_s02`, `k_hjuh_s03`, `k_hjuh_s04`, `k_hjuh_s05`, `k_hjuh_s06`, `k_hjuh_s07`, `k_hjuh_w01`, `k_hjuh_w02`, `k_hjuh_w03`, `k_hjuh_w04`, `k_hjuh_w05`, `k_hjuh_w06`, `k_hjuh_w07`, `k_hjuh_w08`, `k_hjuh_w09`, `k_hjuh_w10`, `k_hmis_misangry`, `k_hmis_misangryx`, `k_hmis_paz`, `k_hmis_planet`, `k_hmis_talk0`, `k_hmis_talk0+`, `k_hmis_talk1`, `k_hmis_talk10`, `k_hmis_talk10lev`, `k_hmis_talk10x`, `k_hmis_talk11`, `k_hmis_talk11lev`, `k_hmis_talk11x`, `k_hmis_talk12`, `k_hmis_talk12x`, `k_hmis_talk13x`, `k_hmis_talk14`, `k_hmis_talk14x`, `k_hmis_talk15`, `k_hmis_talk15x`, `k_hmis_talk1lev`, `k_hmis_talk1x`, `k_hmis_talk2x`, `k_hmis_talk3`, `k_hmis_talk3lev`, `k_hmis_talk3x`, `k_hmis_talk4x`, `k_hmis_talk5`, `k_hmis_talk5x`, `k_hmis_talk6`, `k_hmis_talk6-8`, `k_hmis_talk6lev`, `k_hmis_talk6x`, `k_hmis_talk7`, `k_hmis_talk7lev`, `k_hmis_talk7x`, `k_hmis_talk8`, `k_hmis_talk8x`, `k_hmis_talk9`, `k_hmis_talk99`, `k_hmis_talk99x`, `k_hmis_talk9x`, `k_hmis_talkrev`, `k_hmis_tardest`, `k_hmis_tardestx`, `k_hmis_zaalbar`, `k_hxor_state01`, `k_hxor_state02`, `k_hxor_state03`, `k_hzal_atkash1`, `k_hzal_atkash2`, `k_hzal_beenkashy`, `k_hzal_chuundead`, `k_hzal_freyydead`, `k_hzal_notkash1`, `k_hzal_notkash2`, `k_hzal_talk0`, `k_hzal_talk_1up`, `k_hzal_talkmad_c`, `k_hzal_talkmad_s`, `k_hzal_talkset_1`, `k_messenger_plot`, `k_pdan_anger01`, `k_pdan_anger02`, `k_pdan_anger03`, `k_pdan_anger04`, `k_pdan_anger05`, `k_pdan_anger06`, `k_pdan_anger07`, `k_pdan_anger08`, `k_pdan_anger09`, `k_pdan_anger10`, `k_pdan_anger11`, `k_pdan_anger12`, `k_pdan_anger13`, `k_pdan_anger14`, `k_pdan_anger15`, `k_pdan_anger16`, `k_pdan_anger17`, `k_pdan_anger18`, `k_pdan_anger19`, `k_pdan_anger20`, `k_pdan_bastial05`, `k_pdan_bastila01`, `k_pdan_bastila02`, `k_pdan_bastila03`, `k_pdan_bastila04`, `k_pdan_bastila05`, `k_pdan_bastila06`, `k_pdan_bastila07`, `k_pdan_bastila08`, `k_pdan_bastila09`, `k_pdan_bastila10`, `k_pdan_bastila11`, `k_pdan_belaya01`, `k_pdan_belaya02`, `k_pdan_cander01`, `k_pdan_cander02`, `k_pdan_cander03`, `k_pdan_carth01`, `k_pdan_carth02`, `k_pdan_carth03`, `k_pdan_carth04`, `k_pdan_casus01`, `k_pdan_casus02`, `k_pdan_casus03`, `k_pdan_casus04`, `k_pdan_casus05`, `k_pdan_destself`, `k_pdan_drandom02`, `k_pdan_drandom03`, `k_pdan_drandom04`, `k_pdan_drandom05`, `k_pdan_drandom06`, `k_pdan_drandom07`, `k_pdan_drandom08`, `k_pdan_droid01`, `k_pdan_droid02`, `k_pdan_droid03`, `k_pdan_droid04`, `k_pdan_droid05`, `k_pdan_droid06`, `k_pdan_droid07`, `k_pdan_droid08`, `k_pdan_droid09`, `k_pdan_droid10`, `k_pdan_droid11`, `k_pdan_droid12`, `k_pdan_droid13`, `k_pdan_droid14`, `k_pdan_droid15`, `k_pdan_droid50`, `k_pdan_iriaz_sp`, `k_pdan_iriaz_ud`, `k_pdan_jedi01`, `k_pdan_jedi02`, `k_pdan_jedi03`, `k_pdan_jedi04`, `k_pdan_jedi05`, `k_pdan_jedi06`, `k_pdan_jedi07`, `k_pdan_jedi08`, `k_pdan_jedi09`, `k_pdan_jedi10`, `k_pdan_jedi11`, `k_pdan_jedi12`, `k_pdan_jedi13`, `k_pdan_jedi14`, `k_pdan_jedi15`, `k_pdan_jedi16`, `k_pdan_jedi17`, `k_pdan_jedi18`, `k_pdan_jedi19`, `k_pdan_jedi20`, `k_pdan_jedi21`, `k_pdan_jedi22`, `k_pdan_jedi23`, `k_pdan_jedi24`, `k_pdan_jedi25`, `k_pdan_jedi26`, `k_pdan_jedi27`, `k_pdan_jedi28`, `k_pdan_jedi29`, `k_pdan_jedi30`, `k_pdan_jedi31`, `k_pdan_jedi32`, `k_pdan_jedi33`, `k_pdan_jedi34`, `k_pdan_jedi35`, `k_pdan_jedi36`, `k_pdan_jedi37`, `k_pdan_jedi38`, `k_pdan_jedi39`, `k_pdan_jedi40`, `k_pdan_juhani01`, `k_pdan_juhani02`, `k_pdan_juhani03`, `k_pdan_juhani04`, `k_pdan_juhani05`, `k_pdan_juhani06`, `k_pdan_juhani07`, `k_pdan_juhani08`, `k_pdan_juhani09`, `k_pdan_juhani10`, `k_pdan_juhani11`, `k_pdan_juhani12`, `k_pdan_juhani15`, `k_pdan_juhani16`, `k_pdan_juhani17`, `k_pdan_juhani18`, `k_pdan_juhani19`, `k_pdan_juhani20`, `k_pdan_kath01`, `k_pdan_kath02`, `k_pdan_kath03`, `k_pdan_kath04`, `k_pdan_mand01`, `k_pdan_mand02`, `k_pdan_mand03`, `k_pdan_mand04`, `k_pdan_mand05`, `k_pdan_mand06`, `k_pdan_mand07`, `k_pdan_mand08`, `k_pdan_mand09`, `k_pdan_mand10`, `k_pdan_mand11`, `k_pdan_mand12`, `k_pdan_mand51`, `k_pdan_mand52`, `k_pdan_mission01`, `k_pdan_mission02`, `k_pdan_murder01`, `k_pdan_murder02`, `k_pdan_murder03`, `k_pdan_murder04`, `k_pdan_murder05`, `k_pdan_murder06`, `k_pdan_murder07`, `k_pdan_murder08`, `k_pdan_murder09`, `k_pdan_murder10`, `k_pdan_murder11`, `k_pdan_murder12`, `k_pdan_murder13`, `k_pdan_murder14`, `k_pdan_murder15`, `k_pdan_murder16`, `k_pdan_murder17`, `k_pdan_murder18`, `k_pdan_murder19`, `k_pdan_murder20`, `k_pdan_murder21`, `k_pdan_murder22`, `k_pdan_murder23`, `k_pdan_murder24`, `k_pdan_murder25`, `k_pdan_murder26`, `k_pdan_murder27`, `k_pdan_murder28`, `k_pdan_murder29`, `k_pdan_murder30`, `k_pdan_murder31`, `k_pdan_murder32`, `k_pdan_murder33`, `k_pdan_murder34`, `k_pdan_murder35`, `k_pdan_murder36`, `k_pdan_murder37`, `k_pdan_murder38`, `k_pdan_murder39`, `k_pdan_murder40`, `k_pdan_murder41`, `k_pdan_murder42`, `k_pdan_murder43`, `k_pdan_murder44`, `k_pdan_murder45`, `k_pdan_murder46`, `k_pdan_murder47`, `k_pdan_murder48`, `k_pdan_murder49`, `k_pdan_murder50`, `k_pdan_murder51`, `k_pdan_murder52`, `k_pdan_murder53`, `k_pdan_murder54`, `k_pdan_murder55`, `k_pdan_murder56`, `k_pdan_murder57`, `k_pdan_murder58`, `k_pdan_murder59`, `k_pdan_murder60`, `k_pdan_murder61`, `k_pdan_murder62`, `k_pdan_murder63`, `k_pdan_murder64`, `k_pdan_murder65`, `k_pdan_murder66`, `k_pdan_murder67`, `k_pdan_murder68`, `k_pdan_murder69`, `k_pdan_murder70`, `k_pdan_murder71`, `k_pdan_murder72`, `k_pdan_murder73`, `k_pdan_murder74`, `k_pdan_murder75`, `k_pdan_murder76`, `k_pdan_planet01`, `k_pdan_planet02`, `k_pdan_planet03`, `k_pdan_planet04`, `k_pdan_planet05`, `k_pdan_planet06`, `k_pdan_planet08`, `k_pdan_planet12`, `k_pdan_planet14`, `k_pdan_planet15`, `k_pdan_planet16`, `k_pdan_romance01`, `k_pdan_romance02`, `k_pdan_romance03`, `k_pdan_romance04`, `k_pdan_romance05`, `k_pdan_romance06`, `k_pdan_romance07`, `k_pdan_romance08`, `k_pdan_romance09`, `k_pdan_romance10`, `k_pdan_romance11`, `k_pdan_romance12`, `k_pdan_romance13`, `k_pdan_romance14`, `k_pdan_romance15`, `k_pdan_romance16`, `k_pdan_romance17`, `k_pdan_romance18`, `k_pdan_romance19`, `k_pdan_romance20`, `k_pdan_romance21`, `k_pdan_romance22`, `k_pdan_romance23`, `k_pdan_romance24`, `k_pdan_romance25`, `k_pdan_romance26`, `k_pdan_romance27`, `k_pdan_romance28`, `k_pdan_romance29`, `k_pdan_romance30`, `k_pdan_romance31`, `k_pdan_romance32`, `k_pdan_romance33`, `k_pdan_romance34`, `k_pdan_romance35`, `k_pdan_romance36`, `k_pdan_romance37`, `k_pdan_romance38`, `k_pdan_romance39`, `k_pdan_romance40`, `k_pdan_romance41`, `k_pdan_romance42`, `k_pdan_stand01`, `k_pdan_stand02`, `k_pdan_stand03`, `k_pebn_ctaris01`, `k_pebn_ctaris02`, `k_pebn_ctaris03`, `k_pebn_ctaris04`, `k_pebn_ctaris05`, `k_pebn_ctaris06`, `k_pebn_ctaris07`, `k_pebn_ctaris08`, `k_pebn_ctaris09`, `k_pebn_gotaris`, `k_pebn_loc01`, `k_pebn_loc02`, `k_pebn_loc03`, `k_pebn_loc04`, `k_pebn_loc05`, `k_pebn_loc06`, `k_pebn_loc07`, `k_pebn_loc08`, `k_pebn_loc09`, `k_pebn_swoop`, `k_pend_door05`, `k_pend_door06`, `k_pend_door07`, `k_pend_door09`, `k_pend_explode`, `k_pend_rumble01`, `k_phkd_fighter`, `k_pkas_chuundead`, `k_pkas_enemyfac`, `k_pkas_freyydead`, `k_pkas_freyyhelp`, `k_pkas_freyyspwn`, `k_pkas_hashilt`, `k_pkas_hassword`, `k_pkas_heart_on`, `k_pkas_jaarakblt`, `k_pkas_jaaraksus`, `k_pkas_jaardead`, `k_pkas_jaarfreed`, `k_pkas_nohilt`, `k_pkas_nosword`, `k_pkas_pause3sec`, `k_pkas_randwalk`, `k_pkas_takehilt`, `k_pman_26a_aread`, `k_pman_26c_area`, `k_pman_26d_area`, `k_pman_26d_triga`, `k_pman_26d_trigb`, `k_pman_26d_trigc`, `k_pman_27_area`, `k_pman_28b_at01`, `k_pman_28b_at02`, `k_pman_28c_sur01`, `k_pman_28d_cmp01`, `k_pman_28d_cmp02`, `k_pman_28d_triga`, `k_pman_28d_trigb`, `k_pman_aioff`, `k_pman_aion`, `k_pman_airlock01`, `k_pman_airlock02`, `k_pman_airlock03`, `k_pman_amb_9`, `k_pman_ambient01`, `k_pman_ambient02`, `k_pman_ambient03`, `k_pman_ambient04`, `k_pman_ambient05`, `k_pman_ambient06`, `k_pman_ambient07`, `k_pman_ambient08`, `k_pman_area_28a`, `k_pman_arg01`, `k_pman_argrep_d`, `k_pman_argsith_d`, `k_pman_beast_9`, `k_pman_beast_d`, `k_pman_body01`, `k_pman_casamad_s`, `k_pman_casamad_y`, `k_pman_casname_n`, `k_pman_casname_s`, `k_pman_champ_f01`, `k_pman_champ_f02`, `k_pman_champ_f03`, `k_pman_champ_f04`, `k_pman_champ_m01`, `k_pman_champ_m02`, `k_pman_champ_m03`, `k_pman_champ_m04`, `k_pman_champ_qui`, `k_pman_comp01`, `k_pman_comp02`, `k_pman_comp03`, `k_pman_comp04`, `k_pman_comp05`, `k_pman_comp06`, `k_pman_comp07`, `k_pman_comp08`, `k_pman_comp09`, `k_pman_comp10`, `k_pman_comp11`, `k_pman_comp30`, `k_pman_comp31`, `k_pman_comp32`, `k_pman_comp33`, `k_pman_comp34`, `k_pman_comp35`, `k_pman_door01`, `k_pman_door02`, `k_pman_door04`, `k_pman_door05`, `k_pman_door12`, `k_pman_door31`, `k_pman_door32`, `k_pman_door40`, `k_pman_false`, `k_pman_firixa_d`, `k_pman_guard01`, `k_pman_hukname_n`, `k_pman_hukname_s`, `k_pman_huktmad_1`, `k_pman_huktmad_y`, `k_pman_ignore`, `k_pman_ignus01`, `k_pman_locked01`, `k_pman_locked02`, `k_pman_lostset_n`, `k_pman_lostset_q`, `k_pman_lostset_y`, `k_pman_main01`, `k_pman_misc01`, `k_pman_misc02`, `k_pman_misc03`, `k_pman_misc04`, `k_pman_misc05`, `k_pman_misc06`, `k_pman_misc07`, `k_pman_misc08`, `k_pman_misc09`, `k_pman_misc10`, `k_pman_misc11`, `k_pman_misc12`, `k_pman_misc13`, `k_pman_misc14`, `k_pman_misc15`, `k_pman_misc16`, `k_pman_misc17`, `k_pman_misc18`, `k_pman_misc19`, `k_pman_misc20`, `k_pman_misc21`, `k_pman_misc22`, `k_pman_misc23`, `k_pman_misc24`, `k_pman_misc25`, `k_pman_misc26`, `k_pman_misc27`, `k_pman_misc28`, `k_pman_misc29`, `k_pman_misc30`, `k_pman_misc31`, `k_pman_misc32`, `k_pman_misc33`, `k_pman_misc34`, `k_pman_misc35`, `k_pman_misc36`, `k_pman_misc37`, `k_pman_misc38`, `k_pman_misc39`, `k_pman_misc40`, `k_pman_misc41`, `k_pman_misc42`, `k_pman_misc43`, `k_pman_misc44`, `k_pman_misc45`, `k_pman_misc46`, `k_pman_misc47`, `k_pman_misc48`, `k_pman_misc49`, `k_pman_misc50`, `k_pman_misc51`, `k_pman_misc52`, `k_pman_misc53`, `k_pman_misc54`, `k_pman_misc55`, `k_pman_misc56`, `k_pman_misc57`, `k_pman_misc58`, `k_pman_misc59`, `k_pman_misc60`, `k_pman_misc61`, `k_pman_misc62`, `k_pman_misc70`, `k_pman_misc71`, `k_pman_misc72`, `k_pman_misc73`, `k_pman_misc74`, `k_pman_misc75`, `k_pman_misc76`, `k_pman_miss01`, `k_pman_missing01`, `k_pman_missing02`, `k_pman_missing03`, `k_pman_missing04`, `k_pman_missing05`, `k_pman_missing06`, `k_pman_missing07`, `k_pman_missing08`, `k_pman_missing09`, `k_pman_missing10`, `k_pman_missing11`, `k_pman_missing12`, `k_pman_missing13`, `k_pman_missing14`, `k_pman_missing15`, `k_pman_missing16`, `k_pman_missing17`, `k_pman_missing18`, `k_pman_missing19`, `k_pman_missing20`, `k_pman_missing21`, `k_pman_missing22`, `k_pman_missing23`, `k_pman_missing24`, `k_pman_missing25`, `k_pman_missing26`, `k_pman_missing27`, `k_pman_missing28`, `k_pman_missing29`, `k_pman_missing30`, `k_pman_missing31`, `k_pman_missing32`, `k_pman_missing33`, `k_pman_missing34`, `k_pman_missing35`, `k_pman_missing36`, `k_pman_missing37`, `k_pman_missing38`, `k_pman_missing39`, `k_pman_missing40`, `k_pman_missing41`, `k_pman_missing42`, `k_pman_missing43`, `k_pman_murder01`, `k_pman_murder02`, `k_pman_murder03`, `k_pman_murder04`, `k_pman_murder05`, `k_pman_murder06`, `k_pman_murder07`, `k_pman_murder08`, `k_pman_murder09`, `k_pman_murder10`, `k_pman_murder11`, `k_pman_murder12`, `k_pman_murder13`, `k_pman_murder14`, `k_pman_murder15`, `k_pman_murder16`, `k_pman_murder17`, `k_pman_murder18`, `k_pman_murder19`, `k_pman_murder20`, `k_pman_murder21`, `k_pman_murder22`, `k_pman_murder23`, `k_pman_murder24`, `k_pman_murder25`, `k_pman_murder26`, `k_pman_murder27`, `k_pman_murder28`, `k_pman_murder29`, `k_pman_murder30`, `k_pman_murder31`, `k_pman_murder32`, `k_pman_murder33`, `k_pman_murder34`, `k_pman_murder35`, `k_pman_murder36`, `k_pman_murder37`, `k_pman_murder38`, `k_pman_murder39`, `k_pman_murder40`, `k_pman_murder41`, `k_pman_murder42`, `k_pman_murder43`, `k_pman_murder44`, `k_pman_murder45`, `k_pman_murder46`, `k_pman_murder47`, `k_pman_murder48`, `k_pman_murder49`, `k_pman_murder50`, `k_pman_murder51`, `k_pman_murder52`, `k_pman_murder53`, `k_pman_murder54`, `k_pman_murder55`, `k_pman_murder56`, `k_pman_murder57`, `k_pman_murder58`, `k_pman_murder59`, `k_pman_murder60`, `k_pman_murder61`, `k_pman_murder64`, `k_pman_murder65`, `k_pman_murder66`, `k_pman_murder67`, `k_pman_murder68`, `k_pman_murder69`, `k_pman_murder70`, `k_pman_murder71`, `k_pman_murder72`, `k_pman_murder73`, `k_pman_murder74`, `k_pman_murder75`, `k_pman_murder76`, `k_pman_murder77`, `k_pman_murder78`, `k_pman_murder79`, `k_pman_murder80`, `k_pman_murder81`, `k_pman_murder82`, `k_pman_murder83`, `k_pman_murder84`, `k_pman_murder85`, `k_pman_murder86`, `k_pman_murder87`, `k_pman_murder88`, `k_pman_murder89`, `k_pman_murder90`, `k_pman_murder91`, `k_pman_murder92`, `k_pman_murder93`, `k_pman_murder94`, `k_pman_murder95`, `k_pman_murder96`, `k_pman_murder97`, `k_pman_murder98`, `k_pman_murder99`, `k_pman_murder9a`, `k_pman_murder9b`, `k_pman_murder9c`, `k_pman_murder9d`, `k_pman_murder9e`, `k_pman_murder9f`, `k_pman_murder9g`, `k_pman_murder9h`, `k_pman_murder9i`, `k_pman_murder9j`, `k_pman_murder9k`, `k_pman_murder9l`, `k_pman_murder9m`, `k_pman_murder9n`, `k_pman_murder9o`, `k_pman_murder9p`, `k_pman_murder9q`, `k_pman_murder9r`, `k_pman_murder9s`, `k_pman_murder9t`, `k_pman_murder9u`, `k_pman_murder9v`, `k_pman_murder9w`, `k_pman_murder9x`, `k_pman_murder9y`, `k_pman_murder9z`, `k_pman_murdera01`, `k_pman_murdera02`, `k_pman_murdera03`, `k_pman_murdera04`, `k_pman_murdera05`, `k_pman_murdera06`, `k_pman_murdera07`, `k_pman_murdera08`, `k_pman_murdera09`, `k_pman_murdera11`, `k_pman_no_threat`, `k_pman_notpaid_c`, `k_pman_notpaid_n`, `k_pman_notpaid_s`, `k_pman_npcstart`, `k_pman_npcstop`, `k_pman_patrol_d`, `k_pman_pause2sec`, `k_pman_payrace_1`, `k_pman_payrace_2`, `k_pman_payrace_3`, `k_pman_pazaak01`, `k_pman_pazaak02`, `k_pman_pazaak03`, `k_pman_pazaak04`, `k_pman_pazaak05`, `k_pman_pazaak06`, `k_pman_pazaak07`, `k_pman_pazaak08`, `k_pman_pazaak09`, `k_pman_pazaak10`, `k_pman_pazaak11`, `k_pman_persuad_f`, `k_pman_persuad_s`, `k_pman_persuad_y`, `k_pman_planet01`, `k_pman_planet02`, `k_pman_planet03`, `k_pman_planet04`, `k_pman_planet05`, `k_pman_planet06`, `k_pman_planet07`, `k_pman_planet08`, `k_pman_planet09`, `k_pman_planet10`, `k_pman_planet11`, `k_pman_planet12`, `k_pman_planet13`, `k_pman_planet14`, `k_pman_planet15`, `k_pman_planet16`, `k_pman_planet17`, `k_pman_planet18`, `k_pman_planet19`, `k_pman_planet20`, `k_pman_planet21`, `k_pman_planet22`, `k_pman_planet23`, `k_pman_planet24`, `k_pman_planet25`, `k_pman_planet26`, `k_pman_planet27`, `k_pman_planet28`, `k_pman_planet29`, `k_pman_planet30`, `k_pman_planet31`, `k_pman_planet32`, `k_pman_planet33`, `k_pman_planet34`, `k_pman_planet35`, `k_pman_planet36`, `k_pman_planet37`, `k_pman_planet38`, `k_pman_planet39`, `k_pman_planet40`, `k_pman_planet41`, `k_pman_planet42`, `k_pman_planet43`, `k_pman_planet44`, `k_pman_planet45`, `k_pman_planet46`, `k_pman_planet47`, `k_pman_planet48`, `k_pman_planet49`, `k_pman_planet50`, `k_pman_planet51`, `k_pman_planet52`, `k_pman_planet53`, `k_pman_planet54`, `k_pman_planet55`, `k_pman_planet56`, `k_pman_planet57`, `k_pman_planet58`, `k_pman_planet59`, `k_pman_planet60`, `k_pman_planet61`, `k_pman_planet62`, `k_pman_planet63`, `k_pman_planet64`, `k_pman_planet65`, `k_pman_planet66`, `k_pman_planet67`, `k_pman_planet68`, `k_pman_planet69`, `k_pman_planet70`, `k_pman_planet71`, `k_pman_planet72`, `k_pman_planet73`, `k_pman_planet74`, `k_pman_planet75`, `k_pman_planet76`, `k_pman_planet77`, `k_pman_planet78`, `k_pman_planet79`, `k_pman_planet80`, `k_pman_planet81`, `k_pman_planet82`, `k_pman_planet83`, `k_pman_planet84`, `k_pman_planet85`, `k_pman_planet86`, `k_pman_planet87`, `k_pman_port01`, `k_pman_puzzle01`, `k_pman_puzzle02`, `k_pman_puzzle03`, `k_pman_puzzle04`, `k_pman_puzzle05`, `k_pman_puzzle06`, `k_pman_puzzle07`, `k_pman_puzzle08`, `k_pman_puzzle09`, `k_pman_puzzle10`, `k_pman_puzzle11`, `k_pman_puzzle12`, `k_pman_puzzle13`, `k_pman_puzzle14`, `k_pman_qreturn_n`, `k_pman_qreturn_s`, `k_pman_queecham2`, `k_pman_queechamp`, `k_pman_queelostp`, `k_pman_queemad_s`, `k_pman_queemad_y`, `k_pman_queenotpc`, `k_pman_queeorpcy`, `k_pman_queequit`, `k_pman_quename_n`, `k_pman_quename_s`, `k_pman_quepaid_n`, `k_pman_quepaid_s`, `k_pman_quepaid_y`, `k_pman_quetell_n`, `k_pman_quetell_s`, `k_pman_quetell_y`, `k_pman_quleave_s`, `k_pman_quleave_y`, `k_pman_race_0_02`, `k_pman_race_0_03`, `k_pman_race_0_04`, `k_pman_race_0_q1`, `k_pman_race_0_q2`, `k_pman_race_0_q3`, `k_pman_race_0_q4`, `k_pman_race_1_02`, `k_pman_race_1_03`, `k_pman_race_1_04`, `k_pman_race_2_02`, `k_pman_race_2_03`, `k_pman_race_2_04`, `k_pman_race_3_02`, `k_pman_race_3_03`, `k_pman_race_3_04`, `k_pman_race_s_01`, `k_pman_race_s_02`, `k_pman_race_s_03`, `k_pman_race_s_04`, `k_pman_race_x_02`, `k_pman_race_x_03`, `k_pman_race_x_04`, `k_pman_racecha_n`, `k_pman_racechamp`, `k_pman_racelosty`, `k_pman_racenone`, `k_pman_raceone`, `k_pman_raceonex`, `k_pman_raceset_1`, `k_pman_raceset_2`, `k_pman_raceset_3`, `k_pman_raceset_4`, `k_pman_racetwo`, `k_pman_racetwox`, `k_pman_random01`, `k_pman_random02`, `k_pman_random03`, `k_pman_random04`, `k_pman_random05`, `k_pman_random06`, `k_pman_random07`, `k_pman_random08`, `k_pman_random09`, `k_pman_random10`, `k_pman_random11`, `k_pman_random12`, `k_pman_random13`, `k_pman_random14`, `k_pman_random15`, `k_pman_random16`, `k_pman_random17`, `k_pman_random18`, `k_pman_random19`, `k_pman_random20`, `k_pman_random21`, `k_pman_random22`, `k_pman_random23`, `k_pman_random24`, `k_pman_random25`, `k_pman_random26`, `k_pman_random27`, `k_pman_random28`, `k_pman_random29`, `k_pman_random30`, `k_pman_random31`, `k_pman_random32`, `k_pman_random33`, `k_pman_reward01`, `k_pman_rodian01`, `k_pman_rodian02`, `k_pman_rodian03`, `k_pman_rodian04`, `k_pman_rodian05`, `k_pman_rodian06`, `k_pman_rodian07`, `k_pman_selapr5_d`, `k_pman_selapr_d`, `k_pman_shark01`, `k_pman_shasa01`, `k_pman_shasa02`, `k_pman_shasa03`, `k_pman_shield_9`, `k_pman_shield_d`, `k_pman_sith01`, `k_pman_sith02`, `k_pman_sithmas01`, `k_pman_sonic`, `k_pman_speed01`, `k_pman_starmap`, `k_pman_suituse`, `k_pman_sur01`, `k_pman_sur02`, `k_pman_survive02`, `k_pman_swoop01`, `k_pman_swoop02`, `k_pman_swoop03`, `k_pman_swoop04`, `k_pman_swoop05`, `k_pman_swoop06`, `k_pman_swoop07`, `k_pman_swoop08`, `k_pman_swoop09`, `k_pman_swoop10`, `k_pman_swoop11`, `k_pman_swoop12`, `k_pman_swoop13`, `k_pman_swoop14`, `k_pman_swoop15`, `k_pman_swoop16`, `k_pman_swoop17`, `k_pman_swoop18`, `k_pman_swoop19`, `k_pman_swoop20`, `k_pman_swoop21`, `k_pman_swoop22`, `k_pman_swoop23`, `k_pman_swoop24`, `k_pman_swoop25`, `k_pman_swoop26`, `k_pman_swoop27`, `k_pman_swoop28`, `k_pman_swoop29`, `k_pman_swoop30`, `k_pman_swoop31`, `k_pman_swoop32`, `k_pman_swoop33`, `k_pman_swoop34`, `k_pman_swoop35`, `k_pman_swoop36`, `k_pman_swoop37`, `k_pman_swoop38`, `k_pman_swoop39`, `k_pman_swoop40`, `k_pman_swoop41`, `k_pman_tatracern`, `k_pman_tatracery`, `k_pman_tech01`, `k_pman_trial01`, `k_pman_trial02`, `k_pman_ww`, `k_pman_ye_threat`, `k_ptar_sithdis`, `k_ptar_sithpaper`, `k_ptat_1gamewon`, `k_ptat_2gamewon`, `k_ptat_3gamewon`, `k_ptat_amb1`, `k_ptat_angrgurke`, `k_ptat_baddeal`, `k_ptat_beenkash`, `k_ptat_bounty_n`, `k_ptat_bounty_y`, `k_ptat_bountyset`, `k_ptat_bullynico`, `k_ptat_canhunt`, `k_ptat_celisdeal`, `k_ptat_checkmad`, `k_ptat_custmove`, `k_ptat_dockpaid`, `k_ptat_dockpay`, `k_ptat_dorakexit`, `k_ptat_dorakname`, `k_ptat_dorakwho`, `k_ptat_drandom09`, `k_ptat_drandom10`, `k_ptat_drandom11`, `k_ptat_drandom12`, `k_ptat_dunesea`, `k_ptat_gaffiwrit`, `k_ptat_gammbad`, `k_ptat_gammbribe`, `k_ptat_gammgone`, `k_ptat_gammhunt`, `k_ptat_gammleave`, `k_ptat_gammmean`, `k_ptat_gammwarn`, `k_ptat_gandmad`, `k_ptat_gandname`, `k_ptat_garmleave`, `k_ptat_garmname`, `k_ptat_garmwho`, `k_ptat_gateperm`, `k_ptat_givepearl`, `k_ptat_givewrit`, `k_ptat_gizkaspwn`, `k_ptat_gizkauser`, `k_ptat_gooddeal`, `k_ptat_gurkemad`, `k_ptat_has_k_map`, `k_ptat_hasgaffi`, `k_ptat_haspearl`, `k_ptat_haspermit`, `k_ptat_haswrit`, `k_ptat_huntask_n`, `k_ptat_huntask_y`, `k_ptat_koname_ch`, `k_ptat_konameset`, `k_ptat_leave`, `k_ptat_losscheck`, `k_ptat_m_office`, `k_ptat_movekrayt`, `k_ptat_namecheck`, `k_ptat_nicenico`, `k_ptat_nicofree`, `k_ptat_nicohappy`, `k_ptat_nicoinfo`, `k_ptat_niconews`, `k_ptat_nicotalk`, `k_ptat_nicowait`, `k_ptat_no_k_map`, `k_ptat_nohkinfo`, `k_ptat_nonicotlk`, `k_ptat_nopermit`, `k_ptat_norespon`, `k_ptat_nosalehk`, `k_ptat_notracer`, `k_ptat_nowrit`, `k_ptat_officemad`, `k_ptat_pazzloss`, `k_ptat_pazzwin1`, `k_ptat_pazzwin2`, `k_ptat_pazzwin3`, `k_ptat_persyuka`, `k_ptat_racechamp`, `k_ptat_racenone`, `k_ptat_raceone`, `k_ptat_racer`, `k_ptat_racer_id`, `k_ptat_racetrial`, `k_ptat_racetwo`, `k_ptat_salehk`, `k_ptat_sandper`, `k_ptat_sethunt`, `k_ptat_shariwait`, `k_ptat_sharleave`, `k_ptat_shartrust`, `k_ptat_soldhk`, `k_ptat_soldpearl`, `k_ptat_swoopdwn1`, `k_ptat_swoopdwn2`, `k_ptat_swoopdwn3`, `k_ptat_swoopdwn4`, `k_ptat_talktanis`, `k_ptat_taname_ch`, `k_ptat_tanameset`, `k_ptat_tanileave`, `k_ptat_tanisdead`, `k_ptat_tanisgave`, `k_ptat_tanismad`, `k_ptat_tatinfo_c`, `k_ptat_tatinfo_s`, `k_ptat_toldofhk`, `k_ptat_tuskpdone`, `k_ptat_yukaleave`, `k_ptat_yukanpers`, `k_ptat_yukarname`, `k_ptat_yukarwho`, `k_ptat_zoriiname`, `k_ptat_zoriiwho`, `k_ptat_zorileave`, `k_punk_cold`, `k_ren_leavlev`, `k_ren_levcapture`, `k_ren_levescape`, `k_ren_nextplanet`, `k_ren_starforge`, `k_ren_starland`, `k_ren_taris01`, `k_ren_taris02`, `k_ren_taris03`, `k_ren_turret`, `k_ren_turretld02`, `k_ren_turretload`, `k_ren_unkcrash`, `k_ren_unkturret`, `k_ren_visionland`, `k_sup_actmap`, `k_sup_bands`, `k_sup_comshots`, `k_sup_fear`, `k_sup_guiopen`, `k_sup_healing`, `k_sup_static`, `k_swg_bantforce`, `k_swg_bastdef`, `k_swg_bastevil`, `k_swg_bastforce1`, `k_swg_bastforce3`, `k_swg_bastforcex`, `k_swg_bastforcey`, `k_swg_bastila01`, `k_swg_bastila02`, `k_swg_bastila03`, `k_swg_bastila04`, `k_swg_bastila05`, `k_swg_bastila06`, `k_swg_bastila07`, `k_swg_bastila08`, `k_swg_bastila09`, `k_swg_bastila10`, `k_swg_bastila11`, `k_swg_bastila12`, `k_swg_bastila12a`, `k_swg_bastila13`, `k_swg_bastila14`, `k_swg_bastila15`, `k_swg_bastila16`, `k_swg_bastila17`, `k_swg_bastila18`, `k_swg_bastila19`, `k_swg_bastila20`, `k_swg_bastila20x`, `k_swg_bastila21`, `k_swg_bastila22`, `k_swg_bastila23`, `k_swg_bastila24`, `k_swg_bastila25`, `k_swg_bastila26`, `k_swg_bastila27`, `k_swg_bastila27x`, `k_swg_bastila28`, `k_swg_bastila29`, `k_swg_bastila30`, `k_swg_bastila31`, `k_swg_bastila32`, `k_swg_bastila99`, `k_swg_bastile17`, `k_swg_bastmovie`, `k_swg_carth01`, `k_swg_carth02`, `k_swg_carth03`, `k_swg_carth04`, `k_swg_carth05`, `k_swg_carth06`, `k_swg_carth07`, `k_swg_carth08`, `k_swg_carth09`, `k_swg_carth10`, `k_swg_carth11`, `k_swg_carth12`, `k_swg_carth13`, `k_swg_carth14`, `k_swg_carth15`, `k_swg_carth16`, `k_swg_carth17`, `k_swg_carth18`, `k_swg_carth19`, `k_swg_carth20`, `k_swg_carth21`, `k_swg_carth22`, `k_swg_carth23`, `k_swg_carth24`, `k_swg_carth25`, `k_swg_carth25x`, `k_swg_carth25y`, `k_swg_carth26`, `k_swg_carth99`, `k_swg_helena07`, `k_swg_helena08`, `k_swg_hk4700`, `k_swg_hk4701`, `k_swg_hk4702`, `k_swg_hk4703`, `k_swg_hk4704`, `k_swg_hk4705`, `k_swg_hk4706`, `k_swg_hk4707`, `k_swg_hk4708`, `k_swg_hk4709`, `k_swg_hk4710`, `k_swg_hk4711`, `k_swg_hk4712`, `k_swg_hk4712x`, `k_swg_hk4712y`, `k_swg_hk4713`, `k_swg_hk47fix`, `k_swg_hk47upg01`, `k_swg_hk47upg02`, `k_swg_hk47upg03`, `k_swg_hk47upg04`, `k_swg_jolee01`, `k_swg_jolee02`, `k_swg_jolee03`, `k_swg_jolee04`, `k_swg_jolee05`, `k_swg_jolee06`, `k_swg_jolee07`, `k_swg_jolee08`, `k_swg_jolee09`, `k_swg_jolee10`, `k_swg_jolee11`, `k_swg_jolee12`, `k_swg_jolee13`, `k_swg_jolee14`, `k_swg_jolee15`, `k_swg_jolee16`, `k_swg_jolee17`, `k_swg_jolee18`, `k_swg_jolee19`, `k_swg_jolee20`, `k_swg_jolee99`, `k_swg_joleerev`, `k_swg_replaykas`, `k_swg_replaykor`, `k_swg_replayman`, `k_swg_replaytat`, `k_swg_sasha01`, `k_swg_senni1`, `k_swg_senni2`, `k_swg_xor_def`, `k_swg_ziag01`, `k_trg_bandon`, `k_trg_calonord`, `k_trg_xorattack`, `k_unk_rancor`, `qa_calo`, `qa_lev`, `qa_map`, `qa_master`, `qa_planet`, `qa_setanim`

Size differences, largest clusters first:

| Bytes | Scripts | Examples |
| --- | --- | --- |
| -12 | 299 | `k_hbas_final`, `k_hcan_state01`, `k_hcan_state20` |
| +0 | 252 | `k_act_cmusic_off`, `k_act_cmusic_on`, `k_cheat02` |
| +273 | 109 | `k_act_canderadd`, `k_act_carthadd`, `k_act_heartoff` |
| +261 | 105 | `k_con_at_ebon`, `k_con_joleejoin`, `k_con_juhanilive` |
| -26 | 78 | `k_hcan_state02`, `k_hcan_state03`, `k_hcan_state04` |
| +247 | 63 | `k_hmis_paz`, `k_hmis_talk3lev`, `k_hmis_tardest` |
| -6 | 44 | `k_pdan_drandom02`, `k_pdan_drandom03`, `k_pdan_drandom04` |
| +267 | 34 | `k_con_bastnpm`, `k_con_candernpm`, `k_con_carthnpm` |
| -40 | 31 | `k_hcan_state07`, `k_hcan_state08`, `k_hcan_state09` |
| +241 | 26 | `k_ptat_sandper`, `k_swg_bastila03`, `k_swg_bastila05` |
| +233 | 20 | `k_con_flirtno`, `k_con_fperslow`, `k_hbanter1` |
| +227 | 19 | `k_hjuh_p28`, `k_hmis_planet`, `k_hzal_atkash1` |
| +134191 | 18 | `k_ptat_1gamewon`, `k_ptat_2gamewon`, `k_ptat_3gamewon` |
| +255 | 17 | `k_con_atdant`, `k_con_atforge`, `k_con_atkash` |
| +6 | 16 | `k_pdan_stand01`, `k_pend_door03`, `k_pman_ambient05` |
| +122046 | 15 | `k_pman_race_0_02`, `k_pman_race_0_03`, `k_pman_race_0_04` |
| +133665 | 15 | `k_ptat_bountyset`, `k_ptat_dorakname`, `k_ptat_gandmad` |
| +23671 | 14 | `k_plc_tresciv`, `k_plc_trescorhig`, `k_plc_trescorlow` |
| +75479 | 13 | `k_amb_prey_spawn`, `k_def_ambient`, `k_def_ambmob` |
| +121834 | 12 | `k_pman_casamad_y`, `k_pman_casname_n`, `k_pman_hukname_n` |
| +28775 | 11 | `k_hbanter10x`, `k_hbanter11x`, `k_hbanter1x` |
| -54 | 10 | `k_hjuh_p05`, `k_hjuh_p22`, `k_pman_murder09` |
| -16 | 10 | `k_pkas_hashilt`, `k_pkas_hassword`, `k_pkas_nohilt` |
| +253 | 10 | `k_con_bastpm`, `k_con_canderpm`, `k_con_carthpm` |
| +816 | 10 | `k_cht_p_dan`, `k_cht_p_end`, `k_cht_p_kas` |
| -96 | 9 | `k_pman_planet64`, `k_pman_planet65`, `k_pman_planet66` |
| +897 | 9 | `k_cht_n_bast`, `k_cht_n_cand`, `k_cht_n_carth` |
| +28334 | 9 | `k_pebn_loc01`, `k_pebn_loc02`, `k_pebn_loc03` |
| +28400 | 9 | `k_con_com32`, `k_con_com33`, `k_con_com34` |
| +28932 | 9 | `k_con_comactive`, `k_con_comrep`, `k_con_comver31` |
| +121314 | 9 | `k_pman_casamad_s`, `k_pman_casname_s`, `k_pman_hukname_s` |
| +134420 | 8 | `k_ptat_gammbad`, `k_ptat_huntask_n`, `k_ptat_huntask_y` |
| +24 | 7 | `k_pman_comp08`, `k_pman_door02`, `k_pman_door31` |
| +134186 | 7 | `k_ptat_bullynico`, `k_ptat_celisdeal`, `k_ptat_gurkemad` |
| +28106 | 6 | `k_act_darkhigh`, `k_act_darkmed`, `k_act_darksml` |
| +118549 | 6 | `k_pdan_rapid05`, `k_pman_rapid01`, `k_pman_rapid02` |
| +121993 | 6 | `k_pman_champ_qui`, `k_pman_huktmad_1`, `k_pman_queechamp` |
| +122026 | 6 | `k_pman_champ_f02`, `k_pman_champ_f03`, `k_pman_champ_f04` |
| +122060 | 6 | `k_pman_race_s_01`, `k_pman_racecha_n`, `k_pman_racechamp` |
| +134497 | 6 | `k_ptat_dunesea`, `k_ptat_garmleave`, `k_ptat_norespon` |
| -208 | 5 | `k_pman_murder97`, `k_pman_murder98`, `k_pman_murder99` |
| -10 | 5 | `k_hzal_beenkashy`, `k_hzal_chuundead`, `k_hzal_freyydead` |
| +122140 | 5 | `k_pman_comp11`, `k_pman_door40`, `k_pman_selapr5_d` |
| +133660 | 5 | `k_ptat_angrgurke`, `k_ptat_baddeal`, `k_ptat_gooddeal` |
| -222 | 4 | `k_pman_murder95`, `k_pman_murder9b`, `k_pman_murder9c` |
| +297 | 4 | `k_act_candstim01`, `k_act_candstim02`, `k_act_candstim03` |
| +27683 | 4 | `k_act_com32`, `k_act_com33`, `k_act_com33a` |
| +28332 | 4 | `k_swg_hk4713`, `k_swg_jolee13`, `k_swg_jolee15` |
| +28926 | 4 | `k_con_chrhigh`, `k_con_chrnormal`, `k_con_inthigh` |
| +113336 | 4 | `k_pdan_rapid01`, `k_pdan_rapid02`, `k_pdan_rapid03` |
| +116888 | 4 | `k_pdan_droid06`, `k_pdan_droid07`, `k_pdan_droid08` |
| +121184 | 4 | `k_pman_28ac_init`, `k_pman_door06`, `k_pman_door07` |
| +121979 | 4 | `k_pman_queeorpcy`, `k_pman_race_0_q2`, `k_pman_race_0_q3` |
| +122105 | 4 | `k_pman_aioff`, `k_pman_aion`, `k_pman_npcstart` |
| -20 | 3 | `k_pman_comp01`, `k_pman_puzzle03`, `k_pman_puzzle04` |
| +18 | 3 | `k_pman_argrep_d`, `k_pman_argsith_d`, `k_pman_judge1_d` |
| +209 | 3 | `k_cht_v_levi`, `k_cht_v_mast`, `k_cht_v_star` |
| +213 | 3 | `k_hzal_atkash2`, `k_hzal_notkash2`, `k_swg_bastila27` |
| +8319 | 3 | `k_ren_turretld02`, `k_ren_turretload`, `k_ren_unkturret` |
| +27545 | 3 | `k_con_perseasy`, `k_con_pershigh`, `k_con_persmed` |
| +27565 | 3 | `k_pman_planet57`, `k_pman_planet58`, `k_pman_planet59` |
| +28870 | 3 | `k_con_darkhigh`, `k_con_darklow`, `k_con_lightlow` |
| +116927 | 3 | `k_pdan_bastila11`, `k_pman_28c_sur01`, `k_pman_arg01` |
| +120961 | 3 | `k_pman_26d_triga`, `k_pman_26d_trigb`, `k_pman_26d_trigc` |
| +121198 | 3 | `k_pman_door10`, `k_pman_door13`, `k_pman_door14` |
| +121764 | 3 | `k_pman_payrace_1`, `k_pman_payrace_2`, `k_pman_payrace_3` |
| +121992 | 3 | `k_pman_comp02`, `k_pman_comp31`, `k_pman_comp32` |
| +122017 | 3 | `k_pman_lostset_n`, `k_pman_lostset_y`, `k_pman_quepaid_s` |
| +122047 | 3 | `k_pman_race_x_02`, `k_pman_race_x_03`, `k_pman_race_x_04` |
| +125531 | 3 | `k_pkas_freyydead`, `k_pkas_jaardead`, `k_pkas_jaarfreed` |
| +133549 | 3 | `k_ptat_pazzwin1`, `k_ptat_pazzwin2`, `k_ptat_pazzwin3` |
| +134183 | 3 | `k_ptat_dockpaid`, `k_ptat_nosalehk`, `k_ptat_salehk` |
| +134424 | 3 | `k_ptat_gammgone`, `k_ptat_has_k_map`, `k_ptat_no_k_map` |
| -24 | 2 | `k_hjuh_p07`, `k_pman_tatracern` |
| -2 | 2 | `k_pman_comp06`, `k_pman_comp07` |
| +279 | 2 | `k_con_classdiff`, `k_con_classsame` |
| +8317 | 2 | `k_ren_starforge`, `k_ren_taris01` |
| +27466 | 2 | `k_actq_com33`, `k_actq_com33a` |
| +27686 | 2 | `k_act_com42`, `k_act_com43` |
| +28165 | 2 | `k_com_compart`, `k_sup_star_map` |
| +28623 | 2 | `k_act_exitarea`, `k_act_exitarea2` |
| +28888 | 2 | `k_con_chrlow`, `k_con_intlow` |
| +28904 | 2 | `k_con_comver33a`, `k_con_comver34` |
| +29016 | 2 | `k_con_repair`, `k_swg_hk47fix` |
| +81106 | 2 | `k_act_walkways`, `k_act_walkways2` |
| +115281 | 2 | `k_pend_door05`, `k_pend_door09` |
| +121046 | 2 | `k_pman_init01`, `k_pman_init10` |
| +121178 | 2 | `k_pman_28d_triga`, `k_pman_survive02` |
| +121429 | 2 | `k_pman_puzzle09`, `k_pman_puzzle10` |
| +121448 | 2 | `k_pman_28d_cmp02`, `k_pman_comp04` |
| +121736 | 2 | `k_pman_puzzle07`, `k_pman_puzzle08` |
| +121739 | 2 | `k_pman_at01`, `k_pman_at02` |
| +121833 | 2 | `k_pman_puzzle06`, `k_pman_puzzle11` |
| +121851 | 2 | `k_pman_28b_at01`, `k_pman_28b_at02` |
| +122040 | 2 | `k_pman_champ_f01`, `k_pman_champ_m01` |
| +122061 | 2 | `k_pman_quepaid_n`, `k_pman_racelosty` |
| +122063 | 2 | `k_pman_queecham2`, `k_pman_queequit` |
| +122067 | 2 | `k_pman_huktmad_y`, `k_pman_missing42` |
| +122069 | 2 | `k_pman_missing25`, `k_pman_missing29` |
| +125529 | 2 | `k_pkas_chuundead`, `k_pkas_freyyhelp` |
| +133663 | 2 | `k_ptat_dockpay`, `k_ptat_soldhk` |
| +134376 | 2 | `k_ptat_gammwarn`, `k_ptat_sethunt` |
| +134380 | 2 | `k_ptat_gammleave`, `k_ptat_tanismad` |
| +134422 | 2 | `k_ptat_notracer`, `k_ptat_racer` |
| -818 | 1 | `k_sp1_generic` |
| -152 | 1 | `k_hcan_state22` |
| -138 | 1 | `k_hjuh_h12` |
| -110 | 1 | `k_hjuh_p21` |
| -102 | 1 | `k_trp_generic` |
| -90 | 1 | `k_def_buff` |
| -87 | 1 | `k_ptat_exitkrayt` |
| -82 | 1 | `k_hcan_state23` |
| -75 | 1 | `k_sup_comshots` |
| -68 | 1 | `k_hjuh_p23` |
| -58 | 1 | `k_ptat_gaffiwrit` |
| -44 | 1 | `k_ptat_gateperm` |
| -38 | 1 | `k_pman_tatracery` |
| -28 | 1 | `k_pman_suituse` |
| +8 | 1 | `k_pend_door08` |
| +70 | 1 | `k_trg_transfail1` |
| +79 | 1 | `k_sup_droid` |
| +120 | 1 | `k_pman_murdera01` |
| +177 | 1 | `k_swg_bastila22` |
| +207 | 1 | `k_swg_bastila02` |
| +211 | 1 | `k_swg_hk47upg01` |
| +239 | 1 | `k_pman_sonic` |
| +251 | 1 | `k_con_flirt` |
| +257 | 1 | `k_ptat_haspearl` |
| +259 | 1 | `k_unk_rancor` |
| +263 | 1 | `k_con_inrange` |
| +271 | 1 | `k_sup_static` |
| +275 | 1 | `k_pdan_jedi21` |
| +289 | 1 | `k_trg_senni` |
| +299 | 1 | `k_cheat_01` |
| +301 | 1 | `k_sup_bands` |
| +1420 | 1 | `k_qa` |
| +5870 | 1 | `k_ren_leavlev` |
| +6230 | 1 | `k_ren_nextplanet` |
| +6420 | 1 | `k_ren_turret` |
| +6777 | 1 | `k_ren_visionland` |
| +8282 | 1 | `k_ren_unkcrash` |
| +8285 | 1 | `k_ren_starland` |
| +8286 | 1 | `k_ren_levescape` |
| +8289 | 1 | `k_ren_taris03` |
| +8311 | 1 | `k_ren_taris02` |
| +8316 | 1 | `k_ren_levcapture` |
| +12783 | 1 | `k_act_bandatt` |
| +13261 | 1 | `k_act_caloatt` |
| +13265 | 1 | `k_act_atkonend` |
| +13289 | 1 | `k_act_xoratt` |
| +26975 | 1 | `k_pla_actmap` |
| +27382 | 1 | `k_actq_com34` |
| +27611 | 1 | `k_plc_tresdrdhig` |
| +27662 | 1 | `k_act_com31` |
| +27763 | 1 | `k_act_com44` |
| +27844 | 1 | `k_messenger_plot` |
| +27945 | 1 | `k_amb_predator01` |
| +27956 | 1 | `k_bant_trig` |
| +27982 | 1 | `k_act_electric` |
| +28066 | 1 | `k_con_neutral` |
| +28157 | 1 | `k_sup_gohawk` |
| +28194 | 1 | `k_pebn_swoop` |
| +28199 | 1 | `k_act_startgas` |
| +28283 | 1 | `k_hjuh_p07d` |
| +28309 | 1 | `k_hjuh_p15` |
| +28315 | 1 | `k_hjuh_p07l` |
| +28339 | 1 | `k_act_nospeak` |
| +28379 | 1 | `k_con_com31` |
| +28399 | 1 | `k_act_com35` |
| +28511 | 1 | `k_sup_guiopen` |
| +28588 | 1 | `k_con_dark` |
| +28620 | 1 | `k_con_light` |
| +28685 | 1 | `k_con_tokens` |
| +28714 | 1 | `k_swg_jolee18` |
| +28756 | 1 | `k_swg_jolee12` |
| +28770 | 1 | `k_swg_jolee14` |
| +28776 | 1 | `k_swg_hk4712` |
| +28810 | 1 | `k_con_chrmiddle` |
| +28854 | 1 | `k_con_talkedto` |
| +28861 | 1 | `k_amb_enemyflee` |
| +28864 | 1 | `k_con_comhack` |
| +28878 | 1 | `k_act_comp01` |
| +28885 | 1 | `k_com_comrepr` |
| +28902 | 1 | `k_con_lighthigh` |
| +28912 | 1 | `k_act_talktrue` |
| +29002 | 1 | `k_act_foeinsight` |
| +29048 | 1 | `k_def_pathfail01` |
| +29078 | 1 | `k_trg_messengers` |
| +29170 | 1 | `k_con_comver41` |
| +29190 | 1 | `k_act_com41` |
| +34345 | 1 | `k_ai_master` |
| +35315 | 1 | `k_ptar_sithdis` |
| +36379 | 1 | `k_sup_healing` |
| +36454 | 1 | `k_sup_grenade` |
| +41990 | 1 | `k_def_rependd` |
| +63013 | 1 | `k_ptat_gammenemy` |
| +75206 | 1 | `k_pdan_mand52` |
| +75451 | 1 | `k_def_calospn` |
| +75485 | 1 | `k_def_bandspn` |
| +78556 | 1 | `k_pkas_freyyspwn` |
| +78771 | 1 | `k_def_repairsp` |
| +78829 | 1 | `k_pdan_kath03` |
| +78864 | 1 | `k_pdan_stand02` |
| +78872 | 1 | `k_hen_spawn01` |
| +84407 | 1 | `k_hen_leadchng` |
| +84632 | 1 | `k_hen_retreat` |
| +84636 | 1 | `k_swg_xor_user` |
| +84650 | 1 | `k_def_repuser` |
| +85536 | 1 | `k_zon_catalog` |
| +109722 | 1 | `k_pdan_iriaz_ud` |
| +111558 | 1 | `k_trg_partyinit` |
| +112205 | 1 | `k_act_com45` |
| +112987 | 1 | `k_pman_beast_9` |
| +113064 | 1 | `k_hen_canduser` |
| +113135 | 1 | `k_def_calodef` |
| +113166 | 1 | `k_ptat_gizkauser` |
| +113210 | 1 | `k_pkas_randwalk` |
| +113246 | 1 | `k_amb_prey_ude` |
| +113248 | 1 | `k_def_userdef01` |
| +113266 | 1 | `k_pend_door06` |
| +113687 | 1 | `k_trg_partyheart` |
| +113702 | 1 | `k_zon_control` |
| +114111 | 1 | `k_pend_explode` |
| +115065 | 1 | `k_pend_door02` |
| +115286 | 1 | `k_pend_door01` |
| +115847 | 1 | `k_pman_door32` |
| +115853 | 1 | `k_pman_door20` |
| +116186 | 1 | `k_trg_stealth` |
| +116726 | 1 | `k_pdan_kath02` |
| +116734 | 1 | `k_pdan_kath04` |
| +116783 | 1 | `k_pdan_droid50` |
| +116791 | 1 | `k_pdan_belaya02` |
| +116797 | 1 | `k_pdan_droid09` |
| +116805 | 1 | `k_pdan_murder55` |
| +116829 | 1 | `k_pdan_droid02` |
| +116846 | 1 | `k_pdan_droid15` |
| +116957 | 1 | `k_pdan_elise_d` |
| +118614 | 1 | `k_pman_ww` |
| +119691 | 1 | `k_pman_26a_area` |
| +120369 | 1 | `k_pman_27_area` |
| +120637 | 1 | `k_pman_comp09` |
| +120745 | 1 | `k_pman_27_init01` |
| +121032 | 1 | `k_pman_miss01` |
| +121038 | 1 | `k_pman_init02` |
| +121057 | 1 | `k_pman_26d_area` |
| +121072 | 1 | `k_pman_raceset_3` |
| +121166 | 1 | `k_pman_misc07` |
| +121172 | 1 | `k_pman_28d_trigb` |
| +121189 | 1 | `k_pman_raceset_2` |
| +121192 | 1 | `k_pman_door11` |
| +121196 | 1 | `k_pman_raceset_1` |
| +121243 | 1 | `k_pman_raceset_4` |
| +121293 | 1 | `k_pman_notpaid_s` |
| +121424 | 1 | `k_pman_28d_cmp01` |
| +121802 | 1 | `k_pman_persuad_f` |
| +121852 | 1 | `k_pman_comp05` |
| +121923 | 1 | `k_pman_no_threat` |
| +121938 | 1 | `k_pman_planet22` |
| +121939 | 1 | `k_pdan_mand51` |
| +121951 | 1 | `k_pman_racetwox` |
| +121965 | 1 | `k_pman_raceonex` |
| +121970 | 1 | `k_pman_lostset_q` |
| +121972 | 1 | `k_pman_airlock03` |
| +121994 | 1 | `k_pman_quepaid_y` |
| +122004 | 1 | `k_pman_missing40` |
| +122016 | 1 | `k_pman_ye_threat` |
| +122028 | 1 | `k_pman_sith01` |
| +122070 | 1 | `k_pman_area_28a` |
| +122120 | 1 | `k_pman_comp10` |
| +122146 | 1 | `k_pman_firixa_d` |
| +122148 | 1 | `k_pman_door30` |
| +122154 | 1 | `k_pman_26a_aread` |
| +122164 | 1 | `k_pman_shasa01` |
| +122276 | 1 | `k_pman_missing24` |
| +122349 | 1 | `k_pman_trial02` |
| +122370 | 1 | `k_pman_airlock02` |
| +124359 | 1 | `k_pkas_freyyuser` |
| +124399 | 1 | `k_hjol_userdef01` |
| +124724 | 1 | `k_hjol_spawn01` |
| +125510 | 1 | `k_pkas_jaaraksus` |
| +125530 | 1 | `k_pkas_jaarakblt` |
| +133079 | 1 | `k_ptat_krayttran` |
| +134048 | 1 | `k_ptat_nicowait` |
| +134117 | 1 | `k_ptat_nicofree` |
| +134122 | 1 | `k_ptat_niconews` |
| +134243 | 1 | `k_ptat_talktanis` |
| +134360 | 1 | `k_ptat_gammhunt` |
| +134378 | 1 | `k_ptat_racer_id` |
| +134396 | 1 | `k_ptat_gammmean` |
| +134421 | 1 | `k_ptat_tanisgave` |
| +134423 | 1 | `k_ptat_gammbribe` |
| +134696 | 1 | `k_ptat_tanisdead` |
| +136643 | 1 | `k_ptat17aa_enter` |

## nwnrs-o1

nwnrs 0.0.1, nwnrs pack, --optimization O1

Of the 1731 outputs that differ from BioWare's bytes, 688 make the same engine calls with the same arguments and return the same result under the recording engine, 1007 behave differently, and 36 do not run.

Failed (23):

- **undeclared** (3): uses a name the shipped sources do not declare: a constant BioWare's own build had (TRASK_DOOR_JAMMED), or a Neverwinter Nights function KotOR's table lacks (StartConversation, ResistSpell)
  - `k_pend_door04`: ERROR nwnrs::pack: error=failed to compile k_pend_door04.nss: undefined identifier "TRASK_DOOR_JAMMED" (-622)
  - `nw_g0_conversat`: ERROR nwnrs::pack: error=failed to compile nw_g0_conversat.nss: undefined function "StartConversation" (-622)
  - `nw_s0_lghtnbolt`: ERROR nwnrs::pack: error=failed to compile nw_s0_lghtnbolt.nss: undefined function "ResistSpell" (-622)
- **stale-prototype** (15): BioWare compiled this against declarations that were not shipped: the shipped nwscript.nss prototype of AddPartyMember/RemovePartyMember, or k_inc_ebonhawk's forward declaration of EBO_PlayTakeOff, disagrees with the call or definition
  - `k_act_bastadd`: ERROR nwnrs::pack: error=failed to compile k_act_bastadd.nss: call to "AddPartyMember" did not supply enough parameters: 1 < 2 (-587)
  - `k_act_bastrmv`: ERROR nwnrs::pack: error=failed to compile k_act_bastrmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_canderrmv`: ERROR nwnrs::pack: error=failed to compile k_act_canderrmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_carthrmv`: ERROR nwnrs::pack: error=failed to compile k_act_carthrmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_hki47rmv`: ERROR nwnrs::pack: error=failed to compile k_act_hki47rmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_joleermv`: ERROR nwnrs::pack: error=failed to compile k_act_joleermv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_juhanirmv`: ERROR nwnrs::pack: error=failed to compile k_act_juhanirmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_makeitem`: ERROR nwnrs::pack: error=failed to compile k_act_makeitem.nss: function "EBO_PlayTakeOff" declaration differs from previous declaration (-617)
  - `k_act_missionrmv`: ERROR nwnrs::pack: error=failed to compile k_act_missionrmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_t3m3rmv`: ERROR nwnrs::pack: error=failed to compile k_act_t3m3rmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_zaalrmv`: ERROR nwnrs::pack: error=failed to compile k_act_zaalrmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_con_makeitem`: ERROR nwnrs::pack: error=failed to compile k_con_makeitem.nss: function "EBO_PlayTakeOff" declaration differs from previous declaration (-617)
  - `k_pebn_galaxy`: ERROR nwnrs::pack: error=failed to compile k_pebn_galaxy.nss: function "EBO_PlayTakeOff" declaration differs from previous declaration (-617)
  - `k_sup_galaxmap`: ERROR nwnrs::pack: error=failed to compile k_sup_galaxmap.nss: function "EBO_PlayTakeOff" declaration differs from previous declaration (-617)
  - `k_sup_galaxymap`: ERROR nwnrs::pack: error=failed to compile k_sup_galaxymap.nss: function "EBO_PlayTakeOff" declaration differs from previous declaration (-617)
- **not-a-script** (5): an include file with no entry point; there is nothing to compile
  - `k_inc_generic`: ERROR nwnrs::pack: error=failed to compile k_inc_generic.nss: script must define StartingConditional (-5182)
  - `k_inc_gensupport`: ERROR nwnrs::pack: error=failed to compile k_inc_gensupport.nss: script must define StartingConditional (-5182)
  - `k_inc_switch`: ERROR nwnrs::pack: error=failed to compile k_inc_switch.nss: script must define StartingConditional (-5182)
  - `k_inc_unk`: ERROR nwnrs::pack: error=failed to compile k_inc_unk.nss: script must define StartingConditional (-5182)
  - `k_inc_walkways`: ERROR nwnrs::pack: error=failed to compile k_inc_walkways.nss: script must define StartingConditional (-5182)

Different output (1731), by cause:

- **stale-include** (73): BioWare's bytecode was compiled before an include it uses last changed: the first difference is in an include's globals or functions (k_inc_utility's SW_CONSTANT_* globals, k_inc_generic's GN_TalentMasterRoutine, among others)
  `k_act_atkonend`, `k_act_caloatt`, `k_act_com31`, `k_act_com32`, `k_act_com33`, `k_act_com33a`, `k_act_com34`, `k_act_com35`, `k_act_com41`, `k_act_com42`, `k_act_com43`, `k_act_com45`, `k_act_comp01`, `k_act_electric`, `k_act_startgas`, `k_act_xoratt`, `k_actq_com33`, `k_actq_com33a`, `k_actq_com34`, `k_amb_enemyflee`, `k_amb_predator01`, `k_com_compart`, `k_com_comrepr`, `k_con_com31`, `k_con_com32`, `k_con_com33`, `k_con_com34`, `k_con_com35`, `k_con_com41`, `k_con_com42`, `k_con_com43`, `k_con_com44`, `k_con_com45`, `k_con_comactive`, `k_con_comhack`, `k_con_comrep`, `k_con_comver31`, `k_con_comver32`, `k_con_comver33`, `k_con_comver33a`, `k_con_comver34`, `k_con_comver41`, `k_con_comver42`, `k_con_comver43`, `k_con_comver44`, `k_con_comver45`, `k_con_tokens`, `k_def_rependd`, `k_pla_actmap`, `k_plc_tresciv`, `k_plc_trescorhig`, `k_plc_trescorlow`, `k_plc_trescormid`, `k_plc_tresdrdhig`, `k_plc_tresdrdlow`, `k_plc_tresdrdmid`, `k_plc_tresmilhig`, `k_plc_tresmillow`, `k_plc_tresmilmid`, `k_plc_tresrakat`, `k_plc_tresshahig`, `k_plc_tresshalow`, `k_plc_tresshamid`, `k_plc_tressndppl`, `k_ptat17aa_enter`, `k_ptat_gammenemy`, `k_sp1_generic`, `k_sup_star_map`, `k_trg_messengers`, `k_trg_partyheart`, `k_trg_stealth`, `k_zon_catalog`, `k_zon_control`
- **stale-bytecode** (8): BioWare's bytecode was compiled from an earlier version of this script's own source: the first difference is in the script, and it is a different call, constant or condition
  `k_ptat_exitkrayt`, `k_qa`, `k_sup_droid`, `k_sup_gohawk`, `k_sup_grenade`, `k_trg_senni`, `k_trg_transfail1`, `k_trp_generic`
- **conversation-twelve** (3): BioWare compiled this against the shipped twelve-argument ActionStartConversation, unlike most of its scripts. Identical with the shipped declarations
  `k_ai_master`, `k_pman_27_init01`, `k_trg_transfail`
- **conversation-eleven** (51): BioWare compiled this when ActionStartConversation took eleven arguments; the shipped declaration has twelve. Identical with --declarations bioware
  `k_bant_trig`, `k_def_bandspn`, `k_def_calodef`, `k_hjol_userdef01`, `k_pdan_elise_d`, `k_pdan_rapid01`, `k_pdan_rapid02`, `k_pdan_rapid03`, `k_pdan_rapid04`, `k_pdan_rapid05`, `k_pend_door01`, `k_pend_door02`, `k_pend_door03`, `k_pend_door08`, `k_pkas_freyyuser`, `k_pman_26a_area`, `k_pman_28ac_init`, `k_pman_at01`, `k_pman_at02`, `k_pman_door03`, `k_pman_door06`, `k_pman_door07`, `k_pman_door08`, `k_pman_door10`, `k_pman_door11`, `k_pman_door13`, `k_pman_door14`, `k_pman_door20`, `k_pman_door30`, `k_pman_init01`, `k_pman_init02`, `k_pman_init03`, `k_pman_init10`, `k_pman_judge1_d`, `k_pman_obj_asc`, `k_pman_obj_asc2`, `k_pman_obj_ascc`, `k_pman_obj_ascc2`, `k_pman_obj_ascn`, `k_pman_osc`, `k_pman_rapid01`, `k_pman_rapid02`, `k_pman_rapid03`, `k_pman_rapid04`, `k_pman_rapid05`, `k_pman_sitspeak`, `k_pman_survive01`, `k_ptar_sitharm`, `k_ptat_krayttran`, `k_swg_xor_user`, `k_trg_partyinit`
- **no known cause** (1596): `k_act_bandatt`, `k_act_canderadd`, `k_act_candstim01`, `k_act_candstim02`, `k_act_candstim03`, `k_act_carthadd`, `k_act_cmusic_off`, `k_act_cmusic_on`, `k_act_com44`, `k_act_darkhigh`, `k_act_darkmed`, `k_act_darksml`, `k_act_exitarea`, `k_act_exitarea2`, `k_act_foeinsight`, `k_act_heartoff`, `k_act_hearton`, `k_act_hk47add`, `k_act_hk47simul`, `k_act_joleeadd`, `k_act_juhaniadd`, `k_act_leaverun`, `k_act_lighthigh`, `k_act_lightmed`, `k_act_lightsml`, `k_act_mispaz`, `k_act_missionadd`, `k_act_nospeak`, `k_act_noswitch`, `k_act_npctalk`, `k_act_t3m3add`, `k_act_talktrue`, `k_act_walkways`, `k_act_walkways2`, `k_act_xordie`, `k_act_zaaladd`, `k_amb_prey_spawn`, `k_amb_prey_ude`, `k_cheat02`, `k_cheat_01`, `k_cht_n_bast`, `k_cht_n_cand`, `k_cht_n_carth`, `k_cht_n_hk47`, `k_cht_n_jolee`, `k_cht_n_juhani`, `k_cht_n_mission`, `k_cht_n_screen`, `k_cht_n_t3m4`, `k_cht_n_zaalbar`, `k_cht_p_dan`, `k_cht_p_end`, `k_cht_p_kas`, `k_cht_p_kor`, `k_cht_p_lev`, `k_cht_p_man`, `k_cht_p_sta`, `k_cht_p_tar`, `k_cht_p_tat`, `k_cht_p_unk`, `k_cht_v_levi`, `k_cht_v_mast`, `k_cht_v_star`, `k_con_at_ebon`, `k_con_atdant`, `k_con_atforge`, `k_con_atkash`, `k_con_atkorr`, `k_con_atlev`, `k_con_atmann`, `k_con_atspire`, `k_con_attaris`, `k_con_attat`, `k_con_atunk`, `k_con_bastnpm`, `k_con_bastpm`, `k_con_candernpm`, `k_con_canderpm`, `k_con_candstim01`, `k_con_carthnpm`, `k_con_carthpm`, `k_con_chrhigh`, `k_con_chrlow`, `k_con_chrmiddle`, `k_con_chrnormal`, `k_con_classdiff`, `k_con_classsame`, `k_con_dark`, `k_con_darkhigh`, `k_con_darklow`, `k_con_flirt`, `k_con_flirtno`, `k_con_fpershigh`, `k_con_fperslow`, `k_con_heal`, `k_con_hk47npm`, `k_con_hk47pm`, `k_con_inrange`, `k_con_inthigh`, `k_con_intlow`, `k_con_intnormal`, `k_con_isfemale`, `k_con_isjedi`, `k_con_ismale`, `k_con_isscout`, `k_con_issmug`, `k_con_issoldier`, `k_con_joleejoin`, `k_con_joleenpm`, `k_con_joleepm`, `k_con_juhanilive`, `k_con_juhaninpm`, `k_con_juhanipm`, `k_con_light`, `k_con_lighthigh`, `k_con_lightlow`, `k_con_missionnpm`, `k_con_missionpm`, `k_con_neutral`, `k_con_npctalk`, `k_con_perseasy`, `k_con_pershigh`, `k_con_persmed`, `k_con_repair`, `k_con_t3m3npm`, `k_con_t3m3pm`, `k_con_talkedto`, `k_con_zaalnpm`, `k_con_zaalpm`, `k_def_ambient`, `k_def_ambmob`, `k_def_attacked01`, `k_def_blocked01`, `k_def_buff`, `k_def_calospn`, `k_def_combend01`, `k_def_damage01`, `k_def_death01`, `k_def_dialogue01`, `k_def_disturb01`, `k_def_endconv`, `k_def_grenspn`, `k_def_heartbt01`, `k_def_pathfail01`, `k_def_percept01`, `k_def_repairsp`, `k_def_repuser`, `k_def_spawn01`, `k_def_spawndead`, `k_def_spellat01`, `k_def_userdef01`, `k_hbanter1`, `k_hbanter10`, `k_hbanter10x`, `k_hbanter11`, `k_hbanter11x`, `k_hbanter1x`, `k_hbanter2`, `k_hbanter2x`, `k_hbanter3`, `k_hbanter3x`, `k_hbanter4`, `k_hbanter4x`, `k_hbanter5`, `k_hbanter5x`, `k_hbanter6`, `k_hbanter6x`, `k_hbanter7`, `k_hbanter7x`, `k_hbanter8`, `k_hbanter8x`, `k_hbanter9`, `k_hbanter9x`, `k_hbas_final`, `k_hbas_playkash`, `k_hbas_playkorr`, `k_hbas_playman`, `k_hbas_playtat`, `k_hcan_state01`, `k_hcan_state02`, `k_hcan_state03`, `k_hcan_state04`, `k_hcan_state05`, `k_hcan_state06`, `k_hcan_state07`, `k_hcan_state08`, `k_hcan_state09`, `k_hcan_state10`, `k_hcan_state11`, `k_hcan_state12`, `k_hcan_state13`, `k_hcan_state14`, `k_hcan_state15`, `k_hcan_state16`, `k_hcan_state17`, `k_hcan_state18`, `k_hcan_state19`, `k_hcan_state20`, `k_hcan_state21`, `k_hcan_state22`, `k_hcan_state23`, `k_hcan_state24`, `k_hcan_state25`, `k_hcan_state26`, `k_hcan_state27`, `k_hcan_state28`, `k_hcan_state29`, `k_hcan_state30`, `k_hcan_state31`, `k_hcan_state32`, `k_hcan_state33`, `k_hcan_state34`, `k_hcan_state35`, `k_hcan_state36`, `k_hcan_state37`, `k_hcan_state38`, `k_hcan_state39`, `k_hcan_state40`, `k_hcan_state41`, `k_hcan_state42`, `k_hcan_state43`, `k_hcan_state44`, `k_hcan_state45`, `k_hen_attacked01`, `k_hen_blocked01`, `k_hen_candspwn`, `k_hen_canduser`, `k_hen_combend01`, `k_hen_damage01`, `k_hen_dialogue01`, `k_hen_enter5m`, `k_hen_exit5m`, `k_hen_heartbt01`, `k_hen_leadchng`, `k_hen_percept01`, `k_hen_retreat`, `k_hen_spawn01`, `k_hjol_spawn01`, `k_hjuh_com01`, `k_hjuh_d01`, `k_hjuh_d02`, `k_hjuh_h01`, `k_hjuh_h01x`, `k_hjuh_h02`, `k_hjuh_h03`, `k_hjuh_h04`, `k_hjuh_h05`, `k_hjuh_h06`, `k_hjuh_h07`, `k_hjuh_h08`, `k_hjuh_h09`, `k_hjuh_h10`, `k_hjuh_h10x`, `k_hjuh_h11`, `k_hjuh_h12`, `k_hjuh_h13`, `k_hjuh_p01`, `k_hjuh_p02`, `k_hjuh_p03`, `k_hjuh_p04`, `k_hjuh_p05`, `k_hjuh_p06`, `k_hjuh_p07`, `k_hjuh_p07d`, `k_hjuh_p07l`, `k_hjuh_p08`, `k_hjuh_p09`, `k_hjuh_p10`, `k_hjuh_p10x`, `k_hjuh_p10y`, `k_hjuh_p11`, `k_hjuh_p12`, `k_hjuh_p13`, `k_hjuh_p14`, `k_hjuh_p15`, `k_hjuh_p16`, `k_hjuh_p17`, `k_hjuh_p18`, `k_hjuh_p19`, `k_hjuh_p20`, `k_hjuh_p21`, `k_hjuh_p22`, `k_hjuh_p23`, `k_hjuh_p24`, `k_hjuh_p25`, `k_hjuh_p26`, `k_hjuh_p27`, `k_hjuh_p28`, `k_hjuh_s01`, `k_hjuh_s02`, `k_hjuh_s03`, `k_hjuh_s04`, `k_hjuh_s05`, `k_hjuh_s06`, `k_hjuh_s07`, `k_hjuh_w01`, `k_hjuh_w02`, `k_hjuh_w03`, `k_hjuh_w04`, `k_hjuh_w05`, `k_hjuh_w06`, `k_hjuh_w07`, `k_hjuh_w08`, `k_hjuh_w09`, `k_hjuh_w10`, `k_hmis_misangry`, `k_hmis_misangryx`, `k_hmis_paz`, `k_hmis_planet`, `k_hmis_talk0`, `k_hmis_talk0+`, `k_hmis_talk1`, `k_hmis_talk10`, `k_hmis_talk10lev`, `k_hmis_talk10x`, `k_hmis_talk11`, `k_hmis_talk11lev`, `k_hmis_talk11x`, `k_hmis_talk12`, `k_hmis_talk12x`, `k_hmis_talk13x`, `k_hmis_talk14`, `k_hmis_talk14x`, `k_hmis_talk15`, `k_hmis_talk15x`, `k_hmis_talk1lev`, `k_hmis_talk1x`, `k_hmis_talk2x`, `k_hmis_talk3`, `k_hmis_talk3lev`, `k_hmis_talk3x`, `k_hmis_talk4x`, `k_hmis_talk5`, `k_hmis_talk5x`, `k_hmis_talk6`, `k_hmis_talk6-8`, `k_hmis_talk6lev`, `k_hmis_talk6x`, `k_hmis_talk7`, `k_hmis_talk7lev`, `k_hmis_talk7x`, `k_hmis_talk8`, `k_hmis_talk8x`, `k_hmis_talk9`, `k_hmis_talk99`, `k_hmis_talk99x`, `k_hmis_talk9x`, `k_hmis_talkrev`, `k_hmis_tardest`, `k_hmis_tardestx`, `k_hmis_zaalbar`, `k_hxor_state01`, `k_hxor_state02`, `k_hxor_state03`, `k_hzal_atkash1`, `k_hzal_atkash2`, `k_hzal_beenkashy`, `k_hzal_chuundead`, `k_hzal_freyydead`, `k_hzal_notkash1`, `k_hzal_notkash2`, `k_hzal_talk0`, `k_hzal_talk_1up`, `k_hzal_talkmad_c`, `k_hzal_talkmad_s`, `k_hzal_talkset_1`, `k_messenger_plot`, `k_pdan_anger01`, `k_pdan_anger02`, `k_pdan_anger03`, `k_pdan_anger04`, `k_pdan_anger05`, `k_pdan_anger06`, `k_pdan_anger07`, `k_pdan_anger08`, `k_pdan_anger09`, `k_pdan_anger10`, `k_pdan_anger11`, `k_pdan_anger12`, `k_pdan_anger13`, `k_pdan_anger14`, `k_pdan_anger15`, `k_pdan_anger16`, `k_pdan_anger17`, `k_pdan_anger18`, `k_pdan_anger19`, `k_pdan_anger20`, `k_pdan_bastial05`, `k_pdan_bastila01`, `k_pdan_bastila02`, `k_pdan_bastila03`, `k_pdan_bastila04`, `k_pdan_bastila05`, `k_pdan_bastila06`, `k_pdan_bastila07`, `k_pdan_bastila08`, `k_pdan_bastila09`, `k_pdan_bastila10`, `k_pdan_bastila11`, `k_pdan_belaya01`, `k_pdan_belaya02`, `k_pdan_cander01`, `k_pdan_cander02`, `k_pdan_cander03`, `k_pdan_carth01`, `k_pdan_carth02`, `k_pdan_carth03`, `k_pdan_carth04`, `k_pdan_casus01`, `k_pdan_casus02`, `k_pdan_casus03`, `k_pdan_casus04`, `k_pdan_casus05`, `k_pdan_destself`, `k_pdan_drandom02`, `k_pdan_drandom03`, `k_pdan_drandom04`, `k_pdan_drandom05`, `k_pdan_drandom06`, `k_pdan_drandom07`, `k_pdan_drandom08`, `k_pdan_droid01`, `k_pdan_droid02`, `k_pdan_droid03`, `k_pdan_droid04`, `k_pdan_droid05`, `k_pdan_droid06`, `k_pdan_droid07`, `k_pdan_droid08`, `k_pdan_droid09`, `k_pdan_droid10`, `k_pdan_droid11`, `k_pdan_droid12`, `k_pdan_droid13`, `k_pdan_droid14`, `k_pdan_droid15`, `k_pdan_droid50`, `k_pdan_iriaz_sp`, `k_pdan_iriaz_ud`, `k_pdan_jedi01`, `k_pdan_jedi02`, `k_pdan_jedi03`, `k_pdan_jedi04`, `k_pdan_jedi05`, `k_pdan_jedi06`, `k_pdan_jedi07`, `k_pdan_jedi08`, `k_pdan_jedi09`, `k_pdan_jedi10`, `k_pdan_jedi11`, `k_pdan_jedi12`, `k_pdan_jedi13`, `k_pdan_jedi14`, `k_pdan_jedi15`, `k_pdan_jedi16`, `k_pdan_jedi17`, `k_pdan_jedi18`, `k_pdan_jedi19`, `k_pdan_jedi20`, `k_pdan_jedi21`, `k_pdan_jedi22`, `k_pdan_jedi23`, `k_pdan_jedi24`, `k_pdan_jedi25`, `k_pdan_jedi26`, `k_pdan_jedi27`, `k_pdan_jedi28`, `k_pdan_jedi29`, `k_pdan_jedi30`, `k_pdan_jedi31`, `k_pdan_jedi32`, `k_pdan_jedi33`, `k_pdan_jedi34`, `k_pdan_jedi35`, `k_pdan_jedi36`, `k_pdan_jedi37`, `k_pdan_jedi38`, `k_pdan_jedi39`, `k_pdan_jedi40`, `k_pdan_juhani01`, `k_pdan_juhani02`, `k_pdan_juhani03`, `k_pdan_juhani04`, `k_pdan_juhani05`, `k_pdan_juhani06`, `k_pdan_juhani07`, `k_pdan_juhani08`, `k_pdan_juhani09`, `k_pdan_juhani10`, `k_pdan_juhani11`, `k_pdan_juhani12`, `k_pdan_juhani15`, `k_pdan_juhani16`, `k_pdan_juhani17`, `k_pdan_juhani18`, `k_pdan_juhani19`, `k_pdan_juhani20`, `k_pdan_kath01`, `k_pdan_kath02`, `k_pdan_kath03`, `k_pdan_kath04`, `k_pdan_mand01`, `k_pdan_mand02`, `k_pdan_mand03`, `k_pdan_mand04`, `k_pdan_mand05`, `k_pdan_mand06`, `k_pdan_mand07`, `k_pdan_mand08`, `k_pdan_mand09`, `k_pdan_mand10`, `k_pdan_mand11`, `k_pdan_mand12`, `k_pdan_mand51`, `k_pdan_mand52`, `k_pdan_mission01`, `k_pdan_mission02`, `k_pdan_murder01`, `k_pdan_murder02`, `k_pdan_murder03`, `k_pdan_murder04`, `k_pdan_murder05`, `k_pdan_murder06`, `k_pdan_murder07`, `k_pdan_murder08`, `k_pdan_murder09`, `k_pdan_murder10`, `k_pdan_murder11`, `k_pdan_murder12`, `k_pdan_murder13`, `k_pdan_murder14`, `k_pdan_murder15`, `k_pdan_murder16`, `k_pdan_murder17`, `k_pdan_murder18`, `k_pdan_murder19`, `k_pdan_murder20`, `k_pdan_murder21`, `k_pdan_murder22`, `k_pdan_murder23`, `k_pdan_murder24`, `k_pdan_murder25`, `k_pdan_murder26`, `k_pdan_murder27`, `k_pdan_murder28`, `k_pdan_murder29`, `k_pdan_murder30`, `k_pdan_murder31`, `k_pdan_murder32`, `k_pdan_murder33`, `k_pdan_murder34`, `k_pdan_murder35`, `k_pdan_murder36`, `k_pdan_murder37`, `k_pdan_murder38`, `k_pdan_murder39`, `k_pdan_murder40`, `k_pdan_murder41`, `k_pdan_murder42`, `k_pdan_murder43`, `k_pdan_murder44`, `k_pdan_murder45`, `k_pdan_murder46`, `k_pdan_murder47`, `k_pdan_murder48`, `k_pdan_murder49`, `k_pdan_murder50`, `k_pdan_murder51`, `k_pdan_murder52`, `k_pdan_murder53`, `k_pdan_murder54`, `k_pdan_murder55`, `k_pdan_murder56`, `k_pdan_murder57`, `k_pdan_murder58`, `k_pdan_murder59`, `k_pdan_murder60`, `k_pdan_murder61`, `k_pdan_murder62`, `k_pdan_murder63`, `k_pdan_murder64`, `k_pdan_murder65`, `k_pdan_murder66`, `k_pdan_murder67`, `k_pdan_murder68`, `k_pdan_murder69`, `k_pdan_murder70`, `k_pdan_murder71`, `k_pdan_murder72`, `k_pdan_murder73`, `k_pdan_murder74`, `k_pdan_murder75`, `k_pdan_murder76`, `k_pdan_planet01`, `k_pdan_planet02`, `k_pdan_planet03`, `k_pdan_planet04`, `k_pdan_planet05`, `k_pdan_planet06`, `k_pdan_planet08`, `k_pdan_planet12`, `k_pdan_planet14`, `k_pdan_planet15`, `k_pdan_planet16`, `k_pdan_romance01`, `k_pdan_romance02`, `k_pdan_romance03`, `k_pdan_romance04`, `k_pdan_romance05`, `k_pdan_romance06`, `k_pdan_romance07`, `k_pdan_romance08`, `k_pdan_romance09`, `k_pdan_romance10`, `k_pdan_romance11`, `k_pdan_romance12`, `k_pdan_romance13`, `k_pdan_romance14`, `k_pdan_romance15`, `k_pdan_romance16`, `k_pdan_romance17`, `k_pdan_romance18`, `k_pdan_romance19`, `k_pdan_romance20`, `k_pdan_romance21`, `k_pdan_romance22`, `k_pdan_romance23`, `k_pdan_romance24`, `k_pdan_romance25`, `k_pdan_romance26`, `k_pdan_romance27`, `k_pdan_romance28`, `k_pdan_romance29`, `k_pdan_romance30`, `k_pdan_romance31`, `k_pdan_romance32`, `k_pdan_romance33`, `k_pdan_romance34`, `k_pdan_romance35`, `k_pdan_romance36`, `k_pdan_romance37`, `k_pdan_romance38`, `k_pdan_romance39`, `k_pdan_romance40`, `k_pdan_romance41`, `k_pdan_romance42`, `k_pdan_stand01`, `k_pdan_stand02`, `k_pdan_stand03`, `k_pebn_ctaris01`, `k_pebn_ctaris02`, `k_pebn_ctaris03`, `k_pebn_ctaris04`, `k_pebn_ctaris05`, `k_pebn_ctaris06`, `k_pebn_ctaris07`, `k_pebn_ctaris08`, `k_pebn_ctaris09`, `k_pebn_gotaris`, `k_pebn_loc01`, `k_pebn_loc02`, `k_pebn_loc03`, `k_pebn_loc04`, `k_pebn_loc05`, `k_pebn_loc06`, `k_pebn_loc07`, `k_pebn_loc08`, `k_pebn_loc09`, `k_pebn_swoop`, `k_pend_door05`, `k_pend_door06`, `k_pend_door07`, `k_pend_door09`, `k_pend_explode`, `k_pend_rumble01`, `k_phkd_fighter`, `k_pkas_chuundead`, `k_pkas_enemyfac`, `k_pkas_freyydead`, `k_pkas_freyyhelp`, `k_pkas_freyyspwn`, `k_pkas_hashilt`, `k_pkas_hassword`, `k_pkas_heart_on`, `k_pkas_jaarakblt`, `k_pkas_jaaraksus`, `k_pkas_jaardead`, `k_pkas_jaarfreed`, `k_pkas_nohilt`, `k_pkas_nosword`, `k_pkas_pause3sec`, `k_pkas_randwalk`, `k_pkas_takehilt`, `k_pman_26a_aread`, `k_pman_26c_area`, `k_pman_26d_area`, `k_pman_26d_triga`, `k_pman_26d_trigb`, `k_pman_26d_trigc`, `k_pman_27_area`, `k_pman_28b_at01`, `k_pman_28b_at02`, `k_pman_28c_sur01`, `k_pman_28d_cmp01`, `k_pman_28d_cmp02`, `k_pman_28d_triga`, `k_pman_28d_trigb`, `k_pman_aioff`, `k_pman_aion`, `k_pman_airlock01`, `k_pman_airlock02`, `k_pman_airlock03`, `k_pman_amb_9`, `k_pman_ambient01`, `k_pman_ambient02`, `k_pman_ambient03`, `k_pman_ambient04`, `k_pman_ambient05`, `k_pman_ambient06`, `k_pman_ambient07`, `k_pman_ambient08`, `k_pman_area_28a`, `k_pman_arg01`, `k_pman_argrep_d`, `k_pman_argsith_d`, `k_pman_beast_9`, `k_pman_beast_d`, `k_pman_body01`, `k_pman_casamad_s`, `k_pman_casamad_y`, `k_pman_casname_n`, `k_pman_casname_s`, `k_pman_champ_f01`, `k_pman_champ_f02`, `k_pman_champ_f03`, `k_pman_champ_f04`, `k_pman_champ_m01`, `k_pman_champ_m02`, `k_pman_champ_m03`, `k_pman_champ_m04`, `k_pman_champ_qui`, `k_pman_comp01`, `k_pman_comp02`, `k_pman_comp03`, `k_pman_comp04`, `k_pman_comp05`, `k_pman_comp06`, `k_pman_comp07`, `k_pman_comp08`, `k_pman_comp09`, `k_pman_comp10`, `k_pman_comp11`, `k_pman_comp30`, `k_pman_comp31`, `k_pman_comp32`, `k_pman_comp33`, `k_pman_comp34`, `k_pman_comp35`, `k_pman_door01`, `k_pman_door02`, `k_pman_door04`, `k_pman_door05`, `k_pman_door12`, `k_pman_door31`, `k_pman_door32`, `k_pman_door40`, `k_pman_false`, `k_pman_firixa_d`, `k_pman_guard01`, `k_pman_hukname_n`, `k_pman_hukname_s`, `k_pman_huktmad_1`, `k_pman_huktmad_y`, `k_pman_ignore`, `k_pman_ignus01`, `k_pman_locked01`, `k_pman_locked02`, `k_pman_lostset_n`, `k_pman_lostset_q`, `k_pman_lostset_y`, `k_pman_main01`, `k_pman_misc01`, `k_pman_misc02`, `k_pman_misc03`, `k_pman_misc04`, `k_pman_misc05`, `k_pman_misc06`, `k_pman_misc07`, `k_pman_misc08`, `k_pman_misc09`, `k_pman_misc10`, `k_pman_misc11`, `k_pman_misc12`, `k_pman_misc13`, `k_pman_misc14`, `k_pman_misc15`, `k_pman_misc16`, `k_pman_misc17`, `k_pman_misc18`, `k_pman_misc19`, `k_pman_misc20`, `k_pman_misc21`, `k_pman_misc22`, `k_pman_misc23`, `k_pman_misc24`, `k_pman_misc25`, `k_pman_misc26`, `k_pman_misc27`, `k_pman_misc28`, `k_pman_misc29`, `k_pman_misc30`, `k_pman_misc31`, `k_pman_misc32`, `k_pman_misc33`, `k_pman_misc34`, `k_pman_misc35`, `k_pman_misc36`, `k_pman_misc37`, `k_pman_misc38`, `k_pman_misc39`, `k_pman_misc40`, `k_pman_misc41`, `k_pman_misc42`, `k_pman_misc43`, `k_pman_misc44`, `k_pman_misc45`, `k_pman_misc46`, `k_pman_misc47`, `k_pman_misc48`, `k_pman_misc49`, `k_pman_misc50`, `k_pman_misc51`, `k_pman_misc52`, `k_pman_misc53`, `k_pman_misc54`, `k_pman_misc55`, `k_pman_misc56`, `k_pman_misc57`, `k_pman_misc58`, `k_pman_misc59`, `k_pman_misc60`, `k_pman_misc61`, `k_pman_misc62`, `k_pman_misc70`, `k_pman_misc71`, `k_pman_misc72`, `k_pman_misc73`, `k_pman_misc74`, `k_pman_misc75`, `k_pman_misc76`, `k_pman_miss01`, `k_pman_missing01`, `k_pman_missing02`, `k_pman_missing03`, `k_pman_missing04`, `k_pman_missing05`, `k_pman_missing06`, `k_pman_missing07`, `k_pman_missing08`, `k_pman_missing09`, `k_pman_missing10`, `k_pman_missing11`, `k_pman_missing12`, `k_pman_missing13`, `k_pman_missing14`, `k_pman_missing15`, `k_pman_missing16`, `k_pman_missing17`, `k_pman_missing18`, `k_pman_missing19`, `k_pman_missing20`, `k_pman_missing21`, `k_pman_missing22`, `k_pman_missing23`, `k_pman_missing24`, `k_pman_missing25`, `k_pman_missing26`, `k_pman_missing27`, `k_pman_missing28`, `k_pman_missing29`, `k_pman_missing30`, `k_pman_missing31`, `k_pman_missing32`, `k_pman_missing33`, `k_pman_missing34`, `k_pman_missing35`, `k_pman_missing36`, `k_pman_missing37`, `k_pman_missing38`, `k_pman_missing39`, `k_pman_missing40`, `k_pman_missing41`, `k_pman_missing42`, `k_pman_missing43`, `k_pman_murder01`, `k_pman_murder02`, `k_pman_murder03`, `k_pman_murder04`, `k_pman_murder05`, `k_pman_murder06`, `k_pman_murder07`, `k_pman_murder08`, `k_pman_murder09`, `k_pman_murder10`, `k_pman_murder11`, `k_pman_murder12`, `k_pman_murder13`, `k_pman_murder14`, `k_pman_murder15`, `k_pman_murder16`, `k_pman_murder17`, `k_pman_murder18`, `k_pman_murder19`, `k_pman_murder20`, `k_pman_murder21`, `k_pman_murder22`, `k_pman_murder23`, `k_pman_murder24`, `k_pman_murder25`, `k_pman_murder26`, `k_pman_murder27`, `k_pman_murder28`, `k_pman_murder29`, `k_pman_murder30`, `k_pman_murder31`, `k_pman_murder32`, `k_pman_murder33`, `k_pman_murder34`, `k_pman_murder35`, `k_pman_murder36`, `k_pman_murder37`, `k_pman_murder38`, `k_pman_murder39`, `k_pman_murder40`, `k_pman_murder41`, `k_pman_murder42`, `k_pman_murder43`, `k_pman_murder44`, `k_pman_murder45`, `k_pman_murder46`, `k_pman_murder47`, `k_pman_murder48`, `k_pman_murder49`, `k_pman_murder50`, `k_pman_murder51`, `k_pman_murder52`, `k_pman_murder53`, `k_pman_murder54`, `k_pman_murder55`, `k_pman_murder56`, `k_pman_murder57`, `k_pman_murder58`, `k_pman_murder59`, `k_pman_murder60`, `k_pman_murder61`, `k_pman_murder64`, `k_pman_murder65`, `k_pman_murder66`, `k_pman_murder67`, `k_pman_murder68`, `k_pman_murder69`, `k_pman_murder70`, `k_pman_murder71`, `k_pman_murder72`, `k_pman_murder73`, `k_pman_murder74`, `k_pman_murder75`, `k_pman_murder76`, `k_pman_murder77`, `k_pman_murder78`, `k_pman_murder79`, `k_pman_murder80`, `k_pman_murder81`, `k_pman_murder82`, `k_pman_murder83`, `k_pman_murder84`, `k_pman_murder85`, `k_pman_murder86`, `k_pman_murder87`, `k_pman_murder88`, `k_pman_murder89`, `k_pman_murder90`, `k_pman_murder91`, `k_pman_murder92`, `k_pman_murder93`, `k_pman_murder94`, `k_pman_murder95`, `k_pman_murder96`, `k_pman_murder97`, `k_pman_murder98`, `k_pman_murder99`, `k_pman_murder9a`, `k_pman_murder9b`, `k_pman_murder9c`, `k_pman_murder9d`, `k_pman_murder9e`, `k_pman_murder9f`, `k_pman_murder9g`, `k_pman_murder9h`, `k_pman_murder9i`, `k_pman_murder9j`, `k_pman_murder9k`, `k_pman_murder9l`, `k_pman_murder9m`, `k_pman_murder9n`, `k_pman_murder9o`, `k_pman_murder9p`, `k_pman_murder9q`, `k_pman_murder9r`, `k_pman_murder9s`, `k_pman_murder9t`, `k_pman_murder9u`, `k_pman_murder9v`, `k_pman_murder9w`, `k_pman_murder9x`, `k_pman_murder9y`, `k_pman_murder9z`, `k_pman_murdera01`, `k_pman_murdera02`, `k_pman_murdera03`, `k_pman_murdera04`, `k_pman_murdera05`, `k_pman_murdera06`, `k_pman_murdera07`, `k_pman_murdera08`, `k_pman_murdera09`, `k_pman_murdera11`, `k_pman_no_threat`, `k_pman_notpaid_c`, `k_pman_notpaid_n`, `k_pman_notpaid_s`, `k_pman_npcstart`, `k_pman_npcstop`, `k_pman_patrol_d`, `k_pman_pause2sec`, `k_pman_payrace_1`, `k_pman_payrace_2`, `k_pman_payrace_3`, `k_pman_pazaak01`, `k_pman_pazaak02`, `k_pman_pazaak03`, `k_pman_pazaak04`, `k_pman_pazaak05`, `k_pman_pazaak06`, `k_pman_pazaak07`, `k_pman_pazaak08`, `k_pman_pazaak09`, `k_pman_pazaak10`, `k_pman_pazaak11`, `k_pman_persuad_f`, `k_pman_persuad_s`, `k_pman_persuad_y`, `k_pman_planet01`, `k_pman_planet02`, `k_pman_planet03`, `k_pman_planet04`, `k_pman_planet05`, `k_pman_planet06`, `k_pman_planet07`, `k_pman_planet08`, `k_pman_planet09`, `k_pman_planet10`, `k_pman_planet11`, `k_pman_planet12`, `k_pman_planet13`, `k_pman_planet14`, `k_pman_planet15`, `k_pman_planet16`, `k_pman_planet17`, `k_pman_planet18`, `k_pman_planet19`, `k_pman_planet20`, `k_pman_planet21`, `k_pman_planet22`, `k_pman_planet23`, `k_pman_planet24`, `k_pman_planet25`, `k_pman_planet26`, `k_pman_planet27`, `k_pman_planet28`, `k_pman_planet29`, `k_pman_planet30`, `k_pman_planet31`, `k_pman_planet32`, `k_pman_planet33`, `k_pman_planet34`, `k_pman_planet35`, `k_pman_planet36`, `k_pman_planet37`, `k_pman_planet38`, `k_pman_planet39`, `k_pman_planet40`, `k_pman_planet41`, `k_pman_planet42`, `k_pman_planet43`, `k_pman_planet44`, `k_pman_planet45`, `k_pman_planet46`, `k_pman_planet47`, `k_pman_planet48`, `k_pman_planet49`, `k_pman_planet50`, `k_pman_planet51`, `k_pman_planet52`, `k_pman_planet53`, `k_pman_planet54`, `k_pman_planet55`, `k_pman_planet56`, `k_pman_planet57`, `k_pman_planet58`, `k_pman_planet59`, `k_pman_planet60`, `k_pman_planet61`, `k_pman_planet62`, `k_pman_planet63`, `k_pman_planet64`, `k_pman_planet65`, `k_pman_planet66`, `k_pman_planet67`, `k_pman_planet68`, `k_pman_planet69`, `k_pman_planet70`, `k_pman_planet71`, `k_pman_planet72`, `k_pman_planet73`, `k_pman_planet74`, `k_pman_planet75`, `k_pman_planet76`, `k_pman_planet77`, `k_pman_planet78`, `k_pman_planet79`, `k_pman_planet80`, `k_pman_planet81`, `k_pman_planet82`, `k_pman_planet83`, `k_pman_planet84`, `k_pman_planet85`, `k_pman_planet86`, `k_pman_planet87`, `k_pman_port01`, `k_pman_puzzle01`, `k_pman_puzzle02`, `k_pman_puzzle03`, `k_pman_puzzle04`, `k_pman_puzzle05`, `k_pman_puzzle06`, `k_pman_puzzle07`, `k_pman_puzzle08`, `k_pman_puzzle09`, `k_pman_puzzle10`, `k_pman_puzzle11`, `k_pman_puzzle12`, `k_pman_puzzle13`, `k_pman_puzzle14`, `k_pman_qreturn_n`, `k_pman_qreturn_s`, `k_pman_queecham2`, `k_pman_queechamp`, `k_pman_queelostp`, `k_pman_queemad_s`, `k_pman_queemad_y`, `k_pman_queenotpc`, `k_pman_queeorpcy`, `k_pman_queequit`, `k_pman_quename_n`, `k_pman_quename_s`, `k_pman_quepaid_n`, `k_pman_quepaid_s`, `k_pman_quepaid_y`, `k_pman_quetell_n`, `k_pman_quetell_s`, `k_pman_quetell_y`, `k_pman_quleave_s`, `k_pman_quleave_y`, `k_pman_race_0_02`, `k_pman_race_0_03`, `k_pman_race_0_04`, `k_pman_race_0_q1`, `k_pman_race_0_q2`, `k_pman_race_0_q3`, `k_pman_race_0_q4`, `k_pman_race_1_02`, `k_pman_race_1_03`, `k_pman_race_1_04`, `k_pman_race_2_02`, `k_pman_race_2_03`, `k_pman_race_2_04`, `k_pman_race_3_02`, `k_pman_race_3_03`, `k_pman_race_3_04`, `k_pman_race_s_01`, `k_pman_race_s_02`, `k_pman_race_s_03`, `k_pman_race_s_04`, `k_pman_race_x_02`, `k_pman_race_x_03`, `k_pman_race_x_04`, `k_pman_racecha_n`, `k_pman_racechamp`, `k_pman_racelosty`, `k_pman_racenone`, `k_pman_raceone`, `k_pman_raceonex`, `k_pman_raceset_1`, `k_pman_raceset_2`, `k_pman_raceset_3`, `k_pman_raceset_4`, `k_pman_racetwo`, `k_pman_racetwox`, `k_pman_random01`, `k_pman_random02`, `k_pman_random03`, `k_pman_random04`, `k_pman_random05`, `k_pman_random06`, `k_pman_random07`, `k_pman_random08`, `k_pman_random09`, `k_pman_random10`, `k_pman_random11`, `k_pman_random12`, `k_pman_random13`, `k_pman_random14`, `k_pman_random15`, `k_pman_random16`, `k_pman_random17`, `k_pman_random18`, `k_pman_random19`, `k_pman_random20`, `k_pman_random21`, `k_pman_random22`, `k_pman_random23`, `k_pman_random24`, `k_pman_random25`, `k_pman_random26`, `k_pman_random27`, `k_pman_random28`, `k_pman_random29`, `k_pman_random30`, `k_pman_random31`, `k_pman_random32`, `k_pman_random33`, `k_pman_reward01`, `k_pman_rodian01`, `k_pman_rodian02`, `k_pman_rodian03`, `k_pman_rodian04`, `k_pman_rodian05`, `k_pman_rodian06`, `k_pman_rodian07`, `k_pman_selapr5_d`, `k_pman_selapr_d`, `k_pman_shark01`, `k_pman_shasa01`, `k_pman_shasa02`, `k_pman_shasa03`, `k_pman_shield_9`, `k_pman_shield_d`, `k_pman_sith01`, `k_pman_sith02`, `k_pman_sithmas01`, `k_pman_sonic`, `k_pman_speed01`, `k_pman_starmap`, `k_pman_suituse`, `k_pman_sur01`, `k_pman_sur02`, `k_pman_survive02`, `k_pman_swoop01`, `k_pman_swoop02`, `k_pman_swoop03`, `k_pman_swoop04`, `k_pman_swoop05`, `k_pman_swoop06`, `k_pman_swoop07`, `k_pman_swoop08`, `k_pman_swoop09`, `k_pman_swoop10`, `k_pman_swoop11`, `k_pman_swoop12`, `k_pman_swoop13`, `k_pman_swoop14`, `k_pman_swoop15`, `k_pman_swoop16`, `k_pman_swoop17`, `k_pman_swoop18`, `k_pman_swoop19`, `k_pman_swoop20`, `k_pman_swoop21`, `k_pman_swoop22`, `k_pman_swoop23`, `k_pman_swoop24`, `k_pman_swoop25`, `k_pman_swoop26`, `k_pman_swoop27`, `k_pman_swoop28`, `k_pman_swoop29`, `k_pman_swoop30`, `k_pman_swoop31`, `k_pman_swoop32`, `k_pman_swoop33`, `k_pman_swoop34`, `k_pman_swoop35`, `k_pman_swoop36`, `k_pman_swoop37`, `k_pman_swoop38`, `k_pman_swoop39`, `k_pman_swoop40`, `k_pman_swoop41`, `k_pman_tatracern`, `k_pman_tatracery`, `k_pman_tech01`, `k_pman_trial01`, `k_pman_trial02`, `k_pman_ww`, `k_pman_ye_threat`, `k_ptar_sithdis`, `k_ptar_sithpaper`, `k_ptat_1gamewon`, `k_ptat_2gamewon`, `k_ptat_3gamewon`, `k_ptat_amb1`, `k_ptat_angrgurke`, `k_ptat_baddeal`, `k_ptat_beenkash`, `k_ptat_bounty_n`, `k_ptat_bounty_y`, `k_ptat_bountyset`, `k_ptat_bullynico`, `k_ptat_canhunt`, `k_ptat_celisdeal`, `k_ptat_checkmad`, `k_ptat_custmove`, `k_ptat_dockpaid`, `k_ptat_dockpay`, `k_ptat_dorakexit`, `k_ptat_dorakname`, `k_ptat_dorakwho`, `k_ptat_drandom09`, `k_ptat_drandom10`, `k_ptat_drandom11`, `k_ptat_drandom12`, `k_ptat_dunesea`, `k_ptat_gaffiwrit`, `k_ptat_gammbad`, `k_ptat_gammbribe`, `k_ptat_gammgone`, `k_ptat_gammhunt`, `k_ptat_gammleave`, `k_ptat_gammmean`, `k_ptat_gammwarn`, `k_ptat_gandmad`, `k_ptat_gandname`, `k_ptat_garmleave`, `k_ptat_garmname`, `k_ptat_garmwho`, `k_ptat_gateperm`, `k_ptat_givepearl`, `k_ptat_givewrit`, `k_ptat_gizkaspwn`, `k_ptat_gizkauser`, `k_ptat_gooddeal`, `k_ptat_gurkemad`, `k_ptat_has_k_map`, `k_ptat_hasgaffi`, `k_ptat_haspearl`, `k_ptat_haspermit`, `k_ptat_haswrit`, `k_ptat_huntask_n`, `k_ptat_huntask_y`, `k_ptat_koname_ch`, `k_ptat_konameset`, `k_ptat_leave`, `k_ptat_losscheck`, `k_ptat_m_office`, `k_ptat_movekrayt`, `k_ptat_namecheck`, `k_ptat_nicenico`, `k_ptat_nicofree`, `k_ptat_nicohappy`, `k_ptat_nicoinfo`, `k_ptat_niconews`, `k_ptat_nicotalk`, `k_ptat_nicowait`, `k_ptat_no_k_map`, `k_ptat_nohkinfo`, `k_ptat_nonicotlk`, `k_ptat_nopermit`, `k_ptat_norespon`, `k_ptat_nosalehk`, `k_ptat_notracer`, `k_ptat_nowrit`, `k_ptat_officemad`, `k_ptat_pazzloss`, `k_ptat_pazzwin1`, `k_ptat_pazzwin2`, `k_ptat_pazzwin3`, `k_ptat_persyuka`, `k_ptat_racechamp`, `k_ptat_racenone`, `k_ptat_raceone`, `k_ptat_racer`, `k_ptat_racer_id`, `k_ptat_racetrial`, `k_ptat_racetwo`, `k_ptat_salehk`, `k_ptat_sandper`, `k_ptat_sethunt`, `k_ptat_shariwait`, `k_ptat_sharleave`, `k_ptat_shartrust`, `k_ptat_soldhk`, `k_ptat_soldpearl`, `k_ptat_swoopdwn1`, `k_ptat_swoopdwn2`, `k_ptat_swoopdwn3`, `k_ptat_swoopdwn4`, `k_ptat_talktanis`, `k_ptat_taname_ch`, `k_ptat_tanameset`, `k_ptat_tanileave`, `k_ptat_tanisdead`, `k_ptat_tanisgave`, `k_ptat_tanismad`, `k_ptat_tatinfo_c`, `k_ptat_tatinfo_s`, `k_ptat_toldofhk`, `k_ptat_tuskpdone`, `k_ptat_yukaleave`, `k_ptat_yukanpers`, `k_ptat_yukarname`, `k_ptat_yukarwho`, `k_ptat_zoriiname`, `k_ptat_zoriiwho`, `k_ptat_zorileave`, `k_punk_cold`, `k_ren_leavlev`, `k_ren_levcapture`, `k_ren_levescape`, `k_ren_nextplanet`, `k_ren_starforge`, `k_ren_starland`, `k_ren_taris01`, `k_ren_taris02`, `k_ren_taris03`, `k_ren_turret`, `k_ren_turretld02`, `k_ren_turretload`, `k_ren_unkcrash`, `k_ren_unkturret`, `k_ren_visionland`, `k_sup_actmap`, `k_sup_bands`, `k_sup_comshots`, `k_sup_fear`, `k_sup_guiopen`, `k_sup_healing`, `k_sup_static`, `k_swg_bantforce`, `k_swg_bastdef`, `k_swg_bastevil`, `k_swg_bastforce1`, `k_swg_bastforce3`, `k_swg_bastforcex`, `k_swg_bastforcey`, `k_swg_bastila01`, `k_swg_bastila02`, `k_swg_bastila03`, `k_swg_bastila04`, `k_swg_bastila05`, `k_swg_bastila06`, `k_swg_bastila07`, `k_swg_bastila08`, `k_swg_bastila09`, `k_swg_bastila10`, `k_swg_bastila11`, `k_swg_bastila12`, `k_swg_bastila12a`, `k_swg_bastila13`, `k_swg_bastila14`, `k_swg_bastila15`, `k_swg_bastila16`, `k_swg_bastila17`, `k_swg_bastila18`, `k_swg_bastila19`, `k_swg_bastila20`, `k_swg_bastila20x`, `k_swg_bastila21`, `k_swg_bastila22`, `k_swg_bastila23`, `k_swg_bastila24`, `k_swg_bastila25`, `k_swg_bastila26`, `k_swg_bastila27`, `k_swg_bastila27x`, `k_swg_bastila28`, `k_swg_bastila29`, `k_swg_bastila30`, `k_swg_bastila31`, `k_swg_bastila32`, `k_swg_bastila99`, `k_swg_bastile17`, `k_swg_bastmovie`, `k_swg_carth01`, `k_swg_carth02`, `k_swg_carth03`, `k_swg_carth04`, `k_swg_carth05`, `k_swg_carth06`, `k_swg_carth07`, `k_swg_carth08`, `k_swg_carth09`, `k_swg_carth10`, `k_swg_carth11`, `k_swg_carth12`, `k_swg_carth13`, `k_swg_carth14`, `k_swg_carth15`, `k_swg_carth16`, `k_swg_carth17`, `k_swg_carth18`, `k_swg_carth19`, `k_swg_carth20`, `k_swg_carth21`, `k_swg_carth22`, `k_swg_carth23`, `k_swg_carth24`, `k_swg_carth25`, `k_swg_carth25x`, `k_swg_carth25y`, `k_swg_carth26`, `k_swg_carth99`, `k_swg_helena07`, `k_swg_helena08`, `k_swg_hk4700`, `k_swg_hk4701`, `k_swg_hk4702`, `k_swg_hk4703`, `k_swg_hk4704`, `k_swg_hk4705`, `k_swg_hk4706`, `k_swg_hk4707`, `k_swg_hk4708`, `k_swg_hk4709`, `k_swg_hk4710`, `k_swg_hk4711`, `k_swg_hk4712`, `k_swg_hk4712x`, `k_swg_hk4712y`, `k_swg_hk4713`, `k_swg_hk47fix`, `k_swg_hk47upg01`, `k_swg_hk47upg02`, `k_swg_hk47upg03`, `k_swg_hk47upg04`, `k_swg_jolee01`, `k_swg_jolee02`, `k_swg_jolee03`, `k_swg_jolee04`, `k_swg_jolee05`, `k_swg_jolee06`, `k_swg_jolee07`, `k_swg_jolee08`, `k_swg_jolee09`, `k_swg_jolee10`, `k_swg_jolee11`, `k_swg_jolee12`, `k_swg_jolee13`, `k_swg_jolee14`, `k_swg_jolee15`, `k_swg_jolee16`, `k_swg_jolee17`, `k_swg_jolee18`, `k_swg_jolee19`, `k_swg_jolee20`, `k_swg_jolee99`, `k_swg_joleerev`, `k_swg_replaykas`, `k_swg_replaykor`, `k_swg_replayman`, `k_swg_replaytat`, `k_swg_sasha01`, `k_swg_senni1`, `k_swg_senni2`, `k_swg_xor_def`, `k_swg_ziag01`, `k_trg_bandon`, `k_trg_calonord`, `k_trg_xorattack`, `k_unk_rancor`, `qa_calo`, `qa_lev`, `qa_map`, `qa_master`, `qa_planet`, `qa_setanim`

Size differences, largest clusters first:

| Bytes | Scripts | Examples |
| --- | --- | --- |
| -12 | 417 | `k_con_at_ebon`, `k_con_joleejoin`, `k_con_juhanilive` |
| +0 | 375 | `k_act_canderadd`, `k_act_carthadd`, `k_act_cmusic_off` |
| -26 | 143 | `k_hcan_state02`, `k_hcan_state03`, `k_hcan_state04` |
| -6 | 110 | `k_cht_v_levi`, `k_cht_v_mast`, `k_cht_v_star` |
| -40 | 63 | `k_act_talktrue`, `k_con_flirtno`, `k_con_fperslow` |
| -88 | 37 | `k_pman_casamad_y`, `k_pman_casname_n`, `k_pman_hukname_n` |
| -36 | 37 | `k_con_chrhigh`, `k_con_chrnormal`, `k_con_inthigh` |
| -266 | 32 | `k_pman_casamad_s`, `k_pman_casname_s`, `k_pman_hukname_s` |
| -32 | 31 | `k_pdan_droid15`, `k_pman_missing25`, `k_pman_missing29` |
| -46 | 21 | `k_hjuh_p28`, `k_hmis_planet`, `k_hzal_atkash1` |
| -18 | 19 | `k_con_atdant`, `k_con_atforge`, `k_con_atkash` |
| +6 | 19 | `k_con_classdiff`, `k_con_classsame`, `k_messenger_plot` |
| -50 | 18 | `k_pman_race_0_02`, `k_pman_race_0_03`, `k_pman_race_0_04` |
| -224 | 15 | `k_amb_prey_spawn`, `k_def_ambient`, `k_def_ambmob` |
| -56 | 15 | `k_con_darkhigh`, `k_con_darklow`, `k_con_lightlow` |
| -4866 | 14 | `k_plc_tresciv`, `k_plc_trescorhig`, `k_plc_trescorlow` |
| -20 | 13 | `k_con_bastpm`, `k_con_canderpm`, `k_con_carthpm` |
| -16 | 12 | `k_def_calodef`, `k_pkas_hashilt`, `k_pkas_hassword` |
| +24 | 12 | `k_act_candstim01`, `k_act_candstim02`, `k_act_candstim03` |
| -70 | 11 | `k_def_repairsp`, `k_pman_champ_f02`, `k_pman_champ_f03` |
| +12 | 11 | `k_cht_p_dan`, `k_cht_p_end`, `k_cht_p_kas` |
| -370 | 10 | `k_pdan_rapid01`, `k_pdan_rapid02`, `k_pdan_rapid03` |
| -96 | 10 | `k_pman_planet64`, `k_pman_planet65`, `k_pman_planet66` |
| -54 | 10 | `k_hjuh_p05`, `k_hjuh_p22`, `k_pman_murder09` |
| +10 | 10 | `k_cht_n_bast`, `k_cht_n_cand`, `k_cht_n_carth` |
| -230 | 9 | `k_pebn_loc01`, `k_pebn_loc02`, `k_pebn_loc03` |
| -147 | 9 | `k_con_com32`, `k_con_com33`, `k_con_com34` |
| -2 | 9 | `k_pman_comp05`, `k_pman_comp06`, `k_pman_comp07` |
| +58 | 9 | `k_con_comactive`, `k_con_comrep`, `k_con_comver31` |
| -208 | 7 | `k_pman_28d_cmp02`, `k_pman_comp04`, `k_pman_murder97` |
| +18 | 7 | `k_pman_28b_at01`, `k_pman_28b_at02`, `k_pman_airlock03` |
| -363 | 6 | `k_act_com32`, `k_act_com33`, `k_act_com33a` |
| -232 | 6 | `k_con_neutral`, `k_pman_28d_cmp01`, `k_swg_hk4713` |
| -132 | 6 | `k_act_darkhigh`, `k_act_darkmed`, `k_act_darksml` |
| -108 | 6 | `k_pman_payrace_1`, `k_pman_payrace_2`, `k_pman_payrace_3` |
| -38 | 6 | `k_act_exitarea`, `k_act_exitarea2`, `k_con_repair` |
| -10 | 6 | `k_con_inrange`, `k_hzal_beenkashy`, `k_hzal_chuundead` |
| -278 | 5 | `k_hjol_spawn01`, `k_pman_door10`, `k_pman_door13` |
| -60 | 5 | `k_con_chrmiddle`, `k_hzal_atkash2`, `k_hzal_notkash2` |
| -310 | 4 | `k_pman_init01`, `k_pman_init10`, `k_pman_misc07` |
| -292 | 4 | `k_pman_28ac_init`, `k_pman_door06`, `k_pman_door07` |
| -264 | 4 | `k_con_perseasy`, `k_con_pershigh`, `k_con_persmed` |
| -222 | 4 | `k_pman_murder95`, `k_pman_murder9b`, `k_pman_murder9c` |
| -196 | 4 | `k_act_walkways`, `k_act_walkways2`, `k_pman_ww` |
| -94 | 4 | `k_ptat_dockpaid`, `k_ptat_nosalehk`, `k_ptat_salehk` |
| -64 | 4 | `k_pdan_kath03`, `k_pkas_freyyspwn`, `k_pman_planet22` |
| -44 | 4 | `k_pman_comp02`, `k_pman_comp31`, `k_pman_comp32` |
| -28 | 4 | `k_hen_canduser`, `k_pkas_randwalk`, `k_pman_suituse` |
| -300 | 3 | `k_ptat_pazzwin1`, `k_ptat_pazzwin2`, `k_ptat_pazzwin3` |
| -294 | 3 | `k_pman_26d_triga`, `k_pman_26d_trigb`, `k_pman_26d_trigc` |
| -244 | 3 | `k_pman_planet57`, `k_pman_planet58`, `k_pman_planet59` |
| -52 | 3 | `k_act_foeinsight`, `k_hen_spawn01`, `k_pend_door02` |
| -42 | 3 | `k_con_chrlow`, `k_con_intlow`, `k_con_lighthigh` |
| +2 | 3 | `k_pdan_jedi21`, `k_pend_door01`, `k_pman_door30` |
| +8 | 3 | `k_amb_prey_ude`, `k_pend_door08`, `k_pman_26a_aread` |
| -6634 | 2 | `k_act_caloatt`, `k_def_rependd` |
| -350 | 2 | `k_act_com31`, `k_sup_comshots` |
| -345 | 2 | `k_actq_com33`, `k_actq_com33a` |
| -304 | 2 | `k_pend_explode`, `k_pman_28d_trigb` |
| -298 | 2 | `k_pman_28d_triga`, `k_pman_survive02` |
| -160 | 2 | `k_sup_star_map`, `k_swg_jolee18` |
| -152 | 2 | `k_hcan_state22`, `k_ren_nextplanet` |
| -146 | 2 | `k_ren_leavlev`, `k_sup_guiopen` |
| -128 | 2 | `k_com_compart`, `k_ptat_nicowait` |
| -110 | 2 | `k_con_light`, `k_hjuh_p21` |
| -98 | 2 | `k_pman_racetwox`, `k_swg_hk4712` |
| -84 | 2 | `k_act_nospeak`, `k_pman_raceonex` |
| -76 | 2 | `k_pman_area_28a`, `k_pman_no_threat` |
| -58 | 2 | `k_pdan_stand02`, `k_ptat_gaffiwrit` |
| -24 | 2 | `k_hjuh_p07`, `k_pman_tatracern` |
| -4 | 2 | `k_pman_27_init01`, `k_swg_hk47upg01` |
| +16 | 2 | `k_ptat_krayttran`, `k_trg_senni` |
| +30 | 2 | `k_con_comver33a`, `k_con_comver34` |
| +32 | 2 | `k_ptar_sitharm`, `k_zon_catalog` |
| -8708 | 1 | `k_ai_master` |
| -7054 | 1 | `k_act_bandatt` |
| -6664 | 1 | `k_ptat_gammenemy` |
| -6630 | 1 | `k_act_atkonend` |
| -6606 | 1 | `k_act_xoratt` |
| -1913 | 1 | `k_sp1_generic` |
| -926 | 1 | `k_plc_tresdrdhig` |
| -852 | 1 | `k_bant_trig` |
| -830 | 1 | `k_pend_door06` |
| -668 | 1 | `k_trg_partyinit` |
| -372 | 1 | `k_pkas_freyyuser` |
| -354 | 1 | `k_hjol_userdef01` |
| -353 | 1 | `k_sup_gohawk` |
| -351 | 1 | `k_actq_com34` |
| -324 | 1 | `k_pman_miss01` |
| -320 | 1 | `k_pman_27_area` |
| -318 | 1 | `k_pman_init02` |
| -317 | 1 | `k_trp_generic` |
| -296 | 1 | `k_pman_door32` |
| -290 | 1 | `k_pman_door20` |
| -286 | 1 | `k_act_com44` |
| -284 | 1 | `k_pman_door11` |
| -276 | 1 | `k_pman_raceset_2` |
| -272 | 1 | `k_pman_raceset_4` |
| -252 | 1 | `k_def_calospn` |
| -240 | 1 | `k_pman_26a_area` |
| -218 | 1 | `k_def_bandspn` |
| -214 | 1 | `k_pman_26d_area` |
| -209 | 1 | `k_act_com45` |
| -192 | 1 | `k_pebn_swoop` |
| -180 | 1 | `k_pdan_iriaz_ud` |
| -178 | 1 | `k_ren_visionland` |
| -168 | 1 | `k_amb_predator01` |
| -156 | 1 | `k_hjuh_p07d` |
| -142 | 1 | `k_hjuh_p07l` |
| -138 | 1 | `k_hjuh_h12` |
| -137 | 1 | `k_act_com35` |
| -134 | 1 | `k_con_com31` |
| -130 | 1 | `k_hjuh_p15` |
| -125 | 1 | `k_con_tokens` |
| -124 | 1 | `k_con_dark` |
| -120 | 1 | `k_pman_persuad_f` |
| -118 | 1 | `k_swg_jolee12` |
| -114 | 1 | `k_hen_leadchng` |
| -104 | 1 | `k_swg_jolee14` |
| -90 | 1 | `k_def_buff` |
| -87 | 1 | `k_ptat_exitkrayt` |
| -82 | 1 | `k_hcan_state23` |
| -74 | 1 | `k_ptat_talktanis` |
| -72 | 1 | `k_ptat_gizkauser` |
| -68 | 1 | `k_hjuh_p23` |
| -66 | 1 | `k_swg_bastila02` |
| -34 | 1 | `k_pman_sonic` |
| -23 | 1 | `k_pla_actmap` |
| -22 | 1 | `k_con_flirt` |
| -14 | 1 | `k_unk_rancor` |
| +20 | 1 | `k_sup_healing` |
| +22 | 1 | `k_cheat_01` |
| +28 | 1 | `k_sup_bands` |
| +43 | 1 | `k_trg_stealth` |
| +44 | 1 | `k_pman_airlock02` |
| +60 | 1 | `k_act_startgas` |
| +70 | 1 | `k_trg_transfail1` |
| +80 | 1 | `k_amb_enemyflee` |
| +94 | 1 | `k_com_comrepr` |
| +98 | 1 | `k_act_comp01` |
| +106 | 1 | `k_act_electric` |
| +107 | 1 | `k_con_comhack` |
| +116 | 1 | `k_con_comver41` |
| +120 | 1 | `k_pman_murdera01` |
| +136 | 1 | `k_act_com41` |
| +141 | 1 | `k_trg_messengers` |
| +268 | 1 | `k_trg_partyheart` |
| +1205 | 1 | `k_qa` |
| +2093 | 1 | `k_ptat17aa_enter` |

## nwnrs-o2

nwnrs 0.0.1, nwnrs pack, --optimization O2

Of the 1731 outputs that differ from BioWare's bytes, 688 make the same engine calls with the same arguments and return the same result under the recording engine, 1007 behave differently, and 36 do not run.

Failed (23):

- **undeclared** (3): uses a name the shipped sources do not declare: a constant BioWare's own build had (TRASK_DOOR_JAMMED), or a Neverwinter Nights function KotOR's table lacks (StartConversation, ResistSpell)
  - `k_pend_door04`: ERROR nwnrs::pack: error=failed to compile k_pend_door04.nss: undefined identifier "TRASK_DOOR_JAMMED" (-622)
  - `nw_g0_conversat`: ERROR nwnrs::pack: error=failed to compile nw_g0_conversat.nss: undefined function "StartConversation" (-622)
  - `nw_s0_lghtnbolt`: ERROR nwnrs::pack: error=failed to compile nw_s0_lghtnbolt.nss: undefined function "ResistSpell" (-622)
- **stale-prototype** (15): BioWare compiled this against declarations that were not shipped: the shipped nwscript.nss prototype of AddPartyMember/RemovePartyMember, or k_inc_ebonhawk's forward declaration of EBO_PlayTakeOff, disagrees with the call or definition
  - `k_act_bastadd`: ERROR nwnrs::pack: error=failed to compile k_act_bastadd.nss: call to "AddPartyMember" did not supply enough parameters: 1 < 2 (-587)
  - `k_act_bastrmv`: ERROR nwnrs::pack: error=failed to compile k_act_bastrmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_canderrmv`: ERROR nwnrs::pack: error=failed to compile k_act_canderrmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_carthrmv`: ERROR nwnrs::pack: error=failed to compile k_act_carthrmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_hki47rmv`: ERROR nwnrs::pack: error=failed to compile k_act_hki47rmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_joleermv`: ERROR nwnrs::pack: error=failed to compile k_act_joleermv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_juhanirmv`: ERROR nwnrs::pack: error=failed to compile k_act_juhanirmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_makeitem`: ERROR nwnrs::pack: error=failed to compile k_act_makeitem.nss: function "EBO_PlayTakeOff" declaration differs from previous declaration (-617)
  - `k_act_missionrmv`: ERROR nwnrs::pack: error=failed to compile k_act_missionrmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_t3m3rmv`: ERROR nwnrs::pack: error=failed to compile k_act_t3m3rmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_zaalrmv`: ERROR nwnrs::pack: error=failed to compile k_act_zaalrmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_con_makeitem`: ERROR nwnrs::pack: error=failed to compile k_con_makeitem.nss: function "EBO_PlayTakeOff" declaration differs from previous declaration (-617)
  - `k_pebn_galaxy`: ERROR nwnrs::pack: error=failed to compile k_pebn_galaxy.nss: function "EBO_PlayTakeOff" declaration differs from previous declaration (-617)
  - `k_sup_galaxmap`: ERROR nwnrs::pack: error=failed to compile k_sup_galaxmap.nss: function "EBO_PlayTakeOff" declaration differs from previous declaration (-617)
  - `k_sup_galaxymap`: ERROR nwnrs::pack: error=failed to compile k_sup_galaxymap.nss: function "EBO_PlayTakeOff" declaration differs from previous declaration (-617)
- **not-a-script** (5): an include file with no entry point; there is nothing to compile
  - `k_inc_generic`: ERROR nwnrs::pack: error=failed to compile k_inc_generic.nss: script must define StartingConditional (-5182)
  - `k_inc_gensupport`: ERROR nwnrs::pack: error=failed to compile k_inc_gensupport.nss: script must define StartingConditional (-5182)
  - `k_inc_switch`: ERROR nwnrs::pack: error=failed to compile k_inc_switch.nss: script must define StartingConditional (-5182)
  - `k_inc_unk`: ERROR nwnrs::pack: error=failed to compile k_inc_unk.nss: script must define StartingConditional (-5182)
  - `k_inc_walkways`: ERROR nwnrs::pack: error=failed to compile k_inc_walkways.nss: script must define StartingConditional (-5182)

Different output (1731), by cause:

- **stale-include** (73): BioWare's bytecode was compiled before an include it uses last changed: the first difference is in an include's globals or functions (k_inc_utility's SW_CONSTANT_* globals, k_inc_generic's GN_TalentMasterRoutine, among others)
  `k_act_atkonend`, `k_act_caloatt`, `k_act_com31`, `k_act_com32`, `k_act_com33`, `k_act_com33a`, `k_act_com34`, `k_act_com35`, `k_act_com41`, `k_act_com42`, `k_act_com43`, `k_act_com45`, `k_act_comp01`, `k_act_electric`, `k_act_startgas`, `k_act_xoratt`, `k_actq_com33`, `k_actq_com33a`, `k_actq_com34`, `k_amb_enemyflee`, `k_amb_predator01`, `k_com_compart`, `k_com_comrepr`, `k_con_com31`, `k_con_com32`, `k_con_com33`, `k_con_com34`, `k_con_com35`, `k_con_com41`, `k_con_com42`, `k_con_com43`, `k_con_com44`, `k_con_com45`, `k_con_comactive`, `k_con_comhack`, `k_con_comrep`, `k_con_comver31`, `k_con_comver32`, `k_con_comver33`, `k_con_comver33a`, `k_con_comver34`, `k_con_comver41`, `k_con_comver42`, `k_con_comver43`, `k_con_comver44`, `k_con_comver45`, `k_con_tokens`, `k_def_rependd`, `k_pla_actmap`, `k_plc_tresciv`, `k_plc_trescorhig`, `k_plc_trescorlow`, `k_plc_trescormid`, `k_plc_tresdrdhig`, `k_plc_tresdrdlow`, `k_plc_tresdrdmid`, `k_plc_tresmilhig`, `k_plc_tresmillow`, `k_plc_tresmilmid`, `k_plc_tresrakat`, `k_plc_tresshahig`, `k_plc_tresshalow`, `k_plc_tresshamid`, `k_plc_tressndppl`, `k_ptat17aa_enter`, `k_ptat_gammenemy`, `k_sp1_generic`, `k_sup_star_map`, `k_trg_messengers`, `k_trg_partyheart`, `k_trg_stealth`, `k_zon_catalog`, `k_zon_control`
- **stale-bytecode** (8): BioWare's bytecode was compiled from an earlier version of this script's own source: the first difference is in the script, and it is a different call, constant or condition
  `k_ptat_exitkrayt`, `k_qa`, `k_sup_droid`, `k_sup_gohawk`, `k_sup_grenade`, `k_trg_senni`, `k_trg_transfail1`, `k_trp_generic`
- **conversation-twelve** (3): BioWare compiled this against the shipped twelve-argument ActionStartConversation, unlike most of its scripts. Identical with the shipped declarations
  `k_ai_master`, `k_pman_27_init01`, `k_trg_transfail`
- **conversation-eleven** (51): BioWare compiled this when ActionStartConversation took eleven arguments; the shipped declaration has twelve. Identical with --declarations bioware
  `k_bant_trig`, `k_def_bandspn`, `k_def_calodef`, `k_hjol_userdef01`, `k_pdan_elise_d`, `k_pdan_rapid01`, `k_pdan_rapid02`, `k_pdan_rapid03`, `k_pdan_rapid04`, `k_pdan_rapid05`, `k_pend_door01`, `k_pend_door02`, `k_pend_door03`, `k_pend_door08`, `k_pkas_freyyuser`, `k_pman_26a_area`, `k_pman_28ac_init`, `k_pman_at01`, `k_pman_at02`, `k_pman_door03`, `k_pman_door06`, `k_pman_door07`, `k_pman_door08`, `k_pman_door10`, `k_pman_door11`, `k_pman_door13`, `k_pman_door14`, `k_pman_door20`, `k_pman_door30`, `k_pman_init01`, `k_pman_init02`, `k_pman_init03`, `k_pman_init10`, `k_pman_judge1_d`, `k_pman_obj_asc`, `k_pman_obj_asc2`, `k_pman_obj_ascc`, `k_pman_obj_ascc2`, `k_pman_obj_ascn`, `k_pman_osc`, `k_pman_rapid01`, `k_pman_rapid02`, `k_pman_rapid03`, `k_pman_rapid04`, `k_pman_rapid05`, `k_pman_sitspeak`, `k_pman_survive01`, `k_ptar_sitharm`, `k_ptat_krayttran`, `k_swg_xor_user`, `k_trg_partyinit`
- **no known cause** (1596): `k_act_bandatt`, `k_act_canderadd`, `k_act_candstim01`, `k_act_candstim02`, `k_act_candstim03`, `k_act_carthadd`, `k_act_cmusic_off`, `k_act_cmusic_on`, `k_act_com44`, `k_act_darkhigh`, `k_act_darkmed`, `k_act_darksml`, `k_act_exitarea`, `k_act_exitarea2`, `k_act_foeinsight`, `k_act_heartoff`, `k_act_hearton`, `k_act_hk47add`, `k_act_hk47simul`, `k_act_joleeadd`, `k_act_juhaniadd`, `k_act_leaverun`, `k_act_lighthigh`, `k_act_lightmed`, `k_act_lightsml`, `k_act_mispaz`, `k_act_missionadd`, `k_act_nospeak`, `k_act_noswitch`, `k_act_npctalk`, `k_act_t3m3add`, `k_act_talktrue`, `k_act_walkways`, `k_act_walkways2`, `k_act_xordie`, `k_act_zaaladd`, `k_amb_prey_spawn`, `k_amb_prey_ude`, `k_cheat02`, `k_cheat_01`, `k_cht_n_bast`, `k_cht_n_cand`, `k_cht_n_carth`, `k_cht_n_hk47`, `k_cht_n_jolee`, `k_cht_n_juhani`, `k_cht_n_mission`, `k_cht_n_screen`, `k_cht_n_t3m4`, `k_cht_n_zaalbar`, `k_cht_p_dan`, `k_cht_p_end`, `k_cht_p_kas`, `k_cht_p_kor`, `k_cht_p_lev`, `k_cht_p_man`, `k_cht_p_sta`, `k_cht_p_tar`, `k_cht_p_tat`, `k_cht_p_unk`, `k_cht_v_levi`, `k_cht_v_mast`, `k_cht_v_star`, `k_con_at_ebon`, `k_con_atdant`, `k_con_atforge`, `k_con_atkash`, `k_con_atkorr`, `k_con_atlev`, `k_con_atmann`, `k_con_atspire`, `k_con_attaris`, `k_con_attat`, `k_con_atunk`, `k_con_bastnpm`, `k_con_bastpm`, `k_con_candernpm`, `k_con_canderpm`, `k_con_candstim01`, `k_con_carthnpm`, `k_con_carthpm`, `k_con_chrhigh`, `k_con_chrlow`, `k_con_chrmiddle`, `k_con_chrnormal`, `k_con_classdiff`, `k_con_classsame`, `k_con_dark`, `k_con_darkhigh`, `k_con_darklow`, `k_con_flirt`, `k_con_flirtno`, `k_con_fpershigh`, `k_con_fperslow`, `k_con_heal`, `k_con_hk47npm`, `k_con_hk47pm`, `k_con_inrange`, `k_con_inthigh`, `k_con_intlow`, `k_con_intnormal`, `k_con_isfemale`, `k_con_isjedi`, `k_con_ismale`, `k_con_isscout`, `k_con_issmug`, `k_con_issoldier`, `k_con_joleejoin`, `k_con_joleenpm`, `k_con_joleepm`, `k_con_juhanilive`, `k_con_juhaninpm`, `k_con_juhanipm`, `k_con_light`, `k_con_lighthigh`, `k_con_lightlow`, `k_con_missionnpm`, `k_con_missionpm`, `k_con_neutral`, `k_con_npctalk`, `k_con_perseasy`, `k_con_pershigh`, `k_con_persmed`, `k_con_repair`, `k_con_t3m3npm`, `k_con_t3m3pm`, `k_con_talkedto`, `k_con_zaalnpm`, `k_con_zaalpm`, `k_def_ambient`, `k_def_ambmob`, `k_def_attacked01`, `k_def_blocked01`, `k_def_buff`, `k_def_calospn`, `k_def_combend01`, `k_def_damage01`, `k_def_death01`, `k_def_dialogue01`, `k_def_disturb01`, `k_def_endconv`, `k_def_grenspn`, `k_def_heartbt01`, `k_def_pathfail01`, `k_def_percept01`, `k_def_repairsp`, `k_def_repuser`, `k_def_spawn01`, `k_def_spawndead`, `k_def_spellat01`, `k_def_userdef01`, `k_hbanter1`, `k_hbanter10`, `k_hbanter10x`, `k_hbanter11`, `k_hbanter11x`, `k_hbanter1x`, `k_hbanter2`, `k_hbanter2x`, `k_hbanter3`, `k_hbanter3x`, `k_hbanter4`, `k_hbanter4x`, `k_hbanter5`, `k_hbanter5x`, `k_hbanter6`, `k_hbanter6x`, `k_hbanter7`, `k_hbanter7x`, `k_hbanter8`, `k_hbanter8x`, `k_hbanter9`, `k_hbanter9x`, `k_hbas_final`, `k_hbas_playkash`, `k_hbas_playkorr`, `k_hbas_playman`, `k_hbas_playtat`, `k_hcan_state01`, `k_hcan_state02`, `k_hcan_state03`, `k_hcan_state04`, `k_hcan_state05`, `k_hcan_state06`, `k_hcan_state07`, `k_hcan_state08`, `k_hcan_state09`, `k_hcan_state10`, `k_hcan_state11`, `k_hcan_state12`, `k_hcan_state13`, `k_hcan_state14`, `k_hcan_state15`, `k_hcan_state16`, `k_hcan_state17`, `k_hcan_state18`, `k_hcan_state19`, `k_hcan_state20`, `k_hcan_state21`, `k_hcan_state22`, `k_hcan_state23`, `k_hcan_state24`, `k_hcan_state25`, `k_hcan_state26`, `k_hcan_state27`, `k_hcan_state28`, `k_hcan_state29`, `k_hcan_state30`, `k_hcan_state31`, `k_hcan_state32`, `k_hcan_state33`, `k_hcan_state34`, `k_hcan_state35`, `k_hcan_state36`, `k_hcan_state37`, `k_hcan_state38`, `k_hcan_state39`, `k_hcan_state40`, `k_hcan_state41`, `k_hcan_state42`, `k_hcan_state43`, `k_hcan_state44`, `k_hcan_state45`, `k_hen_attacked01`, `k_hen_blocked01`, `k_hen_candspwn`, `k_hen_canduser`, `k_hen_combend01`, `k_hen_damage01`, `k_hen_dialogue01`, `k_hen_enter5m`, `k_hen_exit5m`, `k_hen_heartbt01`, `k_hen_leadchng`, `k_hen_percept01`, `k_hen_retreat`, `k_hen_spawn01`, `k_hjol_spawn01`, `k_hjuh_com01`, `k_hjuh_d01`, `k_hjuh_d02`, `k_hjuh_h01`, `k_hjuh_h01x`, `k_hjuh_h02`, `k_hjuh_h03`, `k_hjuh_h04`, `k_hjuh_h05`, `k_hjuh_h06`, `k_hjuh_h07`, `k_hjuh_h08`, `k_hjuh_h09`, `k_hjuh_h10`, `k_hjuh_h10x`, `k_hjuh_h11`, `k_hjuh_h12`, `k_hjuh_h13`, `k_hjuh_p01`, `k_hjuh_p02`, `k_hjuh_p03`, `k_hjuh_p04`, `k_hjuh_p05`, `k_hjuh_p06`, `k_hjuh_p07`, `k_hjuh_p07d`, `k_hjuh_p07l`, `k_hjuh_p08`, `k_hjuh_p09`, `k_hjuh_p10`, `k_hjuh_p10x`, `k_hjuh_p10y`, `k_hjuh_p11`, `k_hjuh_p12`, `k_hjuh_p13`, `k_hjuh_p14`, `k_hjuh_p15`, `k_hjuh_p16`, `k_hjuh_p17`, `k_hjuh_p18`, `k_hjuh_p19`, `k_hjuh_p20`, `k_hjuh_p21`, `k_hjuh_p22`, `k_hjuh_p23`, `k_hjuh_p24`, `k_hjuh_p25`, `k_hjuh_p26`, `k_hjuh_p27`, `k_hjuh_p28`, `k_hjuh_s01`, `k_hjuh_s02`, `k_hjuh_s03`, `k_hjuh_s04`, `k_hjuh_s05`, `k_hjuh_s06`, `k_hjuh_s07`, `k_hjuh_w01`, `k_hjuh_w02`, `k_hjuh_w03`, `k_hjuh_w04`, `k_hjuh_w05`, `k_hjuh_w06`, `k_hjuh_w07`, `k_hjuh_w08`, `k_hjuh_w09`, `k_hjuh_w10`, `k_hmis_misangry`, `k_hmis_misangryx`, `k_hmis_paz`, `k_hmis_planet`, `k_hmis_talk0`, `k_hmis_talk0+`, `k_hmis_talk1`, `k_hmis_talk10`, `k_hmis_talk10lev`, `k_hmis_talk10x`, `k_hmis_talk11`, `k_hmis_talk11lev`, `k_hmis_talk11x`, `k_hmis_talk12`, `k_hmis_talk12x`, `k_hmis_talk13x`, `k_hmis_talk14`, `k_hmis_talk14x`, `k_hmis_talk15`, `k_hmis_talk15x`, `k_hmis_talk1lev`, `k_hmis_talk1x`, `k_hmis_talk2x`, `k_hmis_talk3`, `k_hmis_talk3lev`, `k_hmis_talk3x`, `k_hmis_talk4x`, `k_hmis_talk5`, `k_hmis_talk5x`, `k_hmis_talk6`, `k_hmis_talk6-8`, `k_hmis_talk6lev`, `k_hmis_talk6x`, `k_hmis_talk7`, `k_hmis_talk7lev`, `k_hmis_talk7x`, `k_hmis_talk8`, `k_hmis_talk8x`, `k_hmis_talk9`, `k_hmis_talk99`, `k_hmis_talk99x`, `k_hmis_talk9x`, `k_hmis_talkrev`, `k_hmis_tardest`, `k_hmis_tardestx`, `k_hmis_zaalbar`, `k_hxor_state01`, `k_hxor_state02`, `k_hxor_state03`, `k_hzal_atkash1`, `k_hzal_atkash2`, `k_hzal_beenkashy`, `k_hzal_chuundead`, `k_hzal_freyydead`, `k_hzal_notkash1`, `k_hzal_notkash2`, `k_hzal_talk0`, `k_hzal_talk_1up`, `k_hzal_talkmad_c`, `k_hzal_talkmad_s`, `k_hzal_talkset_1`, `k_messenger_plot`, `k_pdan_anger01`, `k_pdan_anger02`, `k_pdan_anger03`, `k_pdan_anger04`, `k_pdan_anger05`, `k_pdan_anger06`, `k_pdan_anger07`, `k_pdan_anger08`, `k_pdan_anger09`, `k_pdan_anger10`, `k_pdan_anger11`, `k_pdan_anger12`, `k_pdan_anger13`, `k_pdan_anger14`, `k_pdan_anger15`, `k_pdan_anger16`, `k_pdan_anger17`, `k_pdan_anger18`, `k_pdan_anger19`, `k_pdan_anger20`, `k_pdan_bastial05`, `k_pdan_bastila01`, `k_pdan_bastila02`, `k_pdan_bastila03`, `k_pdan_bastila04`, `k_pdan_bastila05`, `k_pdan_bastila06`, `k_pdan_bastila07`, `k_pdan_bastila08`, `k_pdan_bastila09`, `k_pdan_bastila10`, `k_pdan_bastila11`, `k_pdan_belaya01`, `k_pdan_belaya02`, `k_pdan_cander01`, `k_pdan_cander02`, `k_pdan_cander03`, `k_pdan_carth01`, `k_pdan_carth02`, `k_pdan_carth03`, `k_pdan_carth04`, `k_pdan_casus01`, `k_pdan_casus02`, `k_pdan_casus03`, `k_pdan_casus04`, `k_pdan_casus05`, `k_pdan_destself`, `k_pdan_drandom02`, `k_pdan_drandom03`, `k_pdan_drandom04`, `k_pdan_drandom05`, `k_pdan_drandom06`, `k_pdan_drandom07`, `k_pdan_drandom08`, `k_pdan_droid01`, `k_pdan_droid02`, `k_pdan_droid03`, `k_pdan_droid04`, `k_pdan_droid05`, `k_pdan_droid06`, `k_pdan_droid07`, `k_pdan_droid08`, `k_pdan_droid09`, `k_pdan_droid10`, `k_pdan_droid11`, `k_pdan_droid12`, `k_pdan_droid13`, `k_pdan_droid14`, `k_pdan_droid15`, `k_pdan_droid50`, `k_pdan_iriaz_sp`, `k_pdan_iriaz_ud`, `k_pdan_jedi01`, `k_pdan_jedi02`, `k_pdan_jedi03`, `k_pdan_jedi04`, `k_pdan_jedi05`, `k_pdan_jedi06`, `k_pdan_jedi07`, `k_pdan_jedi08`, `k_pdan_jedi09`, `k_pdan_jedi10`, `k_pdan_jedi11`, `k_pdan_jedi12`, `k_pdan_jedi13`, `k_pdan_jedi14`, `k_pdan_jedi15`, `k_pdan_jedi16`, `k_pdan_jedi17`, `k_pdan_jedi18`, `k_pdan_jedi19`, `k_pdan_jedi20`, `k_pdan_jedi21`, `k_pdan_jedi22`, `k_pdan_jedi23`, `k_pdan_jedi24`, `k_pdan_jedi25`, `k_pdan_jedi26`, `k_pdan_jedi27`, `k_pdan_jedi28`, `k_pdan_jedi29`, `k_pdan_jedi30`, `k_pdan_jedi31`, `k_pdan_jedi32`, `k_pdan_jedi33`, `k_pdan_jedi34`, `k_pdan_jedi35`, `k_pdan_jedi36`, `k_pdan_jedi37`, `k_pdan_jedi38`, `k_pdan_jedi39`, `k_pdan_jedi40`, `k_pdan_juhani01`, `k_pdan_juhani02`, `k_pdan_juhani03`, `k_pdan_juhani04`, `k_pdan_juhani05`, `k_pdan_juhani06`, `k_pdan_juhani07`, `k_pdan_juhani08`, `k_pdan_juhani09`, `k_pdan_juhani10`, `k_pdan_juhani11`, `k_pdan_juhani12`, `k_pdan_juhani15`, `k_pdan_juhani16`, `k_pdan_juhani17`, `k_pdan_juhani18`, `k_pdan_juhani19`, `k_pdan_juhani20`, `k_pdan_kath01`, `k_pdan_kath02`, `k_pdan_kath03`, `k_pdan_kath04`, `k_pdan_mand01`, `k_pdan_mand02`, `k_pdan_mand03`, `k_pdan_mand04`, `k_pdan_mand05`, `k_pdan_mand06`, `k_pdan_mand07`, `k_pdan_mand08`, `k_pdan_mand09`, `k_pdan_mand10`, `k_pdan_mand11`, `k_pdan_mand12`, `k_pdan_mand51`, `k_pdan_mand52`, `k_pdan_mission01`, `k_pdan_mission02`, `k_pdan_murder01`, `k_pdan_murder02`, `k_pdan_murder03`, `k_pdan_murder04`, `k_pdan_murder05`, `k_pdan_murder06`, `k_pdan_murder07`, `k_pdan_murder08`, `k_pdan_murder09`, `k_pdan_murder10`, `k_pdan_murder11`, `k_pdan_murder12`, `k_pdan_murder13`, `k_pdan_murder14`, `k_pdan_murder15`, `k_pdan_murder16`, `k_pdan_murder17`, `k_pdan_murder18`, `k_pdan_murder19`, `k_pdan_murder20`, `k_pdan_murder21`, `k_pdan_murder22`, `k_pdan_murder23`, `k_pdan_murder24`, `k_pdan_murder25`, `k_pdan_murder26`, `k_pdan_murder27`, `k_pdan_murder28`, `k_pdan_murder29`, `k_pdan_murder30`, `k_pdan_murder31`, `k_pdan_murder32`, `k_pdan_murder33`, `k_pdan_murder34`, `k_pdan_murder35`, `k_pdan_murder36`, `k_pdan_murder37`, `k_pdan_murder38`, `k_pdan_murder39`, `k_pdan_murder40`, `k_pdan_murder41`, `k_pdan_murder42`, `k_pdan_murder43`, `k_pdan_murder44`, `k_pdan_murder45`, `k_pdan_murder46`, `k_pdan_murder47`, `k_pdan_murder48`, `k_pdan_murder49`, `k_pdan_murder50`, `k_pdan_murder51`, `k_pdan_murder52`, `k_pdan_murder53`, `k_pdan_murder54`, `k_pdan_murder55`, `k_pdan_murder56`, `k_pdan_murder57`, `k_pdan_murder58`, `k_pdan_murder59`, `k_pdan_murder60`, `k_pdan_murder61`, `k_pdan_murder62`, `k_pdan_murder63`, `k_pdan_murder64`, `k_pdan_murder65`, `k_pdan_murder66`, `k_pdan_murder67`, `k_pdan_murder68`, `k_pdan_murder69`, `k_pdan_murder70`, `k_pdan_murder71`, `k_pdan_murder72`, `k_pdan_murder73`, `k_pdan_murder74`, `k_pdan_murder75`, `k_pdan_murder76`, `k_pdan_planet01`, `k_pdan_planet02`, `k_pdan_planet03`, `k_pdan_planet04`, `k_pdan_planet05`, `k_pdan_planet06`, `k_pdan_planet08`, `k_pdan_planet12`, `k_pdan_planet14`, `k_pdan_planet15`, `k_pdan_planet16`, `k_pdan_romance01`, `k_pdan_romance02`, `k_pdan_romance03`, `k_pdan_romance04`, `k_pdan_romance05`, `k_pdan_romance06`, `k_pdan_romance07`, `k_pdan_romance08`, `k_pdan_romance09`, `k_pdan_romance10`, `k_pdan_romance11`, `k_pdan_romance12`, `k_pdan_romance13`, `k_pdan_romance14`, `k_pdan_romance15`, `k_pdan_romance16`, `k_pdan_romance17`, `k_pdan_romance18`, `k_pdan_romance19`, `k_pdan_romance20`, `k_pdan_romance21`, `k_pdan_romance22`, `k_pdan_romance23`, `k_pdan_romance24`, `k_pdan_romance25`, `k_pdan_romance26`, `k_pdan_romance27`, `k_pdan_romance28`, `k_pdan_romance29`, `k_pdan_romance30`, `k_pdan_romance31`, `k_pdan_romance32`, `k_pdan_romance33`, `k_pdan_romance34`, `k_pdan_romance35`, `k_pdan_romance36`, `k_pdan_romance37`, `k_pdan_romance38`, `k_pdan_romance39`, `k_pdan_romance40`, `k_pdan_romance41`, `k_pdan_romance42`, `k_pdan_stand01`, `k_pdan_stand02`, `k_pdan_stand03`, `k_pebn_ctaris01`, `k_pebn_ctaris02`, `k_pebn_ctaris03`, `k_pebn_ctaris04`, `k_pebn_ctaris05`, `k_pebn_ctaris06`, `k_pebn_ctaris07`, `k_pebn_ctaris08`, `k_pebn_ctaris09`, `k_pebn_gotaris`, `k_pebn_loc01`, `k_pebn_loc02`, `k_pebn_loc03`, `k_pebn_loc04`, `k_pebn_loc05`, `k_pebn_loc06`, `k_pebn_loc07`, `k_pebn_loc08`, `k_pebn_loc09`, `k_pebn_swoop`, `k_pend_door05`, `k_pend_door06`, `k_pend_door07`, `k_pend_door09`, `k_pend_explode`, `k_pend_rumble01`, `k_phkd_fighter`, `k_pkas_chuundead`, `k_pkas_enemyfac`, `k_pkas_freyydead`, `k_pkas_freyyhelp`, `k_pkas_freyyspwn`, `k_pkas_hashilt`, `k_pkas_hassword`, `k_pkas_heart_on`, `k_pkas_jaarakblt`, `k_pkas_jaaraksus`, `k_pkas_jaardead`, `k_pkas_jaarfreed`, `k_pkas_nohilt`, `k_pkas_nosword`, `k_pkas_pause3sec`, `k_pkas_randwalk`, `k_pkas_takehilt`, `k_pman_26a_aread`, `k_pman_26c_area`, `k_pman_26d_area`, `k_pman_26d_triga`, `k_pman_26d_trigb`, `k_pman_26d_trigc`, `k_pman_27_area`, `k_pman_28b_at01`, `k_pman_28b_at02`, `k_pman_28c_sur01`, `k_pman_28d_cmp01`, `k_pman_28d_cmp02`, `k_pman_28d_triga`, `k_pman_28d_trigb`, `k_pman_aioff`, `k_pman_aion`, `k_pman_airlock01`, `k_pman_airlock02`, `k_pman_airlock03`, `k_pman_amb_9`, `k_pman_ambient01`, `k_pman_ambient02`, `k_pman_ambient03`, `k_pman_ambient04`, `k_pman_ambient05`, `k_pman_ambient06`, `k_pman_ambient07`, `k_pman_ambient08`, `k_pman_area_28a`, `k_pman_arg01`, `k_pman_argrep_d`, `k_pman_argsith_d`, `k_pman_beast_9`, `k_pman_beast_d`, `k_pman_body01`, `k_pman_casamad_s`, `k_pman_casamad_y`, `k_pman_casname_n`, `k_pman_casname_s`, `k_pman_champ_f01`, `k_pman_champ_f02`, `k_pman_champ_f03`, `k_pman_champ_f04`, `k_pman_champ_m01`, `k_pman_champ_m02`, `k_pman_champ_m03`, `k_pman_champ_m04`, `k_pman_champ_qui`, `k_pman_comp01`, `k_pman_comp02`, `k_pman_comp03`, `k_pman_comp04`, `k_pman_comp05`, `k_pman_comp06`, `k_pman_comp07`, `k_pman_comp08`, `k_pman_comp09`, `k_pman_comp10`, `k_pman_comp11`, `k_pman_comp30`, `k_pman_comp31`, `k_pman_comp32`, `k_pman_comp33`, `k_pman_comp34`, `k_pman_comp35`, `k_pman_door01`, `k_pman_door02`, `k_pman_door04`, `k_pman_door05`, `k_pman_door12`, `k_pman_door31`, `k_pman_door32`, `k_pman_door40`, `k_pman_false`, `k_pman_firixa_d`, `k_pman_guard01`, `k_pman_hukname_n`, `k_pman_hukname_s`, `k_pman_huktmad_1`, `k_pman_huktmad_y`, `k_pman_ignore`, `k_pman_ignus01`, `k_pman_locked01`, `k_pman_locked02`, `k_pman_lostset_n`, `k_pman_lostset_q`, `k_pman_lostset_y`, `k_pman_main01`, `k_pman_misc01`, `k_pman_misc02`, `k_pman_misc03`, `k_pman_misc04`, `k_pman_misc05`, `k_pman_misc06`, `k_pman_misc07`, `k_pman_misc08`, `k_pman_misc09`, `k_pman_misc10`, `k_pman_misc11`, `k_pman_misc12`, `k_pman_misc13`, `k_pman_misc14`, `k_pman_misc15`, `k_pman_misc16`, `k_pman_misc17`, `k_pman_misc18`, `k_pman_misc19`, `k_pman_misc20`, `k_pman_misc21`, `k_pman_misc22`, `k_pman_misc23`, `k_pman_misc24`, `k_pman_misc25`, `k_pman_misc26`, `k_pman_misc27`, `k_pman_misc28`, `k_pman_misc29`, `k_pman_misc30`, `k_pman_misc31`, `k_pman_misc32`, `k_pman_misc33`, `k_pman_misc34`, `k_pman_misc35`, `k_pman_misc36`, `k_pman_misc37`, `k_pman_misc38`, `k_pman_misc39`, `k_pman_misc40`, `k_pman_misc41`, `k_pman_misc42`, `k_pman_misc43`, `k_pman_misc44`, `k_pman_misc45`, `k_pman_misc46`, `k_pman_misc47`, `k_pman_misc48`, `k_pman_misc49`, `k_pman_misc50`, `k_pman_misc51`, `k_pman_misc52`, `k_pman_misc53`, `k_pman_misc54`, `k_pman_misc55`, `k_pman_misc56`, `k_pman_misc57`, `k_pman_misc58`, `k_pman_misc59`, `k_pman_misc60`, `k_pman_misc61`, `k_pman_misc62`, `k_pman_misc70`, `k_pman_misc71`, `k_pman_misc72`, `k_pman_misc73`, `k_pman_misc74`, `k_pman_misc75`, `k_pman_misc76`, `k_pman_miss01`, `k_pman_missing01`, `k_pman_missing02`, `k_pman_missing03`, `k_pman_missing04`, `k_pman_missing05`, `k_pman_missing06`, `k_pman_missing07`, `k_pman_missing08`, `k_pman_missing09`, `k_pman_missing10`, `k_pman_missing11`, `k_pman_missing12`, `k_pman_missing13`, `k_pman_missing14`, `k_pman_missing15`, `k_pman_missing16`, `k_pman_missing17`, `k_pman_missing18`, `k_pman_missing19`, `k_pman_missing20`, `k_pman_missing21`, `k_pman_missing22`, `k_pman_missing23`, `k_pman_missing24`, `k_pman_missing25`, `k_pman_missing26`, `k_pman_missing27`, `k_pman_missing28`, `k_pman_missing29`, `k_pman_missing30`, `k_pman_missing31`, `k_pman_missing32`, `k_pman_missing33`, `k_pman_missing34`, `k_pman_missing35`, `k_pman_missing36`, `k_pman_missing37`, `k_pman_missing38`, `k_pman_missing39`, `k_pman_missing40`, `k_pman_missing41`, `k_pman_missing42`, `k_pman_missing43`, `k_pman_murder01`, `k_pman_murder02`, `k_pman_murder03`, `k_pman_murder04`, `k_pman_murder05`, `k_pman_murder06`, `k_pman_murder07`, `k_pman_murder08`, `k_pman_murder09`, `k_pman_murder10`, `k_pman_murder11`, `k_pman_murder12`, `k_pman_murder13`, `k_pman_murder14`, `k_pman_murder15`, `k_pman_murder16`, `k_pman_murder17`, `k_pman_murder18`, `k_pman_murder19`, `k_pman_murder20`, `k_pman_murder21`, `k_pman_murder22`, `k_pman_murder23`, `k_pman_murder24`, `k_pman_murder25`, `k_pman_murder26`, `k_pman_murder27`, `k_pman_murder28`, `k_pman_murder29`, `k_pman_murder30`, `k_pman_murder31`, `k_pman_murder32`, `k_pman_murder33`, `k_pman_murder34`, `k_pman_murder35`, `k_pman_murder36`, `k_pman_murder37`, `k_pman_murder38`, `k_pman_murder39`, `k_pman_murder40`, `k_pman_murder41`, `k_pman_murder42`, `k_pman_murder43`, `k_pman_murder44`, `k_pman_murder45`, `k_pman_murder46`, `k_pman_murder47`, `k_pman_murder48`, `k_pman_murder49`, `k_pman_murder50`, `k_pman_murder51`, `k_pman_murder52`, `k_pman_murder53`, `k_pman_murder54`, `k_pman_murder55`, `k_pman_murder56`, `k_pman_murder57`, `k_pman_murder58`, `k_pman_murder59`, `k_pman_murder60`, `k_pman_murder61`, `k_pman_murder64`, `k_pman_murder65`, `k_pman_murder66`, `k_pman_murder67`, `k_pman_murder68`, `k_pman_murder69`, `k_pman_murder70`, `k_pman_murder71`, `k_pman_murder72`, `k_pman_murder73`, `k_pman_murder74`, `k_pman_murder75`, `k_pman_murder76`, `k_pman_murder77`, `k_pman_murder78`, `k_pman_murder79`, `k_pman_murder80`, `k_pman_murder81`, `k_pman_murder82`, `k_pman_murder83`, `k_pman_murder84`, `k_pman_murder85`, `k_pman_murder86`, `k_pman_murder87`, `k_pman_murder88`, `k_pman_murder89`, `k_pman_murder90`, `k_pman_murder91`, `k_pman_murder92`, `k_pman_murder93`, `k_pman_murder94`, `k_pman_murder95`, `k_pman_murder96`, `k_pman_murder97`, `k_pman_murder98`, `k_pman_murder99`, `k_pman_murder9a`, `k_pman_murder9b`, `k_pman_murder9c`, `k_pman_murder9d`, `k_pman_murder9e`, `k_pman_murder9f`, `k_pman_murder9g`, `k_pman_murder9h`, `k_pman_murder9i`, `k_pman_murder9j`, `k_pman_murder9k`, `k_pman_murder9l`, `k_pman_murder9m`, `k_pman_murder9n`, `k_pman_murder9o`, `k_pman_murder9p`, `k_pman_murder9q`, `k_pman_murder9r`, `k_pman_murder9s`, `k_pman_murder9t`, `k_pman_murder9u`, `k_pman_murder9v`, `k_pman_murder9w`, `k_pman_murder9x`, `k_pman_murder9y`, `k_pman_murder9z`, `k_pman_murdera01`, `k_pman_murdera02`, `k_pman_murdera03`, `k_pman_murdera04`, `k_pman_murdera05`, `k_pman_murdera06`, `k_pman_murdera07`, `k_pman_murdera08`, `k_pman_murdera09`, `k_pman_murdera11`, `k_pman_no_threat`, `k_pman_notpaid_c`, `k_pman_notpaid_n`, `k_pman_notpaid_s`, `k_pman_npcstart`, `k_pman_npcstop`, `k_pman_patrol_d`, `k_pman_pause2sec`, `k_pman_payrace_1`, `k_pman_payrace_2`, `k_pman_payrace_3`, `k_pman_pazaak01`, `k_pman_pazaak02`, `k_pman_pazaak03`, `k_pman_pazaak04`, `k_pman_pazaak05`, `k_pman_pazaak06`, `k_pman_pazaak07`, `k_pman_pazaak08`, `k_pman_pazaak09`, `k_pman_pazaak10`, `k_pman_pazaak11`, `k_pman_persuad_f`, `k_pman_persuad_s`, `k_pman_persuad_y`, `k_pman_planet01`, `k_pman_planet02`, `k_pman_planet03`, `k_pman_planet04`, `k_pman_planet05`, `k_pman_planet06`, `k_pman_planet07`, `k_pman_planet08`, `k_pman_planet09`, `k_pman_planet10`, `k_pman_planet11`, `k_pman_planet12`, `k_pman_planet13`, `k_pman_planet14`, `k_pman_planet15`, `k_pman_planet16`, `k_pman_planet17`, `k_pman_planet18`, `k_pman_planet19`, `k_pman_planet20`, `k_pman_planet21`, `k_pman_planet22`, `k_pman_planet23`, `k_pman_planet24`, `k_pman_planet25`, `k_pman_planet26`, `k_pman_planet27`, `k_pman_planet28`, `k_pman_planet29`, `k_pman_planet30`, `k_pman_planet31`, `k_pman_planet32`, `k_pman_planet33`, `k_pman_planet34`, `k_pman_planet35`, `k_pman_planet36`, `k_pman_planet37`, `k_pman_planet38`, `k_pman_planet39`, `k_pman_planet40`, `k_pman_planet41`, `k_pman_planet42`, `k_pman_planet43`, `k_pman_planet44`, `k_pman_planet45`, `k_pman_planet46`, `k_pman_planet47`, `k_pman_planet48`, `k_pman_planet49`, `k_pman_planet50`, `k_pman_planet51`, `k_pman_planet52`, `k_pman_planet53`, `k_pman_planet54`, `k_pman_planet55`, `k_pman_planet56`, `k_pman_planet57`, `k_pman_planet58`, `k_pman_planet59`, `k_pman_planet60`, `k_pman_planet61`, `k_pman_planet62`, `k_pman_planet63`, `k_pman_planet64`, `k_pman_planet65`, `k_pman_planet66`, `k_pman_planet67`, `k_pman_planet68`, `k_pman_planet69`, `k_pman_planet70`, `k_pman_planet71`, `k_pman_planet72`, `k_pman_planet73`, `k_pman_planet74`, `k_pman_planet75`, `k_pman_planet76`, `k_pman_planet77`, `k_pman_planet78`, `k_pman_planet79`, `k_pman_planet80`, `k_pman_planet81`, `k_pman_planet82`, `k_pman_planet83`, `k_pman_planet84`, `k_pman_planet85`, `k_pman_planet86`, `k_pman_planet87`, `k_pman_port01`, `k_pman_puzzle01`, `k_pman_puzzle02`, `k_pman_puzzle03`, `k_pman_puzzle04`, `k_pman_puzzle05`, `k_pman_puzzle06`, `k_pman_puzzle07`, `k_pman_puzzle08`, `k_pman_puzzle09`, `k_pman_puzzle10`, `k_pman_puzzle11`, `k_pman_puzzle12`, `k_pman_puzzle13`, `k_pman_puzzle14`, `k_pman_qreturn_n`, `k_pman_qreturn_s`, `k_pman_queecham2`, `k_pman_queechamp`, `k_pman_queelostp`, `k_pman_queemad_s`, `k_pman_queemad_y`, `k_pman_queenotpc`, `k_pman_queeorpcy`, `k_pman_queequit`, `k_pman_quename_n`, `k_pman_quename_s`, `k_pman_quepaid_n`, `k_pman_quepaid_s`, `k_pman_quepaid_y`, `k_pman_quetell_n`, `k_pman_quetell_s`, `k_pman_quetell_y`, `k_pman_quleave_s`, `k_pman_quleave_y`, `k_pman_race_0_02`, `k_pman_race_0_03`, `k_pman_race_0_04`, `k_pman_race_0_q1`, `k_pman_race_0_q2`, `k_pman_race_0_q3`, `k_pman_race_0_q4`, `k_pman_race_1_02`, `k_pman_race_1_03`, `k_pman_race_1_04`, `k_pman_race_2_02`, `k_pman_race_2_03`, `k_pman_race_2_04`, `k_pman_race_3_02`, `k_pman_race_3_03`, `k_pman_race_3_04`, `k_pman_race_s_01`, `k_pman_race_s_02`, `k_pman_race_s_03`, `k_pman_race_s_04`, `k_pman_race_x_02`, `k_pman_race_x_03`, `k_pman_race_x_04`, `k_pman_racecha_n`, `k_pman_racechamp`, `k_pman_racelosty`, `k_pman_racenone`, `k_pman_raceone`, `k_pman_raceonex`, `k_pman_raceset_1`, `k_pman_raceset_2`, `k_pman_raceset_3`, `k_pman_raceset_4`, `k_pman_racetwo`, `k_pman_racetwox`, `k_pman_random01`, `k_pman_random02`, `k_pman_random03`, `k_pman_random04`, `k_pman_random05`, `k_pman_random06`, `k_pman_random07`, `k_pman_random08`, `k_pman_random09`, `k_pman_random10`, `k_pman_random11`, `k_pman_random12`, `k_pman_random13`, `k_pman_random14`, `k_pman_random15`, `k_pman_random16`, `k_pman_random17`, `k_pman_random18`, `k_pman_random19`, `k_pman_random20`, `k_pman_random21`, `k_pman_random22`, `k_pman_random23`, `k_pman_random24`, `k_pman_random25`, `k_pman_random26`, `k_pman_random27`, `k_pman_random28`, `k_pman_random29`, `k_pman_random30`, `k_pman_random31`, `k_pman_random32`, `k_pman_random33`, `k_pman_reward01`, `k_pman_rodian01`, `k_pman_rodian02`, `k_pman_rodian03`, `k_pman_rodian04`, `k_pman_rodian05`, `k_pman_rodian06`, `k_pman_rodian07`, `k_pman_selapr5_d`, `k_pman_selapr_d`, `k_pman_shark01`, `k_pman_shasa01`, `k_pman_shasa02`, `k_pman_shasa03`, `k_pman_shield_9`, `k_pman_shield_d`, `k_pman_sith01`, `k_pman_sith02`, `k_pman_sithmas01`, `k_pman_sonic`, `k_pman_speed01`, `k_pman_starmap`, `k_pman_suituse`, `k_pman_sur01`, `k_pman_sur02`, `k_pman_survive02`, `k_pman_swoop01`, `k_pman_swoop02`, `k_pman_swoop03`, `k_pman_swoop04`, `k_pman_swoop05`, `k_pman_swoop06`, `k_pman_swoop07`, `k_pman_swoop08`, `k_pman_swoop09`, `k_pman_swoop10`, `k_pman_swoop11`, `k_pman_swoop12`, `k_pman_swoop13`, `k_pman_swoop14`, `k_pman_swoop15`, `k_pman_swoop16`, `k_pman_swoop17`, `k_pman_swoop18`, `k_pman_swoop19`, `k_pman_swoop20`, `k_pman_swoop21`, `k_pman_swoop22`, `k_pman_swoop23`, `k_pman_swoop24`, `k_pman_swoop25`, `k_pman_swoop26`, `k_pman_swoop27`, `k_pman_swoop28`, `k_pman_swoop29`, `k_pman_swoop30`, `k_pman_swoop31`, `k_pman_swoop32`, `k_pman_swoop33`, `k_pman_swoop34`, `k_pman_swoop35`, `k_pman_swoop36`, `k_pman_swoop37`, `k_pman_swoop38`, `k_pman_swoop39`, `k_pman_swoop40`, `k_pman_swoop41`, `k_pman_tatracern`, `k_pman_tatracery`, `k_pman_tech01`, `k_pman_trial01`, `k_pman_trial02`, `k_pman_ww`, `k_pman_ye_threat`, `k_ptar_sithdis`, `k_ptar_sithpaper`, `k_ptat_1gamewon`, `k_ptat_2gamewon`, `k_ptat_3gamewon`, `k_ptat_amb1`, `k_ptat_angrgurke`, `k_ptat_baddeal`, `k_ptat_beenkash`, `k_ptat_bounty_n`, `k_ptat_bounty_y`, `k_ptat_bountyset`, `k_ptat_bullynico`, `k_ptat_canhunt`, `k_ptat_celisdeal`, `k_ptat_checkmad`, `k_ptat_custmove`, `k_ptat_dockpaid`, `k_ptat_dockpay`, `k_ptat_dorakexit`, `k_ptat_dorakname`, `k_ptat_dorakwho`, `k_ptat_drandom09`, `k_ptat_drandom10`, `k_ptat_drandom11`, `k_ptat_drandom12`, `k_ptat_dunesea`, `k_ptat_gaffiwrit`, `k_ptat_gammbad`, `k_ptat_gammbribe`, `k_ptat_gammgone`, `k_ptat_gammhunt`, `k_ptat_gammleave`, `k_ptat_gammmean`, `k_ptat_gammwarn`, `k_ptat_gandmad`, `k_ptat_gandname`, `k_ptat_garmleave`, `k_ptat_garmname`, `k_ptat_garmwho`, `k_ptat_gateperm`, `k_ptat_givepearl`, `k_ptat_givewrit`, `k_ptat_gizkaspwn`, `k_ptat_gizkauser`, `k_ptat_gooddeal`, `k_ptat_gurkemad`, `k_ptat_has_k_map`, `k_ptat_hasgaffi`, `k_ptat_haspearl`, `k_ptat_haspermit`, `k_ptat_haswrit`, `k_ptat_huntask_n`, `k_ptat_huntask_y`, `k_ptat_koname_ch`, `k_ptat_konameset`, `k_ptat_leave`, `k_ptat_losscheck`, `k_ptat_m_office`, `k_ptat_movekrayt`, `k_ptat_namecheck`, `k_ptat_nicenico`, `k_ptat_nicofree`, `k_ptat_nicohappy`, `k_ptat_nicoinfo`, `k_ptat_niconews`, `k_ptat_nicotalk`, `k_ptat_nicowait`, `k_ptat_no_k_map`, `k_ptat_nohkinfo`, `k_ptat_nonicotlk`, `k_ptat_nopermit`, `k_ptat_norespon`, `k_ptat_nosalehk`, `k_ptat_notracer`, `k_ptat_nowrit`, `k_ptat_officemad`, `k_ptat_pazzloss`, `k_ptat_pazzwin1`, `k_ptat_pazzwin2`, `k_ptat_pazzwin3`, `k_ptat_persyuka`, `k_ptat_racechamp`, `k_ptat_racenone`, `k_ptat_raceone`, `k_ptat_racer`, `k_ptat_racer_id`, `k_ptat_racetrial`, `k_ptat_racetwo`, `k_ptat_salehk`, `k_ptat_sandper`, `k_ptat_sethunt`, `k_ptat_shariwait`, `k_ptat_sharleave`, `k_ptat_shartrust`, `k_ptat_soldhk`, `k_ptat_soldpearl`, `k_ptat_swoopdwn1`, `k_ptat_swoopdwn2`, `k_ptat_swoopdwn3`, `k_ptat_swoopdwn4`, `k_ptat_talktanis`, `k_ptat_taname_ch`, `k_ptat_tanameset`, `k_ptat_tanileave`, `k_ptat_tanisdead`, `k_ptat_tanisgave`, `k_ptat_tanismad`, `k_ptat_tatinfo_c`, `k_ptat_tatinfo_s`, `k_ptat_toldofhk`, `k_ptat_tuskpdone`, `k_ptat_yukaleave`, `k_ptat_yukanpers`, `k_ptat_yukarname`, `k_ptat_yukarwho`, `k_ptat_zoriiname`, `k_ptat_zoriiwho`, `k_ptat_zorileave`, `k_punk_cold`, `k_ren_leavlev`, `k_ren_levcapture`, `k_ren_levescape`, `k_ren_nextplanet`, `k_ren_starforge`, `k_ren_starland`, `k_ren_taris01`, `k_ren_taris02`, `k_ren_taris03`, `k_ren_turret`, `k_ren_turretld02`, `k_ren_turretload`, `k_ren_unkcrash`, `k_ren_unkturret`, `k_ren_visionland`, `k_sup_actmap`, `k_sup_bands`, `k_sup_comshots`, `k_sup_fear`, `k_sup_guiopen`, `k_sup_healing`, `k_sup_static`, `k_swg_bantforce`, `k_swg_bastdef`, `k_swg_bastevil`, `k_swg_bastforce1`, `k_swg_bastforce3`, `k_swg_bastforcex`, `k_swg_bastforcey`, `k_swg_bastila01`, `k_swg_bastila02`, `k_swg_bastila03`, `k_swg_bastila04`, `k_swg_bastila05`, `k_swg_bastila06`, `k_swg_bastila07`, `k_swg_bastila08`, `k_swg_bastila09`, `k_swg_bastila10`, `k_swg_bastila11`, `k_swg_bastila12`, `k_swg_bastila12a`, `k_swg_bastila13`, `k_swg_bastila14`, `k_swg_bastila15`, `k_swg_bastila16`, `k_swg_bastila17`, `k_swg_bastila18`, `k_swg_bastila19`, `k_swg_bastila20`, `k_swg_bastila20x`, `k_swg_bastila21`, `k_swg_bastila22`, `k_swg_bastila23`, `k_swg_bastila24`, `k_swg_bastila25`, `k_swg_bastila26`, `k_swg_bastila27`, `k_swg_bastila27x`, `k_swg_bastila28`, `k_swg_bastila29`, `k_swg_bastila30`, `k_swg_bastila31`, `k_swg_bastila32`, `k_swg_bastila99`, `k_swg_bastile17`, `k_swg_bastmovie`, `k_swg_carth01`, `k_swg_carth02`, `k_swg_carth03`, `k_swg_carth04`, `k_swg_carth05`, `k_swg_carth06`, `k_swg_carth07`, `k_swg_carth08`, `k_swg_carth09`, `k_swg_carth10`, `k_swg_carth11`, `k_swg_carth12`, `k_swg_carth13`, `k_swg_carth14`, `k_swg_carth15`, `k_swg_carth16`, `k_swg_carth17`, `k_swg_carth18`, `k_swg_carth19`, `k_swg_carth20`, `k_swg_carth21`, `k_swg_carth22`, `k_swg_carth23`, `k_swg_carth24`, `k_swg_carth25`, `k_swg_carth25x`, `k_swg_carth25y`, `k_swg_carth26`, `k_swg_carth99`, `k_swg_helena07`, `k_swg_helena08`, `k_swg_hk4700`, `k_swg_hk4701`, `k_swg_hk4702`, `k_swg_hk4703`, `k_swg_hk4704`, `k_swg_hk4705`, `k_swg_hk4706`, `k_swg_hk4707`, `k_swg_hk4708`, `k_swg_hk4709`, `k_swg_hk4710`, `k_swg_hk4711`, `k_swg_hk4712`, `k_swg_hk4712x`, `k_swg_hk4712y`, `k_swg_hk4713`, `k_swg_hk47fix`, `k_swg_hk47upg01`, `k_swg_hk47upg02`, `k_swg_hk47upg03`, `k_swg_hk47upg04`, `k_swg_jolee01`, `k_swg_jolee02`, `k_swg_jolee03`, `k_swg_jolee04`, `k_swg_jolee05`, `k_swg_jolee06`, `k_swg_jolee07`, `k_swg_jolee08`, `k_swg_jolee09`, `k_swg_jolee10`, `k_swg_jolee11`, `k_swg_jolee12`, `k_swg_jolee13`, `k_swg_jolee14`, `k_swg_jolee15`, `k_swg_jolee16`, `k_swg_jolee17`, `k_swg_jolee18`, `k_swg_jolee19`, `k_swg_jolee20`, `k_swg_jolee99`, `k_swg_joleerev`, `k_swg_replaykas`, `k_swg_replaykor`, `k_swg_replayman`, `k_swg_replaytat`, `k_swg_sasha01`, `k_swg_senni1`, `k_swg_senni2`, `k_swg_xor_def`, `k_swg_ziag01`, `k_trg_bandon`, `k_trg_calonord`, `k_trg_xorattack`, `k_unk_rancor`, `qa_calo`, `qa_lev`, `qa_map`, `qa_master`, `qa_planet`, `qa_setanim`

Size differences, largest clusters first:

| Bytes | Scripts | Examples |
| --- | --- | --- |
| -12 | 417 | `k_con_at_ebon`, `k_con_joleejoin`, `k_con_juhanilive` |
| +0 | 375 | `k_act_canderadd`, `k_act_carthadd`, `k_act_cmusic_off` |
| -26 | 143 | `k_hcan_state02`, `k_hcan_state03`, `k_hcan_state04` |
| -6 | 110 | `k_cht_v_levi`, `k_cht_v_mast`, `k_cht_v_star` |
| -40 | 63 | `k_act_talktrue`, `k_con_flirtno`, `k_con_fperslow` |
| -88 | 37 | `k_pman_casamad_y`, `k_pman_casname_n`, `k_pman_hukname_n` |
| -36 | 37 | `k_con_chrhigh`, `k_con_chrnormal`, `k_con_inthigh` |
| -266 | 32 | `k_pman_casamad_s`, `k_pman_casname_s`, `k_pman_hukname_s` |
| -32 | 31 | `k_pdan_droid15`, `k_pman_missing25`, `k_pman_missing29` |
| -46 | 21 | `k_hjuh_p28`, `k_hmis_planet`, `k_hzal_atkash1` |
| -18 | 19 | `k_con_atdant`, `k_con_atforge`, `k_con_atkash` |
| +6 | 19 | `k_con_classdiff`, `k_con_classsame`, `k_messenger_plot` |
| -50 | 18 | `k_pman_race_0_02`, `k_pman_race_0_03`, `k_pman_race_0_04` |
| -224 | 15 | `k_amb_prey_spawn`, `k_def_ambient`, `k_def_ambmob` |
| -56 | 15 | `k_con_darkhigh`, `k_con_darklow`, `k_con_lightlow` |
| -4866 | 14 | `k_plc_tresciv`, `k_plc_trescorhig`, `k_plc_trescorlow` |
| -20 | 13 | `k_con_bastpm`, `k_con_canderpm`, `k_con_carthpm` |
| -16 | 12 | `k_def_calodef`, `k_pkas_hashilt`, `k_pkas_hassword` |
| +24 | 12 | `k_act_candstim01`, `k_act_candstim02`, `k_act_candstim03` |
| -70 | 11 | `k_def_repairsp`, `k_pman_champ_f02`, `k_pman_champ_f03` |
| +12 | 11 | `k_cht_p_dan`, `k_cht_p_end`, `k_cht_p_kas` |
| -370 | 10 | `k_pdan_rapid01`, `k_pdan_rapid02`, `k_pdan_rapid03` |
| -96 | 10 | `k_pman_planet64`, `k_pman_planet65`, `k_pman_planet66` |
| -54 | 10 | `k_hjuh_p05`, `k_hjuh_p22`, `k_pman_murder09` |
| +10 | 10 | `k_cht_n_bast`, `k_cht_n_cand`, `k_cht_n_carth` |
| -230 | 9 | `k_pebn_loc01`, `k_pebn_loc02`, `k_pebn_loc03` |
| -147 | 9 | `k_con_com32`, `k_con_com33`, `k_con_com34` |
| -2 | 9 | `k_pman_comp05`, `k_pman_comp06`, `k_pman_comp07` |
| +58 | 9 | `k_con_comactive`, `k_con_comrep`, `k_con_comver31` |
| -208 | 7 | `k_pman_28d_cmp02`, `k_pman_comp04`, `k_pman_murder97` |
| +18 | 7 | `k_pman_28b_at01`, `k_pman_28b_at02`, `k_pman_airlock03` |
| -363 | 6 | `k_act_com32`, `k_act_com33`, `k_act_com33a` |
| -232 | 6 | `k_con_neutral`, `k_pman_28d_cmp01`, `k_swg_hk4713` |
| -132 | 6 | `k_act_darkhigh`, `k_act_darkmed`, `k_act_darksml` |
| -108 | 6 | `k_pman_payrace_1`, `k_pman_payrace_2`, `k_pman_payrace_3` |
| -38 | 6 | `k_act_exitarea`, `k_act_exitarea2`, `k_con_repair` |
| -10 | 6 | `k_con_inrange`, `k_hzal_beenkashy`, `k_hzal_chuundead` |
| -278 | 5 | `k_hjol_spawn01`, `k_pman_door10`, `k_pman_door13` |
| -60 | 5 | `k_con_chrmiddle`, `k_hzal_atkash2`, `k_hzal_notkash2` |
| -310 | 4 | `k_pman_init01`, `k_pman_init10`, `k_pman_misc07` |
| -292 | 4 | `k_pman_28ac_init`, `k_pman_door06`, `k_pman_door07` |
| -264 | 4 | `k_con_perseasy`, `k_con_pershigh`, `k_con_persmed` |
| -222 | 4 | `k_pman_murder95`, `k_pman_murder9b`, `k_pman_murder9c` |
| -196 | 4 | `k_act_walkways`, `k_act_walkways2`, `k_pman_ww` |
| -94 | 4 | `k_ptat_dockpaid`, `k_ptat_nosalehk`, `k_ptat_salehk` |
| -64 | 4 | `k_pdan_kath03`, `k_pkas_freyyspwn`, `k_pman_planet22` |
| -44 | 4 | `k_pman_comp02`, `k_pman_comp31`, `k_pman_comp32` |
| -28 | 4 | `k_hen_canduser`, `k_pkas_randwalk`, `k_pman_suituse` |
| -300 | 3 | `k_ptat_pazzwin1`, `k_ptat_pazzwin2`, `k_ptat_pazzwin3` |
| -294 | 3 | `k_pman_26d_triga`, `k_pman_26d_trigb`, `k_pman_26d_trigc` |
| -244 | 3 | `k_pman_planet57`, `k_pman_planet58`, `k_pman_planet59` |
| -52 | 3 | `k_act_foeinsight`, `k_hen_spawn01`, `k_pend_door02` |
| -42 | 3 | `k_con_chrlow`, `k_con_intlow`, `k_con_lighthigh` |
| +2 | 3 | `k_pdan_jedi21`, `k_pend_door01`, `k_pman_door30` |
| +8 | 3 | `k_amb_prey_ude`, `k_pend_door08`, `k_pman_26a_aread` |
| -6634 | 2 | `k_act_caloatt`, `k_def_rependd` |
| -350 | 2 | `k_act_com31`, `k_sup_comshots` |
| -345 | 2 | `k_actq_com33`, `k_actq_com33a` |
| -304 | 2 | `k_pend_explode`, `k_pman_28d_trigb` |
| -298 | 2 | `k_pman_28d_triga`, `k_pman_survive02` |
| -160 | 2 | `k_sup_star_map`, `k_swg_jolee18` |
| -152 | 2 | `k_hcan_state22`, `k_ren_nextplanet` |
| -146 | 2 | `k_ren_leavlev`, `k_sup_guiopen` |
| -128 | 2 | `k_com_compart`, `k_ptat_nicowait` |
| -110 | 2 | `k_con_light`, `k_hjuh_p21` |
| -98 | 2 | `k_pman_racetwox`, `k_swg_hk4712` |
| -84 | 2 | `k_act_nospeak`, `k_pman_raceonex` |
| -76 | 2 | `k_pman_area_28a`, `k_pman_no_threat` |
| -58 | 2 | `k_pdan_stand02`, `k_ptat_gaffiwrit` |
| -24 | 2 | `k_hjuh_p07`, `k_pman_tatracern` |
| -4 | 2 | `k_pman_27_init01`, `k_swg_hk47upg01` |
| +16 | 2 | `k_ptat_krayttran`, `k_trg_senni` |
| +30 | 2 | `k_con_comver33a`, `k_con_comver34` |
| +32 | 2 | `k_ptar_sitharm`, `k_zon_catalog` |
| -8708 | 1 | `k_ai_master` |
| -7054 | 1 | `k_act_bandatt` |
| -6664 | 1 | `k_ptat_gammenemy` |
| -6630 | 1 | `k_act_atkonend` |
| -6606 | 1 | `k_act_xoratt` |
| -1913 | 1 | `k_sp1_generic` |
| -926 | 1 | `k_plc_tresdrdhig` |
| -852 | 1 | `k_bant_trig` |
| -830 | 1 | `k_pend_door06` |
| -668 | 1 | `k_trg_partyinit` |
| -372 | 1 | `k_pkas_freyyuser` |
| -354 | 1 | `k_hjol_userdef01` |
| -353 | 1 | `k_sup_gohawk` |
| -351 | 1 | `k_actq_com34` |
| -324 | 1 | `k_pman_miss01` |
| -320 | 1 | `k_pman_27_area` |
| -318 | 1 | `k_pman_init02` |
| -317 | 1 | `k_trp_generic` |
| -296 | 1 | `k_pman_door32` |
| -290 | 1 | `k_pman_door20` |
| -286 | 1 | `k_act_com44` |
| -284 | 1 | `k_pman_door11` |
| -276 | 1 | `k_pman_raceset_2` |
| -272 | 1 | `k_pman_raceset_4` |
| -252 | 1 | `k_def_calospn` |
| -240 | 1 | `k_pman_26a_area` |
| -218 | 1 | `k_def_bandspn` |
| -214 | 1 | `k_pman_26d_area` |
| -209 | 1 | `k_act_com45` |
| -192 | 1 | `k_pebn_swoop` |
| -180 | 1 | `k_pdan_iriaz_ud` |
| -178 | 1 | `k_ren_visionland` |
| -168 | 1 | `k_amb_predator01` |
| -156 | 1 | `k_hjuh_p07d` |
| -142 | 1 | `k_hjuh_p07l` |
| -138 | 1 | `k_hjuh_h12` |
| -137 | 1 | `k_act_com35` |
| -134 | 1 | `k_con_com31` |
| -130 | 1 | `k_hjuh_p15` |
| -125 | 1 | `k_con_tokens` |
| -124 | 1 | `k_con_dark` |
| -120 | 1 | `k_pman_persuad_f` |
| -118 | 1 | `k_swg_jolee12` |
| -114 | 1 | `k_hen_leadchng` |
| -104 | 1 | `k_swg_jolee14` |
| -90 | 1 | `k_def_buff` |
| -87 | 1 | `k_ptat_exitkrayt` |
| -82 | 1 | `k_hcan_state23` |
| -74 | 1 | `k_ptat_talktanis` |
| -72 | 1 | `k_ptat_gizkauser` |
| -68 | 1 | `k_hjuh_p23` |
| -66 | 1 | `k_swg_bastila02` |
| -34 | 1 | `k_pman_sonic` |
| -23 | 1 | `k_pla_actmap` |
| -22 | 1 | `k_con_flirt` |
| -14 | 1 | `k_unk_rancor` |
| +20 | 1 | `k_sup_healing` |
| +22 | 1 | `k_cheat_01` |
| +28 | 1 | `k_sup_bands` |
| +43 | 1 | `k_trg_stealth` |
| +44 | 1 | `k_pman_airlock02` |
| +60 | 1 | `k_act_startgas` |
| +70 | 1 | `k_trg_transfail1` |
| +80 | 1 | `k_amb_enemyflee` |
| +94 | 1 | `k_com_comrepr` |
| +98 | 1 | `k_act_comp01` |
| +106 | 1 | `k_act_electric` |
| +107 | 1 | `k_con_comhack` |
| +116 | 1 | `k_con_comver41` |
| +120 | 1 | `k_pman_murdera01` |
| +136 | 1 | `k_act_com41` |
| +141 | 1 | `k_trg_messengers` |
| +268 | 1 | `k_trg_partyheart` |
| +1205 | 1 | `k_qa` |
| +2093 | 1 | `k_ptat17aa_enter` |

## nwnrs-o3

nwnrs 0.0.1, nwnrs pack, --optimization O3

Of the 1731 outputs that differ from BioWare's bytes, 688 make the same engine calls with the same arguments and return the same result under the recording engine, 1007 behave differently, and 36 do not run.

Failed (23):

- **undeclared** (3): uses a name the shipped sources do not declare: a constant BioWare's own build had (TRASK_DOOR_JAMMED), or a Neverwinter Nights function KotOR's table lacks (StartConversation, ResistSpell)
  - `k_pend_door04`: ERROR nwnrs::pack: error=failed to compile k_pend_door04.nss: undefined identifier "TRASK_DOOR_JAMMED" (-622)
  - `nw_g0_conversat`: ERROR nwnrs::pack: error=failed to compile nw_g0_conversat.nss: undefined function "StartConversation" (-622)
  - `nw_s0_lghtnbolt`: ERROR nwnrs::pack: error=failed to compile nw_s0_lghtnbolt.nss: undefined function "ResistSpell" (-622)
- **stale-prototype** (15): BioWare compiled this against declarations that were not shipped: the shipped nwscript.nss prototype of AddPartyMember/RemovePartyMember, or k_inc_ebonhawk's forward declaration of EBO_PlayTakeOff, disagrees with the call or definition
  - `k_act_bastadd`: ERROR nwnrs::pack: error=failed to compile k_act_bastadd.nss: call to "AddPartyMember" did not supply enough parameters: 1 < 2 (-587)
  - `k_act_bastrmv`: ERROR nwnrs::pack: error=failed to compile k_act_bastrmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_canderrmv`: ERROR nwnrs::pack: error=failed to compile k_act_canderrmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_carthrmv`: ERROR nwnrs::pack: error=failed to compile k_act_carthrmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_hki47rmv`: ERROR nwnrs::pack: error=failed to compile k_act_hki47rmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_joleermv`: ERROR nwnrs::pack: error=failed to compile k_act_joleermv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_juhanirmv`: ERROR nwnrs::pack: error=failed to compile k_act_juhanirmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_makeitem`: ERROR nwnrs::pack: error=failed to compile k_act_makeitem.nss: function "EBO_PlayTakeOff" declaration differs from previous declaration (-617)
  - `k_act_missionrmv`: ERROR nwnrs::pack: error=failed to compile k_act_missionrmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_t3m3rmv`: ERROR nwnrs::pack: error=failed to compile k_act_t3m3rmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_act_zaalrmv`: ERROR nwnrs::pack: error=failed to compile k_act_zaalrmv.nss: parameter "nNPC" expects Int, got Object (-587)
  - `k_con_makeitem`: ERROR nwnrs::pack: error=failed to compile k_con_makeitem.nss: function "EBO_PlayTakeOff" declaration differs from previous declaration (-617)
  - `k_pebn_galaxy`: ERROR nwnrs::pack: error=failed to compile k_pebn_galaxy.nss: function "EBO_PlayTakeOff" declaration differs from previous declaration (-617)
  - `k_sup_galaxmap`: ERROR nwnrs::pack: error=failed to compile k_sup_galaxmap.nss: function "EBO_PlayTakeOff" declaration differs from previous declaration (-617)
  - `k_sup_galaxymap`: ERROR nwnrs::pack: error=failed to compile k_sup_galaxymap.nss: function "EBO_PlayTakeOff" declaration differs from previous declaration (-617)
- **not-a-script** (5): an include file with no entry point; there is nothing to compile
  - `k_inc_generic`: ERROR nwnrs::pack: error=failed to compile k_inc_generic.nss: script must define StartingConditional (-5182)
  - `k_inc_gensupport`: ERROR nwnrs::pack: error=failed to compile k_inc_gensupport.nss: script must define StartingConditional (-5182)
  - `k_inc_switch`: ERROR nwnrs::pack: error=failed to compile k_inc_switch.nss: script must define StartingConditional (-5182)
  - `k_inc_unk`: ERROR nwnrs::pack: error=failed to compile k_inc_unk.nss: script must define StartingConditional (-5182)
  - `k_inc_walkways`: ERROR nwnrs::pack: error=failed to compile k_inc_walkways.nss: script must define StartingConditional (-5182)

Different output (1731), by cause:

- **stale-include** (73): BioWare's bytecode was compiled before an include it uses last changed: the first difference is in an include's globals or functions (k_inc_utility's SW_CONSTANT_* globals, k_inc_generic's GN_TalentMasterRoutine, among others)
  `k_act_atkonend`, `k_act_caloatt`, `k_act_com31`, `k_act_com32`, `k_act_com33`, `k_act_com33a`, `k_act_com34`, `k_act_com35`, `k_act_com41`, `k_act_com42`, `k_act_com43`, `k_act_com45`, `k_act_comp01`, `k_act_electric`, `k_act_startgas`, `k_act_xoratt`, `k_actq_com33`, `k_actq_com33a`, `k_actq_com34`, `k_amb_enemyflee`, `k_amb_predator01`, `k_com_compart`, `k_com_comrepr`, `k_con_com31`, `k_con_com32`, `k_con_com33`, `k_con_com34`, `k_con_com35`, `k_con_com41`, `k_con_com42`, `k_con_com43`, `k_con_com44`, `k_con_com45`, `k_con_comactive`, `k_con_comhack`, `k_con_comrep`, `k_con_comver31`, `k_con_comver32`, `k_con_comver33`, `k_con_comver33a`, `k_con_comver34`, `k_con_comver41`, `k_con_comver42`, `k_con_comver43`, `k_con_comver44`, `k_con_comver45`, `k_con_tokens`, `k_def_rependd`, `k_pla_actmap`, `k_plc_tresciv`, `k_plc_trescorhig`, `k_plc_trescorlow`, `k_plc_trescormid`, `k_plc_tresdrdhig`, `k_plc_tresdrdlow`, `k_plc_tresdrdmid`, `k_plc_tresmilhig`, `k_plc_tresmillow`, `k_plc_tresmilmid`, `k_plc_tresrakat`, `k_plc_tresshahig`, `k_plc_tresshalow`, `k_plc_tresshamid`, `k_plc_tressndppl`, `k_ptat17aa_enter`, `k_ptat_gammenemy`, `k_sp1_generic`, `k_sup_star_map`, `k_trg_messengers`, `k_trg_partyheart`, `k_trg_stealth`, `k_zon_catalog`, `k_zon_control`
- **stale-bytecode** (8): BioWare's bytecode was compiled from an earlier version of this script's own source: the first difference is in the script, and it is a different call, constant or condition
  `k_ptat_exitkrayt`, `k_qa`, `k_sup_droid`, `k_sup_gohawk`, `k_sup_grenade`, `k_trg_senni`, `k_trg_transfail1`, `k_trp_generic`
- **conversation-twelve** (3): BioWare compiled this against the shipped twelve-argument ActionStartConversation, unlike most of its scripts. Identical with the shipped declarations
  `k_ai_master`, `k_pman_27_init01`, `k_trg_transfail`
- **conversation-eleven** (51): BioWare compiled this when ActionStartConversation took eleven arguments; the shipped declaration has twelve. Identical with --declarations bioware
  `k_bant_trig`, `k_def_bandspn`, `k_def_calodef`, `k_hjol_userdef01`, `k_pdan_elise_d`, `k_pdan_rapid01`, `k_pdan_rapid02`, `k_pdan_rapid03`, `k_pdan_rapid04`, `k_pdan_rapid05`, `k_pend_door01`, `k_pend_door02`, `k_pend_door03`, `k_pend_door08`, `k_pkas_freyyuser`, `k_pman_26a_area`, `k_pman_28ac_init`, `k_pman_at01`, `k_pman_at02`, `k_pman_door03`, `k_pman_door06`, `k_pman_door07`, `k_pman_door08`, `k_pman_door10`, `k_pman_door11`, `k_pman_door13`, `k_pman_door14`, `k_pman_door20`, `k_pman_door30`, `k_pman_init01`, `k_pman_init02`, `k_pman_init03`, `k_pman_init10`, `k_pman_judge1_d`, `k_pman_obj_asc`, `k_pman_obj_asc2`, `k_pman_obj_ascc`, `k_pman_obj_ascc2`, `k_pman_obj_ascn`, `k_pman_osc`, `k_pman_rapid01`, `k_pman_rapid02`, `k_pman_rapid03`, `k_pman_rapid04`, `k_pman_rapid05`, `k_pman_sitspeak`, `k_pman_survive01`, `k_ptar_sitharm`, `k_ptat_krayttran`, `k_swg_xor_user`, `k_trg_partyinit`
- **no known cause** (1596): `k_act_bandatt`, `k_act_canderadd`, `k_act_candstim01`, `k_act_candstim02`, `k_act_candstim03`, `k_act_carthadd`, `k_act_cmusic_off`, `k_act_cmusic_on`, `k_act_com44`, `k_act_darkhigh`, `k_act_darkmed`, `k_act_darksml`, `k_act_exitarea`, `k_act_exitarea2`, `k_act_foeinsight`, `k_act_heartoff`, `k_act_hearton`, `k_act_hk47add`, `k_act_hk47simul`, `k_act_joleeadd`, `k_act_juhaniadd`, `k_act_leaverun`, `k_act_lighthigh`, `k_act_lightmed`, `k_act_lightsml`, `k_act_mispaz`, `k_act_missionadd`, `k_act_nospeak`, `k_act_noswitch`, `k_act_npctalk`, `k_act_t3m3add`, `k_act_talktrue`, `k_act_walkways`, `k_act_walkways2`, `k_act_xordie`, `k_act_zaaladd`, `k_amb_prey_spawn`, `k_amb_prey_ude`, `k_cheat02`, `k_cheat_01`, `k_cht_n_bast`, `k_cht_n_cand`, `k_cht_n_carth`, `k_cht_n_hk47`, `k_cht_n_jolee`, `k_cht_n_juhani`, `k_cht_n_mission`, `k_cht_n_screen`, `k_cht_n_t3m4`, `k_cht_n_zaalbar`, `k_cht_p_dan`, `k_cht_p_end`, `k_cht_p_kas`, `k_cht_p_kor`, `k_cht_p_lev`, `k_cht_p_man`, `k_cht_p_sta`, `k_cht_p_tar`, `k_cht_p_tat`, `k_cht_p_unk`, `k_cht_v_levi`, `k_cht_v_mast`, `k_cht_v_star`, `k_con_at_ebon`, `k_con_atdant`, `k_con_atforge`, `k_con_atkash`, `k_con_atkorr`, `k_con_atlev`, `k_con_atmann`, `k_con_atspire`, `k_con_attaris`, `k_con_attat`, `k_con_atunk`, `k_con_bastnpm`, `k_con_bastpm`, `k_con_candernpm`, `k_con_canderpm`, `k_con_candstim01`, `k_con_carthnpm`, `k_con_carthpm`, `k_con_chrhigh`, `k_con_chrlow`, `k_con_chrmiddle`, `k_con_chrnormal`, `k_con_classdiff`, `k_con_classsame`, `k_con_dark`, `k_con_darkhigh`, `k_con_darklow`, `k_con_flirt`, `k_con_flirtno`, `k_con_fpershigh`, `k_con_fperslow`, `k_con_heal`, `k_con_hk47npm`, `k_con_hk47pm`, `k_con_inrange`, `k_con_inthigh`, `k_con_intlow`, `k_con_intnormal`, `k_con_isfemale`, `k_con_isjedi`, `k_con_ismale`, `k_con_isscout`, `k_con_issmug`, `k_con_issoldier`, `k_con_joleejoin`, `k_con_joleenpm`, `k_con_joleepm`, `k_con_juhanilive`, `k_con_juhaninpm`, `k_con_juhanipm`, `k_con_light`, `k_con_lighthigh`, `k_con_lightlow`, `k_con_missionnpm`, `k_con_missionpm`, `k_con_neutral`, `k_con_npctalk`, `k_con_perseasy`, `k_con_pershigh`, `k_con_persmed`, `k_con_repair`, `k_con_t3m3npm`, `k_con_t3m3pm`, `k_con_talkedto`, `k_con_zaalnpm`, `k_con_zaalpm`, `k_def_ambient`, `k_def_ambmob`, `k_def_attacked01`, `k_def_blocked01`, `k_def_buff`, `k_def_calospn`, `k_def_combend01`, `k_def_damage01`, `k_def_death01`, `k_def_dialogue01`, `k_def_disturb01`, `k_def_endconv`, `k_def_grenspn`, `k_def_heartbt01`, `k_def_pathfail01`, `k_def_percept01`, `k_def_repairsp`, `k_def_repuser`, `k_def_spawn01`, `k_def_spawndead`, `k_def_spellat01`, `k_def_userdef01`, `k_hbanter1`, `k_hbanter10`, `k_hbanter10x`, `k_hbanter11`, `k_hbanter11x`, `k_hbanter1x`, `k_hbanter2`, `k_hbanter2x`, `k_hbanter3`, `k_hbanter3x`, `k_hbanter4`, `k_hbanter4x`, `k_hbanter5`, `k_hbanter5x`, `k_hbanter6`, `k_hbanter6x`, `k_hbanter7`, `k_hbanter7x`, `k_hbanter8`, `k_hbanter8x`, `k_hbanter9`, `k_hbanter9x`, `k_hbas_final`, `k_hbas_playkash`, `k_hbas_playkorr`, `k_hbas_playman`, `k_hbas_playtat`, `k_hcan_state01`, `k_hcan_state02`, `k_hcan_state03`, `k_hcan_state04`, `k_hcan_state05`, `k_hcan_state06`, `k_hcan_state07`, `k_hcan_state08`, `k_hcan_state09`, `k_hcan_state10`, `k_hcan_state11`, `k_hcan_state12`, `k_hcan_state13`, `k_hcan_state14`, `k_hcan_state15`, `k_hcan_state16`, `k_hcan_state17`, `k_hcan_state18`, `k_hcan_state19`, `k_hcan_state20`, `k_hcan_state21`, `k_hcan_state22`, `k_hcan_state23`, `k_hcan_state24`, `k_hcan_state25`, `k_hcan_state26`, `k_hcan_state27`, `k_hcan_state28`, `k_hcan_state29`, `k_hcan_state30`, `k_hcan_state31`, `k_hcan_state32`, `k_hcan_state33`, `k_hcan_state34`, `k_hcan_state35`, `k_hcan_state36`, `k_hcan_state37`, `k_hcan_state38`, `k_hcan_state39`, `k_hcan_state40`, `k_hcan_state41`, `k_hcan_state42`, `k_hcan_state43`, `k_hcan_state44`, `k_hcan_state45`, `k_hen_attacked01`, `k_hen_blocked01`, `k_hen_candspwn`, `k_hen_canduser`, `k_hen_combend01`, `k_hen_damage01`, `k_hen_dialogue01`, `k_hen_enter5m`, `k_hen_exit5m`, `k_hen_heartbt01`, `k_hen_leadchng`, `k_hen_percept01`, `k_hen_retreat`, `k_hen_spawn01`, `k_hjol_spawn01`, `k_hjuh_com01`, `k_hjuh_d01`, `k_hjuh_d02`, `k_hjuh_h01`, `k_hjuh_h01x`, `k_hjuh_h02`, `k_hjuh_h03`, `k_hjuh_h04`, `k_hjuh_h05`, `k_hjuh_h06`, `k_hjuh_h07`, `k_hjuh_h08`, `k_hjuh_h09`, `k_hjuh_h10`, `k_hjuh_h10x`, `k_hjuh_h11`, `k_hjuh_h12`, `k_hjuh_h13`, `k_hjuh_p01`, `k_hjuh_p02`, `k_hjuh_p03`, `k_hjuh_p04`, `k_hjuh_p05`, `k_hjuh_p06`, `k_hjuh_p07`, `k_hjuh_p07d`, `k_hjuh_p07l`, `k_hjuh_p08`, `k_hjuh_p09`, `k_hjuh_p10`, `k_hjuh_p10x`, `k_hjuh_p10y`, `k_hjuh_p11`, `k_hjuh_p12`, `k_hjuh_p13`, `k_hjuh_p14`, `k_hjuh_p15`, `k_hjuh_p16`, `k_hjuh_p17`, `k_hjuh_p18`, `k_hjuh_p19`, `k_hjuh_p20`, `k_hjuh_p21`, `k_hjuh_p22`, `k_hjuh_p23`, `k_hjuh_p24`, `k_hjuh_p25`, `k_hjuh_p26`, `k_hjuh_p27`, `k_hjuh_p28`, `k_hjuh_s01`, `k_hjuh_s02`, `k_hjuh_s03`, `k_hjuh_s04`, `k_hjuh_s05`, `k_hjuh_s06`, `k_hjuh_s07`, `k_hjuh_w01`, `k_hjuh_w02`, `k_hjuh_w03`, `k_hjuh_w04`, `k_hjuh_w05`, `k_hjuh_w06`, `k_hjuh_w07`, `k_hjuh_w08`, `k_hjuh_w09`, `k_hjuh_w10`, `k_hmis_misangry`, `k_hmis_misangryx`, `k_hmis_paz`, `k_hmis_planet`, `k_hmis_talk0`, `k_hmis_talk0+`, `k_hmis_talk1`, `k_hmis_talk10`, `k_hmis_talk10lev`, `k_hmis_talk10x`, `k_hmis_talk11`, `k_hmis_talk11lev`, `k_hmis_talk11x`, `k_hmis_talk12`, `k_hmis_talk12x`, `k_hmis_talk13x`, `k_hmis_talk14`, `k_hmis_talk14x`, `k_hmis_talk15`, `k_hmis_talk15x`, `k_hmis_talk1lev`, `k_hmis_talk1x`, `k_hmis_talk2x`, `k_hmis_talk3`, `k_hmis_talk3lev`, `k_hmis_talk3x`, `k_hmis_talk4x`, `k_hmis_talk5`, `k_hmis_talk5x`, `k_hmis_talk6`, `k_hmis_talk6-8`, `k_hmis_talk6lev`, `k_hmis_talk6x`, `k_hmis_talk7`, `k_hmis_talk7lev`, `k_hmis_talk7x`, `k_hmis_talk8`, `k_hmis_talk8x`, `k_hmis_talk9`, `k_hmis_talk99`, `k_hmis_talk99x`, `k_hmis_talk9x`, `k_hmis_talkrev`, `k_hmis_tardest`, `k_hmis_tardestx`, `k_hmis_zaalbar`, `k_hxor_state01`, `k_hxor_state02`, `k_hxor_state03`, `k_hzal_atkash1`, `k_hzal_atkash2`, `k_hzal_beenkashy`, `k_hzal_chuundead`, `k_hzal_freyydead`, `k_hzal_notkash1`, `k_hzal_notkash2`, `k_hzal_talk0`, `k_hzal_talk_1up`, `k_hzal_talkmad_c`, `k_hzal_talkmad_s`, `k_hzal_talkset_1`, `k_messenger_plot`, `k_pdan_anger01`, `k_pdan_anger02`, `k_pdan_anger03`, `k_pdan_anger04`, `k_pdan_anger05`, `k_pdan_anger06`, `k_pdan_anger07`, `k_pdan_anger08`, `k_pdan_anger09`, `k_pdan_anger10`, `k_pdan_anger11`, `k_pdan_anger12`, `k_pdan_anger13`, `k_pdan_anger14`, `k_pdan_anger15`, `k_pdan_anger16`, `k_pdan_anger17`, `k_pdan_anger18`, `k_pdan_anger19`, `k_pdan_anger20`, `k_pdan_bastial05`, `k_pdan_bastila01`, `k_pdan_bastila02`, `k_pdan_bastila03`, `k_pdan_bastila04`, `k_pdan_bastila05`, `k_pdan_bastila06`, `k_pdan_bastila07`, `k_pdan_bastila08`, `k_pdan_bastila09`, `k_pdan_bastila10`, `k_pdan_bastila11`, `k_pdan_belaya01`, `k_pdan_belaya02`, `k_pdan_cander01`, `k_pdan_cander02`, `k_pdan_cander03`, `k_pdan_carth01`, `k_pdan_carth02`, `k_pdan_carth03`, `k_pdan_carth04`, `k_pdan_casus01`, `k_pdan_casus02`, `k_pdan_casus03`, `k_pdan_casus04`, `k_pdan_casus05`, `k_pdan_destself`, `k_pdan_drandom02`, `k_pdan_drandom03`, `k_pdan_drandom04`, `k_pdan_drandom05`, `k_pdan_drandom06`, `k_pdan_drandom07`, `k_pdan_drandom08`, `k_pdan_droid01`, `k_pdan_droid02`, `k_pdan_droid03`, `k_pdan_droid04`, `k_pdan_droid05`, `k_pdan_droid06`, `k_pdan_droid07`, `k_pdan_droid08`, `k_pdan_droid09`, `k_pdan_droid10`, `k_pdan_droid11`, `k_pdan_droid12`, `k_pdan_droid13`, `k_pdan_droid14`, `k_pdan_droid15`, `k_pdan_droid50`, `k_pdan_iriaz_sp`, `k_pdan_iriaz_ud`, `k_pdan_jedi01`, `k_pdan_jedi02`, `k_pdan_jedi03`, `k_pdan_jedi04`, `k_pdan_jedi05`, `k_pdan_jedi06`, `k_pdan_jedi07`, `k_pdan_jedi08`, `k_pdan_jedi09`, `k_pdan_jedi10`, `k_pdan_jedi11`, `k_pdan_jedi12`, `k_pdan_jedi13`, `k_pdan_jedi14`, `k_pdan_jedi15`, `k_pdan_jedi16`, `k_pdan_jedi17`, `k_pdan_jedi18`, `k_pdan_jedi19`, `k_pdan_jedi20`, `k_pdan_jedi21`, `k_pdan_jedi22`, `k_pdan_jedi23`, `k_pdan_jedi24`, `k_pdan_jedi25`, `k_pdan_jedi26`, `k_pdan_jedi27`, `k_pdan_jedi28`, `k_pdan_jedi29`, `k_pdan_jedi30`, `k_pdan_jedi31`, `k_pdan_jedi32`, `k_pdan_jedi33`, `k_pdan_jedi34`, `k_pdan_jedi35`, `k_pdan_jedi36`, `k_pdan_jedi37`, `k_pdan_jedi38`, `k_pdan_jedi39`, `k_pdan_jedi40`, `k_pdan_juhani01`, `k_pdan_juhani02`, `k_pdan_juhani03`, `k_pdan_juhani04`, `k_pdan_juhani05`, `k_pdan_juhani06`, `k_pdan_juhani07`, `k_pdan_juhani08`, `k_pdan_juhani09`, `k_pdan_juhani10`, `k_pdan_juhani11`, `k_pdan_juhani12`, `k_pdan_juhani15`, `k_pdan_juhani16`, `k_pdan_juhani17`, `k_pdan_juhani18`, `k_pdan_juhani19`, `k_pdan_juhani20`, `k_pdan_kath01`, `k_pdan_kath02`, `k_pdan_kath03`, `k_pdan_kath04`, `k_pdan_mand01`, `k_pdan_mand02`, `k_pdan_mand03`, `k_pdan_mand04`, `k_pdan_mand05`, `k_pdan_mand06`, `k_pdan_mand07`, `k_pdan_mand08`, `k_pdan_mand09`, `k_pdan_mand10`, `k_pdan_mand11`, `k_pdan_mand12`, `k_pdan_mand51`, `k_pdan_mand52`, `k_pdan_mission01`, `k_pdan_mission02`, `k_pdan_murder01`, `k_pdan_murder02`, `k_pdan_murder03`, `k_pdan_murder04`, `k_pdan_murder05`, `k_pdan_murder06`, `k_pdan_murder07`, `k_pdan_murder08`, `k_pdan_murder09`, `k_pdan_murder10`, `k_pdan_murder11`, `k_pdan_murder12`, `k_pdan_murder13`, `k_pdan_murder14`, `k_pdan_murder15`, `k_pdan_murder16`, `k_pdan_murder17`, `k_pdan_murder18`, `k_pdan_murder19`, `k_pdan_murder20`, `k_pdan_murder21`, `k_pdan_murder22`, `k_pdan_murder23`, `k_pdan_murder24`, `k_pdan_murder25`, `k_pdan_murder26`, `k_pdan_murder27`, `k_pdan_murder28`, `k_pdan_murder29`, `k_pdan_murder30`, `k_pdan_murder31`, `k_pdan_murder32`, `k_pdan_murder33`, `k_pdan_murder34`, `k_pdan_murder35`, `k_pdan_murder36`, `k_pdan_murder37`, `k_pdan_murder38`, `k_pdan_murder39`, `k_pdan_murder40`, `k_pdan_murder41`, `k_pdan_murder42`, `k_pdan_murder43`, `k_pdan_murder44`, `k_pdan_murder45`, `k_pdan_murder46`, `k_pdan_murder47`, `k_pdan_murder48`, `k_pdan_murder49`, `k_pdan_murder50`, `k_pdan_murder51`, `k_pdan_murder52`, `k_pdan_murder53`, `k_pdan_murder54`, `k_pdan_murder55`, `k_pdan_murder56`, `k_pdan_murder57`, `k_pdan_murder58`, `k_pdan_murder59`, `k_pdan_murder60`, `k_pdan_murder61`, `k_pdan_murder62`, `k_pdan_murder63`, `k_pdan_murder64`, `k_pdan_murder65`, `k_pdan_murder66`, `k_pdan_murder67`, `k_pdan_murder68`, `k_pdan_murder69`, `k_pdan_murder70`, `k_pdan_murder71`, `k_pdan_murder72`, `k_pdan_murder73`, `k_pdan_murder74`, `k_pdan_murder75`, `k_pdan_murder76`, `k_pdan_planet01`, `k_pdan_planet02`, `k_pdan_planet03`, `k_pdan_planet04`, `k_pdan_planet05`, `k_pdan_planet06`, `k_pdan_planet08`, `k_pdan_planet12`, `k_pdan_planet14`, `k_pdan_planet15`, `k_pdan_planet16`, `k_pdan_romance01`, `k_pdan_romance02`, `k_pdan_romance03`, `k_pdan_romance04`, `k_pdan_romance05`, `k_pdan_romance06`, `k_pdan_romance07`, `k_pdan_romance08`, `k_pdan_romance09`, `k_pdan_romance10`, `k_pdan_romance11`, `k_pdan_romance12`, `k_pdan_romance13`, `k_pdan_romance14`, `k_pdan_romance15`, `k_pdan_romance16`, `k_pdan_romance17`, `k_pdan_romance18`, `k_pdan_romance19`, `k_pdan_romance20`, `k_pdan_romance21`, `k_pdan_romance22`, `k_pdan_romance23`, `k_pdan_romance24`, `k_pdan_romance25`, `k_pdan_romance26`, `k_pdan_romance27`, `k_pdan_romance28`, `k_pdan_romance29`, `k_pdan_romance30`, `k_pdan_romance31`, `k_pdan_romance32`, `k_pdan_romance33`, `k_pdan_romance34`, `k_pdan_romance35`, `k_pdan_romance36`, `k_pdan_romance37`, `k_pdan_romance38`, `k_pdan_romance39`, `k_pdan_romance40`, `k_pdan_romance41`, `k_pdan_romance42`, `k_pdan_stand01`, `k_pdan_stand02`, `k_pdan_stand03`, `k_pebn_ctaris01`, `k_pebn_ctaris02`, `k_pebn_ctaris03`, `k_pebn_ctaris04`, `k_pebn_ctaris05`, `k_pebn_ctaris06`, `k_pebn_ctaris07`, `k_pebn_ctaris08`, `k_pebn_ctaris09`, `k_pebn_gotaris`, `k_pebn_loc01`, `k_pebn_loc02`, `k_pebn_loc03`, `k_pebn_loc04`, `k_pebn_loc05`, `k_pebn_loc06`, `k_pebn_loc07`, `k_pebn_loc08`, `k_pebn_loc09`, `k_pebn_swoop`, `k_pend_door05`, `k_pend_door06`, `k_pend_door07`, `k_pend_door09`, `k_pend_explode`, `k_pend_rumble01`, `k_phkd_fighter`, `k_pkas_chuundead`, `k_pkas_enemyfac`, `k_pkas_freyydead`, `k_pkas_freyyhelp`, `k_pkas_freyyspwn`, `k_pkas_hashilt`, `k_pkas_hassword`, `k_pkas_heart_on`, `k_pkas_jaarakblt`, `k_pkas_jaaraksus`, `k_pkas_jaardead`, `k_pkas_jaarfreed`, `k_pkas_nohilt`, `k_pkas_nosword`, `k_pkas_pause3sec`, `k_pkas_randwalk`, `k_pkas_takehilt`, `k_pman_26a_aread`, `k_pman_26c_area`, `k_pman_26d_area`, `k_pman_26d_triga`, `k_pman_26d_trigb`, `k_pman_26d_trigc`, `k_pman_27_area`, `k_pman_28b_at01`, `k_pman_28b_at02`, `k_pman_28c_sur01`, `k_pman_28d_cmp01`, `k_pman_28d_cmp02`, `k_pman_28d_triga`, `k_pman_28d_trigb`, `k_pman_aioff`, `k_pman_aion`, `k_pman_airlock01`, `k_pman_airlock02`, `k_pman_airlock03`, `k_pman_amb_9`, `k_pman_ambient01`, `k_pman_ambient02`, `k_pman_ambient03`, `k_pman_ambient04`, `k_pman_ambient05`, `k_pman_ambient06`, `k_pman_ambient07`, `k_pman_ambient08`, `k_pman_area_28a`, `k_pman_arg01`, `k_pman_argrep_d`, `k_pman_argsith_d`, `k_pman_beast_9`, `k_pman_beast_d`, `k_pman_body01`, `k_pman_casamad_s`, `k_pman_casamad_y`, `k_pman_casname_n`, `k_pman_casname_s`, `k_pman_champ_f01`, `k_pman_champ_f02`, `k_pman_champ_f03`, `k_pman_champ_f04`, `k_pman_champ_m01`, `k_pman_champ_m02`, `k_pman_champ_m03`, `k_pman_champ_m04`, `k_pman_champ_qui`, `k_pman_comp01`, `k_pman_comp02`, `k_pman_comp03`, `k_pman_comp04`, `k_pman_comp05`, `k_pman_comp06`, `k_pman_comp07`, `k_pman_comp08`, `k_pman_comp09`, `k_pman_comp10`, `k_pman_comp11`, `k_pman_comp30`, `k_pman_comp31`, `k_pman_comp32`, `k_pman_comp33`, `k_pman_comp34`, `k_pman_comp35`, `k_pman_door01`, `k_pman_door02`, `k_pman_door04`, `k_pman_door05`, `k_pman_door12`, `k_pman_door31`, `k_pman_door32`, `k_pman_door40`, `k_pman_false`, `k_pman_firixa_d`, `k_pman_guard01`, `k_pman_hukname_n`, `k_pman_hukname_s`, `k_pman_huktmad_1`, `k_pman_huktmad_y`, `k_pman_ignore`, `k_pman_ignus01`, `k_pman_locked01`, `k_pman_locked02`, `k_pman_lostset_n`, `k_pman_lostset_q`, `k_pman_lostset_y`, `k_pman_main01`, `k_pman_misc01`, `k_pman_misc02`, `k_pman_misc03`, `k_pman_misc04`, `k_pman_misc05`, `k_pman_misc06`, `k_pman_misc07`, `k_pman_misc08`, `k_pman_misc09`, `k_pman_misc10`, `k_pman_misc11`, `k_pman_misc12`, `k_pman_misc13`, `k_pman_misc14`, `k_pman_misc15`, `k_pman_misc16`, `k_pman_misc17`, `k_pman_misc18`, `k_pman_misc19`, `k_pman_misc20`, `k_pman_misc21`, `k_pman_misc22`, `k_pman_misc23`, `k_pman_misc24`, `k_pman_misc25`, `k_pman_misc26`, `k_pman_misc27`, `k_pman_misc28`, `k_pman_misc29`, `k_pman_misc30`, `k_pman_misc31`, `k_pman_misc32`, `k_pman_misc33`, `k_pman_misc34`, `k_pman_misc35`, `k_pman_misc36`, `k_pman_misc37`, `k_pman_misc38`, `k_pman_misc39`, `k_pman_misc40`, `k_pman_misc41`, `k_pman_misc42`, `k_pman_misc43`, `k_pman_misc44`, `k_pman_misc45`, `k_pman_misc46`, `k_pman_misc47`, `k_pman_misc48`, `k_pman_misc49`, `k_pman_misc50`, `k_pman_misc51`, `k_pman_misc52`, `k_pman_misc53`, `k_pman_misc54`, `k_pman_misc55`, `k_pman_misc56`, `k_pman_misc57`, `k_pman_misc58`, `k_pman_misc59`, `k_pman_misc60`, `k_pman_misc61`, `k_pman_misc62`, `k_pman_misc70`, `k_pman_misc71`, `k_pman_misc72`, `k_pman_misc73`, `k_pman_misc74`, `k_pman_misc75`, `k_pman_misc76`, `k_pman_miss01`, `k_pman_missing01`, `k_pman_missing02`, `k_pman_missing03`, `k_pman_missing04`, `k_pman_missing05`, `k_pman_missing06`, `k_pman_missing07`, `k_pman_missing08`, `k_pman_missing09`, `k_pman_missing10`, `k_pman_missing11`, `k_pman_missing12`, `k_pman_missing13`, `k_pman_missing14`, `k_pman_missing15`, `k_pman_missing16`, `k_pman_missing17`, `k_pman_missing18`, `k_pman_missing19`, `k_pman_missing20`, `k_pman_missing21`, `k_pman_missing22`, `k_pman_missing23`, `k_pman_missing24`, `k_pman_missing25`, `k_pman_missing26`, `k_pman_missing27`, `k_pman_missing28`, `k_pman_missing29`, `k_pman_missing30`, `k_pman_missing31`, `k_pman_missing32`, `k_pman_missing33`, `k_pman_missing34`, `k_pman_missing35`, `k_pman_missing36`, `k_pman_missing37`, `k_pman_missing38`, `k_pman_missing39`, `k_pman_missing40`, `k_pman_missing41`, `k_pman_missing42`, `k_pman_missing43`, `k_pman_murder01`, `k_pman_murder02`, `k_pman_murder03`, `k_pman_murder04`, `k_pman_murder05`, `k_pman_murder06`, `k_pman_murder07`, `k_pman_murder08`, `k_pman_murder09`, `k_pman_murder10`, `k_pman_murder11`, `k_pman_murder12`, `k_pman_murder13`, `k_pman_murder14`, `k_pman_murder15`, `k_pman_murder16`, `k_pman_murder17`, `k_pman_murder18`, `k_pman_murder19`, `k_pman_murder20`, `k_pman_murder21`, `k_pman_murder22`, `k_pman_murder23`, `k_pman_murder24`, `k_pman_murder25`, `k_pman_murder26`, `k_pman_murder27`, `k_pman_murder28`, `k_pman_murder29`, `k_pman_murder30`, `k_pman_murder31`, `k_pman_murder32`, `k_pman_murder33`, `k_pman_murder34`, `k_pman_murder35`, `k_pman_murder36`, `k_pman_murder37`, `k_pman_murder38`, `k_pman_murder39`, `k_pman_murder40`, `k_pman_murder41`, `k_pman_murder42`, `k_pman_murder43`, `k_pman_murder44`, `k_pman_murder45`, `k_pman_murder46`, `k_pman_murder47`, `k_pman_murder48`, `k_pman_murder49`, `k_pman_murder50`, `k_pman_murder51`, `k_pman_murder52`, `k_pman_murder53`, `k_pman_murder54`, `k_pman_murder55`, `k_pman_murder56`, `k_pman_murder57`, `k_pman_murder58`, `k_pman_murder59`, `k_pman_murder60`, `k_pman_murder61`, `k_pman_murder64`, `k_pman_murder65`, `k_pman_murder66`, `k_pman_murder67`, `k_pman_murder68`, `k_pman_murder69`, `k_pman_murder70`, `k_pman_murder71`, `k_pman_murder72`, `k_pman_murder73`, `k_pman_murder74`, `k_pman_murder75`, `k_pman_murder76`, `k_pman_murder77`, `k_pman_murder78`, `k_pman_murder79`, `k_pman_murder80`, `k_pman_murder81`, `k_pman_murder82`, `k_pman_murder83`, `k_pman_murder84`, `k_pman_murder85`, `k_pman_murder86`, `k_pman_murder87`, `k_pman_murder88`, `k_pman_murder89`, `k_pman_murder90`, `k_pman_murder91`, `k_pman_murder92`, `k_pman_murder93`, `k_pman_murder94`, `k_pman_murder95`, `k_pman_murder96`, `k_pman_murder97`, `k_pman_murder98`, `k_pman_murder99`, `k_pman_murder9a`, `k_pman_murder9b`, `k_pman_murder9c`, `k_pman_murder9d`, `k_pman_murder9e`, `k_pman_murder9f`, `k_pman_murder9g`, `k_pman_murder9h`, `k_pman_murder9i`, `k_pman_murder9j`, `k_pman_murder9k`, `k_pman_murder9l`, `k_pman_murder9m`, `k_pman_murder9n`, `k_pman_murder9o`, `k_pman_murder9p`, `k_pman_murder9q`, `k_pman_murder9r`, `k_pman_murder9s`, `k_pman_murder9t`, `k_pman_murder9u`, `k_pman_murder9v`, `k_pman_murder9w`, `k_pman_murder9x`, `k_pman_murder9y`, `k_pman_murder9z`, `k_pman_murdera01`, `k_pman_murdera02`, `k_pman_murdera03`, `k_pman_murdera04`, `k_pman_murdera05`, `k_pman_murdera06`, `k_pman_murdera07`, `k_pman_murdera08`, `k_pman_murdera09`, `k_pman_murdera11`, `k_pman_no_threat`, `k_pman_notpaid_c`, `k_pman_notpaid_n`, `k_pman_notpaid_s`, `k_pman_npcstart`, `k_pman_npcstop`, `k_pman_patrol_d`, `k_pman_pause2sec`, `k_pman_payrace_1`, `k_pman_payrace_2`, `k_pman_payrace_3`, `k_pman_pazaak01`, `k_pman_pazaak02`, `k_pman_pazaak03`, `k_pman_pazaak04`, `k_pman_pazaak05`, `k_pman_pazaak06`, `k_pman_pazaak07`, `k_pman_pazaak08`, `k_pman_pazaak09`, `k_pman_pazaak10`, `k_pman_pazaak11`, `k_pman_persuad_f`, `k_pman_persuad_s`, `k_pman_persuad_y`, `k_pman_planet01`, `k_pman_planet02`, `k_pman_planet03`, `k_pman_planet04`, `k_pman_planet05`, `k_pman_planet06`, `k_pman_planet07`, `k_pman_planet08`, `k_pman_planet09`, `k_pman_planet10`, `k_pman_planet11`, `k_pman_planet12`, `k_pman_planet13`, `k_pman_planet14`, `k_pman_planet15`, `k_pman_planet16`, `k_pman_planet17`, `k_pman_planet18`, `k_pman_planet19`, `k_pman_planet20`, `k_pman_planet21`, `k_pman_planet22`, `k_pman_planet23`, `k_pman_planet24`, `k_pman_planet25`, `k_pman_planet26`, `k_pman_planet27`, `k_pman_planet28`, `k_pman_planet29`, `k_pman_planet30`, `k_pman_planet31`, `k_pman_planet32`, `k_pman_planet33`, `k_pman_planet34`, `k_pman_planet35`, `k_pman_planet36`, `k_pman_planet37`, `k_pman_planet38`, `k_pman_planet39`, `k_pman_planet40`, `k_pman_planet41`, `k_pman_planet42`, `k_pman_planet43`, `k_pman_planet44`, `k_pman_planet45`, `k_pman_planet46`, `k_pman_planet47`, `k_pman_planet48`, `k_pman_planet49`, `k_pman_planet50`, `k_pman_planet51`, `k_pman_planet52`, `k_pman_planet53`, `k_pman_planet54`, `k_pman_planet55`, `k_pman_planet56`, `k_pman_planet57`, `k_pman_planet58`, `k_pman_planet59`, `k_pman_planet60`, `k_pman_planet61`, `k_pman_planet62`, `k_pman_planet63`, `k_pman_planet64`, `k_pman_planet65`, `k_pman_planet66`, `k_pman_planet67`, `k_pman_planet68`, `k_pman_planet69`, `k_pman_planet70`, `k_pman_planet71`, `k_pman_planet72`, `k_pman_planet73`, `k_pman_planet74`, `k_pman_planet75`, `k_pman_planet76`, `k_pman_planet77`, `k_pman_planet78`, `k_pman_planet79`, `k_pman_planet80`, `k_pman_planet81`, `k_pman_planet82`, `k_pman_planet83`, `k_pman_planet84`, `k_pman_planet85`, `k_pman_planet86`, `k_pman_planet87`, `k_pman_port01`, `k_pman_puzzle01`, `k_pman_puzzle02`, `k_pman_puzzle03`, `k_pman_puzzle04`, `k_pman_puzzle05`, `k_pman_puzzle06`, `k_pman_puzzle07`, `k_pman_puzzle08`, `k_pman_puzzle09`, `k_pman_puzzle10`, `k_pman_puzzle11`, `k_pman_puzzle12`, `k_pman_puzzle13`, `k_pman_puzzle14`, `k_pman_qreturn_n`, `k_pman_qreturn_s`, `k_pman_queecham2`, `k_pman_queechamp`, `k_pman_queelostp`, `k_pman_queemad_s`, `k_pman_queemad_y`, `k_pman_queenotpc`, `k_pman_queeorpcy`, `k_pman_queequit`, `k_pman_quename_n`, `k_pman_quename_s`, `k_pman_quepaid_n`, `k_pman_quepaid_s`, `k_pman_quepaid_y`, `k_pman_quetell_n`, `k_pman_quetell_s`, `k_pman_quetell_y`, `k_pman_quleave_s`, `k_pman_quleave_y`, `k_pman_race_0_02`, `k_pman_race_0_03`, `k_pman_race_0_04`, `k_pman_race_0_q1`, `k_pman_race_0_q2`, `k_pman_race_0_q3`, `k_pman_race_0_q4`, `k_pman_race_1_02`, `k_pman_race_1_03`, `k_pman_race_1_04`, `k_pman_race_2_02`, `k_pman_race_2_03`, `k_pman_race_2_04`, `k_pman_race_3_02`, `k_pman_race_3_03`, `k_pman_race_3_04`, `k_pman_race_s_01`, `k_pman_race_s_02`, `k_pman_race_s_03`, `k_pman_race_s_04`, `k_pman_race_x_02`, `k_pman_race_x_03`, `k_pman_race_x_04`, `k_pman_racecha_n`, `k_pman_racechamp`, `k_pman_racelosty`, `k_pman_racenone`, `k_pman_raceone`, `k_pman_raceonex`, `k_pman_raceset_1`, `k_pman_raceset_2`, `k_pman_raceset_3`, `k_pman_raceset_4`, `k_pman_racetwo`, `k_pman_racetwox`, `k_pman_random01`, `k_pman_random02`, `k_pman_random03`, `k_pman_random04`, `k_pman_random05`, `k_pman_random06`, `k_pman_random07`, `k_pman_random08`, `k_pman_random09`, `k_pman_random10`, `k_pman_random11`, `k_pman_random12`, `k_pman_random13`, `k_pman_random14`, `k_pman_random15`, `k_pman_random16`, `k_pman_random17`, `k_pman_random18`, `k_pman_random19`, `k_pman_random20`, `k_pman_random21`, `k_pman_random22`, `k_pman_random23`, `k_pman_random24`, `k_pman_random25`, `k_pman_random26`, `k_pman_random27`, `k_pman_random28`, `k_pman_random29`, `k_pman_random30`, `k_pman_random31`, `k_pman_random32`, `k_pman_random33`, `k_pman_reward01`, `k_pman_rodian01`, `k_pman_rodian02`, `k_pman_rodian03`, `k_pman_rodian04`, `k_pman_rodian05`, `k_pman_rodian06`, `k_pman_rodian07`, `k_pman_selapr5_d`, `k_pman_selapr_d`, `k_pman_shark01`, `k_pman_shasa01`, `k_pman_shasa02`, `k_pman_shasa03`, `k_pman_shield_9`, `k_pman_shield_d`, `k_pman_sith01`, `k_pman_sith02`, `k_pman_sithmas01`, `k_pman_sonic`, `k_pman_speed01`, `k_pman_starmap`, `k_pman_suituse`, `k_pman_sur01`, `k_pman_sur02`, `k_pman_survive02`, `k_pman_swoop01`, `k_pman_swoop02`, `k_pman_swoop03`, `k_pman_swoop04`, `k_pman_swoop05`, `k_pman_swoop06`, `k_pman_swoop07`, `k_pman_swoop08`, `k_pman_swoop09`, `k_pman_swoop10`, `k_pman_swoop11`, `k_pman_swoop12`, `k_pman_swoop13`, `k_pman_swoop14`, `k_pman_swoop15`, `k_pman_swoop16`, `k_pman_swoop17`, `k_pman_swoop18`, `k_pman_swoop19`, `k_pman_swoop20`, `k_pman_swoop21`, `k_pman_swoop22`, `k_pman_swoop23`, `k_pman_swoop24`, `k_pman_swoop25`, `k_pman_swoop26`, `k_pman_swoop27`, `k_pman_swoop28`, `k_pman_swoop29`, `k_pman_swoop30`, `k_pman_swoop31`, `k_pman_swoop32`, `k_pman_swoop33`, `k_pman_swoop34`, `k_pman_swoop35`, `k_pman_swoop36`, `k_pman_swoop37`, `k_pman_swoop38`, `k_pman_swoop39`, `k_pman_swoop40`, `k_pman_swoop41`, `k_pman_tatracern`, `k_pman_tatracery`, `k_pman_tech01`, `k_pman_trial01`, `k_pman_trial02`, `k_pman_ww`, `k_pman_ye_threat`, `k_ptar_sithdis`, `k_ptar_sithpaper`, `k_ptat_1gamewon`, `k_ptat_2gamewon`, `k_ptat_3gamewon`, `k_ptat_amb1`, `k_ptat_angrgurke`, `k_ptat_baddeal`, `k_ptat_beenkash`, `k_ptat_bounty_n`, `k_ptat_bounty_y`, `k_ptat_bountyset`, `k_ptat_bullynico`, `k_ptat_canhunt`, `k_ptat_celisdeal`, `k_ptat_checkmad`, `k_ptat_custmove`, `k_ptat_dockpaid`, `k_ptat_dockpay`, `k_ptat_dorakexit`, `k_ptat_dorakname`, `k_ptat_dorakwho`, `k_ptat_drandom09`, `k_ptat_drandom10`, `k_ptat_drandom11`, `k_ptat_drandom12`, `k_ptat_dunesea`, `k_ptat_gaffiwrit`, `k_ptat_gammbad`, `k_ptat_gammbribe`, `k_ptat_gammgone`, `k_ptat_gammhunt`, `k_ptat_gammleave`, `k_ptat_gammmean`, `k_ptat_gammwarn`, `k_ptat_gandmad`, `k_ptat_gandname`, `k_ptat_garmleave`, `k_ptat_garmname`, `k_ptat_garmwho`, `k_ptat_gateperm`, `k_ptat_givepearl`, `k_ptat_givewrit`, `k_ptat_gizkaspwn`, `k_ptat_gizkauser`, `k_ptat_gooddeal`, `k_ptat_gurkemad`, `k_ptat_has_k_map`, `k_ptat_hasgaffi`, `k_ptat_haspearl`, `k_ptat_haspermit`, `k_ptat_haswrit`, `k_ptat_huntask_n`, `k_ptat_huntask_y`, `k_ptat_koname_ch`, `k_ptat_konameset`, `k_ptat_leave`, `k_ptat_losscheck`, `k_ptat_m_office`, `k_ptat_movekrayt`, `k_ptat_namecheck`, `k_ptat_nicenico`, `k_ptat_nicofree`, `k_ptat_nicohappy`, `k_ptat_nicoinfo`, `k_ptat_niconews`, `k_ptat_nicotalk`, `k_ptat_nicowait`, `k_ptat_no_k_map`, `k_ptat_nohkinfo`, `k_ptat_nonicotlk`, `k_ptat_nopermit`, `k_ptat_norespon`, `k_ptat_nosalehk`, `k_ptat_notracer`, `k_ptat_nowrit`, `k_ptat_officemad`, `k_ptat_pazzloss`, `k_ptat_pazzwin1`, `k_ptat_pazzwin2`, `k_ptat_pazzwin3`, `k_ptat_persyuka`, `k_ptat_racechamp`, `k_ptat_racenone`, `k_ptat_raceone`, `k_ptat_racer`, `k_ptat_racer_id`, `k_ptat_racetrial`, `k_ptat_racetwo`, `k_ptat_salehk`, `k_ptat_sandper`, `k_ptat_sethunt`, `k_ptat_shariwait`, `k_ptat_sharleave`, `k_ptat_shartrust`, `k_ptat_soldhk`, `k_ptat_soldpearl`, `k_ptat_swoopdwn1`, `k_ptat_swoopdwn2`, `k_ptat_swoopdwn3`, `k_ptat_swoopdwn4`, `k_ptat_talktanis`, `k_ptat_taname_ch`, `k_ptat_tanameset`, `k_ptat_tanileave`, `k_ptat_tanisdead`, `k_ptat_tanisgave`, `k_ptat_tanismad`, `k_ptat_tatinfo_c`, `k_ptat_tatinfo_s`, `k_ptat_toldofhk`, `k_ptat_tuskpdone`, `k_ptat_yukaleave`, `k_ptat_yukanpers`, `k_ptat_yukarname`, `k_ptat_yukarwho`, `k_ptat_zoriiname`, `k_ptat_zoriiwho`, `k_ptat_zorileave`, `k_punk_cold`, `k_ren_leavlev`, `k_ren_levcapture`, `k_ren_levescape`, `k_ren_nextplanet`, `k_ren_starforge`, `k_ren_starland`, `k_ren_taris01`, `k_ren_taris02`, `k_ren_taris03`, `k_ren_turret`, `k_ren_turretld02`, `k_ren_turretload`, `k_ren_unkcrash`, `k_ren_unkturret`, `k_ren_visionland`, `k_sup_actmap`, `k_sup_bands`, `k_sup_comshots`, `k_sup_fear`, `k_sup_guiopen`, `k_sup_healing`, `k_sup_static`, `k_swg_bantforce`, `k_swg_bastdef`, `k_swg_bastevil`, `k_swg_bastforce1`, `k_swg_bastforce3`, `k_swg_bastforcex`, `k_swg_bastforcey`, `k_swg_bastila01`, `k_swg_bastila02`, `k_swg_bastila03`, `k_swg_bastila04`, `k_swg_bastila05`, `k_swg_bastila06`, `k_swg_bastila07`, `k_swg_bastila08`, `k_swg_bastila09`, `k_swg_bastila10`, `k_swg_bastila11`, `k_swg_bastila12`, `k_swg_bastila12a`, `k_swg_bastila13`, `k_swg_bastila14`, `k_swg_bastila15`, `k_swg_bastila16`, `k_swg_bastila17`, `k_swg_bastila18`, `k_swg_bastila19`, `k_swg_bastila20`, `k_swg_bastila20x`, `k_swg_bastila21`, `k_swg_bastila22`, `k_swg_bastila23`, `k_swg_bastila24`, `k_swg_bastila25`, `k_swg_bastila26`, `k_swg_bastila27`, `k_swg_bastila27x`, `k_swg_bastila28`, `k_swg_bastila29`, `k_swg_bastila30`, `k_swg_bastila31`, `k_swg_bastila32`, `k_swg_bastila99`, `k_swg_bastile17`, `k_swg_bastmovie`, `k_swg_carth01`, `k_swg_carth02`, `k_swg_carth03`, `k_swg_carth04`, `k_swg_carth05`, `k_swg_carth06`, `k_swg_carth07`, `k_swg_carth08`, `k_swg_carth09`, `k_swg_carth10`, `k_swg_carth11`, `k_swg_carth12`, `k_swg_carth13`, `k_swg_carth14`, `k_swg_carth15`, `k_swg_carth16`, `k_swg_carth17`, `k_swg_carth18`, `k_swg_carth19`, `k_swg_carth20`, `k_swg_carth21`, `k_swg_carth22`, `k_swg_carth23`, `k_swg_carth24`, `k_swg_carth25`, `k_swg_carth25x`, `k_swg_carth25y`, `k_swg_carth26`, `k_swg_carth99`, `k_swg_helena07`, `k_swg_helena08`, `k_swg_hk4700`, `k_swg_hk4701`, `k_swg_hk4702`, `k_swg_hk4703`, `k_swg_hk4704`, `k_swg_hk4705`, `k_swg_hk4706`, `k_swg_hk4707`, `k_swg_hk4708`, `k_swg_hk4709`, `k_swg_hk4710`, `k_swg_hk4711`, `k_swg_hk4712`, `k_swg_hk4712x`, `k_swg_hk4712y`, `k_swg_hk4713`, `k_swg_hk47fix`, `k_swg_hk47upg01`, `k_swg_hk47upg02`, `k_swg_hk47upg03`, `k_swg_hk47upg04`, `k_swg_jolee01`, `k_swg_jolee02`, `k_swg_jolee03`, `k_swg_jolee04`, `k_swg_jolee05`, `k_swg_jolee06`, `k_swg_jolee07`, `k_swg_jolee08`, `k_swg_jolee09`, `k_swg_jolee10`, `k_swg_jolee11`, `k_swg_jolee12`, `k_swg_jolee13`, `k_swg_jolee14`, `k_swg_jolee15`, `k_swg_jolee16`, `k_swg_jolee17`, `k_swg_jolee18`, `k_swg_jolee19`, `k_swg_jolee20`, `k_swg_jolee99`, `k_swg_joleerev`, `k_swg_replaykas`, `k_swg_replaykor`, `k_swg_replayman`, `k_swg_replaytat`, `k_swg_sasha01`, `k_swg_senni1`, `k_swg_senni2`, `k_swg_xor_def`, `k_swg_ziag01`, `k_trg_bandon`, `k_trg_calonord`, `k_trg_xorattack`, `k_unk_rancor`, `qa_calo`, `qa_lev`, `qa_map`, `qa_master`, `qa_planet`, `qa_setanim`

Size differences, largest clusters first:

| Bytes | Scripts | Examples |
| --- | --- | --- |
| -12 | 382 | `k_con_at_ebon`, `k_con_joleejoin`, `k_con_juhanilive` |
| +0 | 375 | `k_act_canderadd`, `k_act_carthadd`, `k_act_cmusic_off` |
| -26 | 143 | `k_hcan_state02`, `k_hcan_state03`, `k_hcan_state04` |
| -6 | 110 | `k_cht_v_levi`, `k_cht_v_mast`, `k_cht_v_star` |
| -40 | 63 | `k_act_talktrue`, `k_con_flirtno`, `k_con_fperslow` |
| -28 | 39 | `k_hen_canduser`, `k_pkas_randwalk`, `k_pman_main01` |
| -88 | 37 | `k_pman_casamad_y`, `k_pman_casname_n`, `k_pman_hukname_n` |
| -36 | 37 | `k_con_chrhigh`, `k_con_chrnormal`, `k_con_inthigh` |
| -266 | 32 | `k_pman_casamad_s`, `k_pman_casname_s`, `k_pman_hukname_s` |
| -32 | 31 | `k_pdan_droid15`, `k_pman_missing25`, `k_pman_missing29` |
| -46 | 21 | `k_hjuh_p28`, `k_hmis_planet`, `k_hzal_atkash1` |
| -18 | 19 | `k_con_atdant`, `k_con_atforge`, `k_con_atkash` |
| +6 | 19 | `k_con_classdiff`, `k_con_classsame`, `k_messenger_plot` |
| -50 | 18 | `k_pman_race_0_02`, `k_pman_race_0_03`, `k_pman_race_0_04` |
| -224 | 15 | `k_amb_prey_spawn`, `k_def_ambient`, `k_def_ambmob` |
| -56 | 15 | `k_con_darkhigh`, `k_con_darklow`, `k_con_lightlow` |
| -4866 | 14 | `k_plc_tresciv`, `k_plc_trescorhig`, `k_plc_trescorlow` |
| -20 | 13 | `k_con_bastpm`, `k_con_canderpm`, `k_con_carthpm` |
| -16 | 12 | `k_def_calodef`, `k_pkas_hashilt`, `k_pkas_hassword` |
| -70 | 11 | `k_def_repairsp`, `k_pman_champ_f02`, `k_pman_champ_f03` |
| +12 | 11 | `k_cht_p_dan`, `k_cht_p_end`, `k_cht_p_kas` |
| +24 | 11 | `k_act_candstim01`, `k_act_candstim02`, `k_act_candstim03` |
| -370 | 10 | `k_pdan_rapid01`, `k_pdan_rapid02`, `k_pdan_rapid03` |
| -96 | 10 | `k_pman_planet64`, `k_pman_planet65`, `k_pman_planet66` |
| -54 | 10 | `k_hjuh_p05`, `k_hjuh_p22`, `k_pman_murder09` |
| +10 | 10 | `k_cht_n_bast`, `k_cht_n_cand`, `k_cht_n_carth` |
| -230 | 9 | `k_pebn_loc01`, `k_pebn_loc02`, `k_pebn_loc03` |
| -163 | 9 | `k_con_com32`, `k_con_com33`, `k_con_com34` |
| -2 | 9 | `k_pman_comp05`, `k_pman_comp06`, `k_pman_comp07` |
| +58 | 9 | `k_con_comactive`, `k_con_comrep`, `k_con_comver31` |
| -208 | 7 | `k_pman_28d_cmp02`, `k_pman_comp04`, `k_pman_murder97` |
| +18 | 7 | `k_pman_28b_at01`, `k_pman_28b_at02`, `k_pman_airlock03` |
| -363 | 6 | `k_act_com32`, `k_act_com33`, `k_act_com33a` |
| -232 | 6 | `k_con_neutral`, `k_pman_28d_cmp01`, `k_swg_hk4713` |
| -132 | 6 | `k_act_darkhigh`, `k_act_darkmed`, `k_act_darksml` |
| -108 | 6 | `k_pman_payrace_1`, `k_pman_payrace_2`, `k_pman_payrace_3` |
| -38 | 6 | `k_act_exitarea`, `k_act_exitarea2`, `k_con_repair` |
| -10 | 6 | `k_con_inrange`, `k_hzal_beenkashy`, `k_hzal_chuundead` |
| -278 | 5 | `k_hjol_spawn01`, `k_pman_door10`, `k_pman_door13` |
| -60 | 5 | `k_con_chrmiddle`, `k_hzal_atkash2`, `k_hzal_notkash2` |
| -310 | 4 | `k_pman_init01`, `k_pman_init10`, `k_pman_misc07` |
| -292 | 4 | `k_pman_28ac_init`, `k_pman_door06`, `k_pman_door07` |
| -264 | 4 | `k_con_perseasy`, `k_con_pershigh`, `k_con_persmed` |
| -222 | 4 | `k_pman_murder95`, `k_pman_murder9b`, `k_pman_murder9c` |
| -196 | 4 | `k_act_walkways`, `k_act_walkways2`, `k_pman_ww` |
| -94 | 4 | `k_ptat_dockpaid`, `k_ptat_nosalehk`, `k_ptat_salehk` |
| -64 | 4 | `k_pdan_kath03`, `k_pkas_freyyspwn`, `k_pman_planet22` |
| -44 | 4 | `k_pman_comp02`, `k_pman_comp31`, `k_pman_comp32` |
| +8 | 4 | `k_amb_prey_ude`, `k_pend_door08`, `k_pman_26a_aread` |
| -300 | 3 | `k_ptat_pazzwin1`, `k_ptat_pazzwin2`, `k_ptat_pazzwin3` |
| -294 | 3 | `k_pman_26d_triga`, `k_pman_26d_trigb`, `k_pman_26d_trigc` |
| -244 | 3 | `k_pman_planet57`, `k_pman_planet58`, `k_pman_planet59` |
| -52 | 3 | `k_act_foeinsight`, `k_hen_spawn01`, `k_pend_door02` |
| -42 | 3 | `k_con_chrlow`, `k_con_intlow`, `k_con_lighthigh` |
| +2 | 3 | `k_pdan_jedi21`, `k_pend_door01`, `k_pman_door30` |
| -6634 | 2 | `k_act_caloatt`, `k_def_rependd` |
| -350 | 2 | `k_act_com31`, `k_sup_comshots` |
| -345 | 2 | `k_actq_com33`, `k_actq_com33a` |
| -304 | 2 | `k_pend_explode`, `k_pman_28d_trigb` |
| -298 | 2 | `k_pman_28d_triga`, `k_pman_survive02` |
| -160 | 2 | `k_sup_star_map`, `k_swg_jolee18` |
| -152 | 2 | `k_hcan_state22`, `k_ren_nextplanet` |
| -146 | 2 | `k_ren_leavlev`, `k_sup_guiopen` |
| -128 | 2 | `k_com_compart`, `k_ptat_nicowait` |
| -110 | 2 | `k_con_light`, `k_hjuh_p21` |
| -98 | 2 | `k_pman_racetwox`, `k_swg_hk4712` |
| -84 | 2 | `k_act_nospeak`, `k_pman_raceonex` |
| -76 | 2 | `k_pman_area_28a`, `k_pman_no_threat` |
| -58 | 2 | `k_pdan_stand02`, `k_ptat_gaffiwrit` |
| -24 | 2 | `k_hjuh_p07`, `k_pman_tatracern` |
| -4 | 2 | `k_pman_27_init01`, `k_swg_hk47upg01` |
| +16 | 2 | `k_ptat_krayttran`, `k_trg_senni` |
| +28 | 2 | `k_pman_airlock02`, `k_sup_bands` |
| +30 | 2 | `k_con_comver33a`, `k_con_comver34` |
| +32 | 2 | `k_ptar_sitharm`, `k_zon_catalog` |
| -8708 | 1 | `k_ai_master` |
| -7054 | 1 | `k_act_bandatt` |
| -6664 | 1 | `k_ptat_gammenemy` |
| -6630 | 1 | `k_act_atkonend` |
| -6606 | 1 | `k_act_xoratt` |
| -1913 | 1 | `k_sp1_generic` |
| -926 | 1 | `k_plc_tresdrdhig` |
| -852 | 1 | `k_bant_trig` |
| -830 | 1 | `k_pend_door06` |
| -668 | 1 | `k_trg_partyinit` |
| -372 | 1 | `k_pkas_freyyuser` |
| -369 | 1 | `k_sup_gohawk` |
| -354 | 1 | `k_hjol_userdef01` |
| -351 | 1 | `k_actq_com34` |
| -324 | 1 | `k_pman_miss01` |
| -320 | 1 | `k_pman_27_area` |
| -318 | 1 | `k_pman_init02` |
| -317 | 1 | `k_trp_generic` |
| -296 | 1 | `k_pman_door32` |
| -290 | 1 | `k_pman_door20` |
| -286 | 1 | `k_act_com44` |
| -284 | 1 | `k_pman_door11` |
| -276 | 1 | `k_pman_raceset_2` |
| -272 | 1 | `k_pman_raceset_4` |
| -252 | 1 | `k_def_calospn` |
| -240 | 1 | `k_pman_26a_area` |
| -218 | 1 | `k_def_bandspn` |
| -214 | 1 | `k_pman_26d_area` |
| -209 | 1 | `k_act_com45` |
| -192 | 1 | `k_pebn_swoop` |
| -180 | 1 | `k_pdan_iriaz_ud` |
| -178 | 1 | `k_ren_visionland` |
| -168 | 1 | `k_amb_predator01` |
| -156 | 1 | `k_hjuh_p07d` |
| -150 | 1 | `k_con_com31` |
| -142 | 1 | `k_hjuh_p07l` |
| -138 | 1 | `k_hjuh_h12` |
| -137 | 1 | `k_act_com35` |
| -130 | 1 | `k_hjuh_p15` |
| -125 | 1 | `k_con_tokens` |
| -124 | 1 | `k_con_dark` |
| -120 | 1 | `k_pman_persuad_f` |
| -118 | 1 | `k_swg_jolee12` |
| -114 | 1 | `k_hen_leadchng` |
| -104 | 1 | `k_swg_jolee14` |
| -90 | 1 | `k_def_buff` |
| -87 | 1 | `k_ptat_exitkrayt` |
| -82 | 1 | `k_hcan_state23` |
| -74 | 1 | `k_ptat_talktanis` |
| -72 | 1 | `k_ptat_gizkauser` |
| -68 | 1 | `k_hjuh_p23` |
| -66 | 1 | `k_swg_bastila02` |
| -34 | 1 | `k_pman_sonic` |
| -23 | 1 | `k_pla_actmap` |
| -22 | 1 | `k_con_flirt` |
| -14 | 1 | `k_unk_rancor` |
| +20 | 1 | `k_sup_healing` |
| +22 | 1 | `k_cheat_01` |
| +43 | 1 | `k_trg_stealth` |
| +60 | 1 | `k_act_startgas` |
| +70 | 1 | `k_trg_transfail1` |
| +80 | 1 | `k_amb_enemyflee` |
| +94 | 1 | `k_com_comrepr` |
| +98 | 1 | `k_act_comp01` |
| +104 | 1 | `k_pman_murdera01` |
| +106 | 1 | `k_act_electric` |
| +107 | 1 | `k_con_comhack` |
| +116 | 1 | `k_con_comver41` |
| +136 | 1 | `k_act_com41` |
| +141 | 1 | `k_trg_messengers` |
| +268 | 1 | `k_trg_partyheart` |
| +1205 | 1 | `k_qa` |
| +2093 | 1 | `k_ptat17aa_enter` |
