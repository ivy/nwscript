# Conformance

1754 scripts with both source and bytecode in `conformance/k1.toml`.

| Compiler | Identical | Same length, different | Different length | Failed |
| --- | --- | --- | --- | --- |
| nwnnsscomp-v100 | 1598 | 1 | 132 | 23 |
| nwnnsscomp | 1204 | 66 | 461 | 23 |
| pykotor | 262 | 11 | 1206 | 275 |

## nwnnsscomp-v100

Edward T. Smith's nwnnsscomp, native build (modshop's mise run nwnnsscomp:build), -v1.00

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
- **not-a-script** (5): an include file with no entry point; there is nothing to compile
  - `k_inc_generic`: error: EntryPointError: This file has no entry point and cannot be compiled (Most likely an include file).
  - `k_inc_gensupport`: error: EntryPointError: This file has no entry point and cannot be compiled (Most likely an include file).
  - `k_inc_switch`: error: EntryPointError: This file has no entry point and cannot be compiled (Most likely an include file).
  - `k_inc_unk`: error: EntryPointError: This file has no entry point and cannot be compiled (Most likely an include file).
  - `k_inc_walkways`: error: EntryPointError: This file has no entry point and cannot be compiled (Most likely an include file).
- **no known cause** (252)
  - `k_act_mispaz`: error: CompileError: Type mismatch for parameter 'nMaxWager' in call to 'PlayPazaak'
  - `k_pdan_mand51`: error: CompileError: Syntax error at line 67, position 1254, token='nOp'
  - `k_pdan_rapid05`: error: CompileError: Syntax error at line 53, position 1254, token='nOp'
  - `k_pman_26a_area`: error: CompileError: Syntax error at line 74, position 1254, token='nOp'
  - `k_pman_26a_aread`: error: CompileError: Syntax error at line 128, position 1254, token='nOp'
  - `k_pman_26d_area`: error: CompileError: Syntax error at line 124, position 1254, token='nOp'
  - `k_pman_26d_triga`: error: CompileError: Syntax error at line 61, position 1254, token='nOp'
  - `k_pman_26d_trigb`: error: CompileError: Syntax error at line 61, position 1254, token='nOp'
  - `k_pman_26d_trigc`: error: CompileError: Syntax error at line 66, position 1254, token='nOp'
  - `k_pman_27_area`: error: CompileError: Syntax error at line 76, position 1254, token='nOp'
  - `k_pman_27_init01`: error: CompileError: Syntax error at line 57, position 1254, token='nOp'
  - `k_pman_28ac_init`: error: CompileError: Syntax error at line 52, position 1254, token='nOp'
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
  - `k_pman_at01`: error: CompileError: Syntax error at line 55, position 1254, token='nOp'
  - `k_pman_at02`: error: CompileError: Syntax error at line 56, position 1254, token='nOp'
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
  - `k_pman_door06`: error: CompileError: Syntax error at line 54, position 1254, token='nOp'
  - `k_pman_door07`: error: CompileError: Syntax error at line 54, position 1254, token='nOp'
  - `k_pman_door10`: error: CompileError: Syntax error at line 51, position 1254, token='nOp'
  - `k_pman_door11`: error: CompileError: Syntax error at line 62, position 1254, token='nOp'
  - `k_pman_door13`: error: CompileError: Syntax error at line 53, position 1254, token='nOp'
  - `k_pman_door14`: error: CompileError: Syntax error at line 53, position 1254, token='nOp'
  - `k_pman_door30`: error: CompileError: Syntax error at line 74, position 1254, token='nOp'
  - `k_pman_door40`: error: CompileError: Syntax error at line 64, position 1254, token='nOp'
  - `k_pman_firixa_d`: error: CompileError: Syntax error at line 56, position 1254, token='nOp'
  - `k_pman_hukname_n`: error: CompileError: Syntax error at line 63, position 1254, token='nOp'
  - `k_pman_hukname_s`: error: CompileError: Syntax error at line 55, position 1254, token='nOp'
  - `k_pman_huktmad_1`: error: CompileError: Syntax error at line 70, position 1254, token='nOp'
  - `k_pman_huktmad_y`: error: CompileError: Syntax error at line 62, position 1254, token='nOp'
  - `k_pman_init01`: error: CompileError: Syntax error at line 55, position 1254, token='nOp'
  - `k_pman_init02`: error: CompileError: Syntax error at line 53, position 1254, token='nOp'
  - `k_pman_init03`: error: CompileError: Syntax error at line 54, position 1254, token='nOp'
  - `k_pman_init10`: error: CompileError: Syntax error at line 70, position 1254, token='nOp'
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
  - `k_pman_rapid01`: error: CompileError: Syntax error at line 53, position 1254, token='nOp'
  - `k_pman_rapid02`: error: CompileError: Syntax error at line 53, position 1254, token='nOp'
  - `k_pman_rapid03`: error: CompileError: Syntax error at line 53, position 1254, token='nOp'
  - `k_pman_rapid04`: error: CompileError: Syntax error at line 53, position 1254, token='nOp'
  - `k_pman_rapid05`: error: CompileError: Syntax error at line 53, position 1254, token='nOp'
  - `k_pman_selapr5_d`: error: CompileError: Syntax error at line 64, position 1254, token='nOp'
  - `k_pman_shasa01`: error: CompileError: Syntax error at line 57, position 1254, token='nOp'
  - `k_pman_sith01`: error: CompileError: Syntax error at line 53, position 1254, token='nOp'
  - `k_pman_sith02`: error: CompileError: Syntax error at line 59, position 1254, token='nOp'
  - `k_pman_sithmas01`: error: CompileError: Syntax error at line 57, position 1254, token='nOp'
  - `k_pman_survive02`: error: CompileError: Syntax error at line 52, position 1254, token='nOp'
  - `k_pman_trial02`: error: CompileError: Syntax error at line 150, position 1254, token='nOp'
  - `k_pman_ww`: error: CompileError: Syntax error at line 46, position 1254, token='nOp'
  - `k_pman_ye_threat`: error: CompileError: Syntax error at line 61, position 1254, token='nOp'
  - `k_ptat17aa_enter`: error: CompileError: Syntax error at line 1840, position 51873, token='nOp'
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
  - `k_ptat_gammenemy`: error: CompileError: Syntax error at line 1843, position 51873, token='nOp'
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
  - `k_ptat_krayttran`: error: CompileError: Syntax error at line 1848, position 51873, token='nOp'
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
