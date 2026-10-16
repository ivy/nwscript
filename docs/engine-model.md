# The engine model

The engine routines `nwscript_vm::World` models, and what each does there. A routine not listed returns its type's default, and the call is marked unmodelled in the trace. This file is generated from `MODELS` in `crates/nwscript-vm/src/world.rs`; a test fails when the two differ.

## Computed exactly

Functions of their arguments alone.

| Routine | Here |
| --- | --- |
| `abs` | absolute value; `abs(i32::MIN)` wraps |
| `fabs` | absolute value |
| `IntToString` | decimal digits |
| `IntToFloat` | the nearest float |
| `FloatToInt` | truncated toward zero, saturating |
| `FloatToString` | printf `%*.*f` with the width and decimals given (18 and 9 by default) |
| `StringToInt` | C `atoi`: leading spaces, a sign, digits; 0 when there are none |
| `GetStringLength` | length in bytes |
| `GetSubString` | the bytes from `nStart`, at most `nCount`; empty when out of range |
| `GetStringLeft` | the first `nCount` bytes |
| `GetStringRight` | the last `nCount` bytes |
| `GetStringUpperCase` | ASCII upper case |
| `GetStringLowerCase` | ASCII lower case |
| `FindSubString` | the byte index of the first occurrence, or -1 |

## Modelled with the world's state

They read or change the world: its objects, variables, party, context and generator.

| Routine | Here |
| --- | --- |
| `Random` | a number from 0 to `nMaxInteger - 1` from the world's seeded generator; 0 when `nMaxInteger` is below 1 |
| `d2` | the sum of `nNumDice` rolls (at least one) of a two-sided die |
| `d3` | as `d2`, three sides |
| `d4` | as `d2`, four sides |
| `d6` | as `d2`, six sides |
| `d8` | as `d2`, eight sides |
| `d10` | as `d2`, ten sides |
| `d12` | as `d2`, twelve sides |
| `d20` | as `d2`, twenty sides |
| `d100` | as `d2`, a hundred sides |
| `ShipBuild` | the world's `ship_build` flag, true by default as in the released game |
| `GetGlobalNumber` | the world's global number, 0 when unset |
| `SetGlobalNumber` | sets the global number |
| `GetGlobalBoolean` | the world's global boolean, false when unset |
| `SetGlobalBoolean` | sets the global boolean (any non-zero value is true) |
| `GetGlobalString` | the world's global string, empty when unset |
| `SetGlobalString` | sets the global string |
| `GetLocalNumber` | the object's local number at the index, 0 when unset or the object is invalid |
| `SetLocalNumber` | sets the object's local number |
| `GetLocalBoolean` | the object's local boolean at the index |
| `SetLocalBoolean` | sets the object's local boolean |
| `GetIsObjectValid` | true for an object that exists and has not been destroyed |
| `GetObjectByTag` | the `nNth` object (from 0) with the tag, ignoring case, in creation order |
| `GetWaypointByTag` | the first waypoint with the tag |
| `GetNearestObjectByTag` | the `nNth` nearest (from 1) other object in the target's area with the tag |
| `GetNearestObject` | the `nNth` nearest other object in the target's area of the type mask |
| `GetNearestCreature` | the `nNth` nearest other creature meeting the criteria; racial type, player character, alive, and has or lacks a spell effect are modelled, other criteria are not |
| `GetTag` | the object's tag; empty for an invalid object |
| `GetName` | the object's name |
| `GetObjectType` | the object's `OBJECT_TYPE_*`; `OBJECT_TYPE_INVALID` for an invalid one |
| `GetFirstPC` | the party leader |
| `GetPCSpeaker` | the conversation's PC speaker, from the context |
| `GetEnteringObject` | from the context |
| `GetExitingObject` | from the context |
| `GetLastOpenedBy` | from the context |
| `GetLastUsedBy` | from the context |
| `GetClickingObject` | from the context |
| `GetLastPerceived` | from the context |
| `GetLastAttacker` | the attackee's last attacker |
| `GetUserDefinedEventNumber` | from the context |
| `GetModule` | the module object |
| `GetArea` | the object's area |
| `GetIsPC` | true for the player character |
| `GetPartyMemberByIndex` | the party member at the index, the leader at 0 |
| `GetPartyMemberCount` | the party's size |
| `IsObjectPartyMember` | true for an object in the party |
| `IsNPCPartyMember` | true when the `NPC_*` number is in the party |
| `GetSoloMode` | the world's solo-mode flag |
| `GetCurrentHitPoints` | the object's hit points |
| `GetMaxHitPoints` | the object's maximum hit points |
| `GetIsDead` | true when the creature is dead |
| `GetRacialType` | the creature's `RACIAL_TYPE_*` |
| `GetGender` | the creature's `GENDER_*` |
| `GetHitDice` | the creature's total level |
| `GetLevelByClass` | the creature's levels in the class |
| `GetStandardFaction` | the object's `STANDARD_FACTION_*` |
| `GetIsEnemy` | true when the two objects' factions are listed as enemies in the world |
| `GetIsFriend` | true when the two objects share a faction |
| `GetIsNeutral` | true when neither enemies nor friends |
| `GetObjectSeen` | true for a pair the world lists as seen, or for two objects in one area within the world's perception range (20 m by default) |
| `GetIsInCombat` | the creature's in-combat flag |
| `GetNPCAIStyle` | the creature's AI style |
| `SetNPCAIStyle` | sets the creature's AI style |
| `GetPlotFlag` | the object's plot flag |
| `SetPlotFlag` | sets the object's plot flag |
| `GetHasSpell` | 1 when the creature knows the spell or Force power, else 0 |
| `GetHasFeat` | true when the creature has the feat |
| `GetHasSpellEffect` | true when the object is under the spell's effect |
| `TalentSpell` | a talent for the spell |
| `GetIsTalentValid` | true for a talent that is not null |
| `GetIdFromTalent` | the talent's spell, feat or skill number |
| `GetTypeFromTalent` | the talent's `TALENT_TYPE_*` |
| `GetCreatureHasTalent` | true when the creature has the talent's spell, feat or skill |
| `GetCreatureTalentRandom` | one of the creature's talents in the category, chosen by the world's generator; null when it has none |
| `GetCreatureTalentBest` | the creature's first talent in the category, as listed; null when it has none |
| `GetBaseItemType` | the item's `BASE_ITEM_*` |
| `GetItemInSlot` | the item the creature has in the slot |
| `GetItemPossessedBy` | the first item in the creature's inventory or slots with the tag |
| `GetFirstItemInInventory` | the first item in the object's inventory, starting its iteration |
| `GetNextItemInInventory` | the next item of that iteration |
| `CreateItemOnObject` | a new item, tagged with the template's name, in the target's inventory |
| `DestroyObject` | destroys the object at once, whatever the delay |
| `GetPosition` | the object's position |
| `GetFacing` | the object's facing in degrees |
| `GetLocation` | a location holding the object's area, position and facing |
| `GetDistanceBetween` | the distance between two objects in one area; 0 otherwise |

## Recorded

Their effects lie outside anything a script can read back through the model, so the call in the trace is the whole of their behaviour here.

| Routine | Here |
| --- | --- |
| `AssignCommand` | the action runs after the script, as the given object |
| `DelayCommand` | the action runs after the script, as the caller; the delay is not modelled |
| `ActionDoCommand` | the action runs after the script, as the caller |
| `ClearAllActions` | recorded |
| `ActionPlayAnimation` | recorded |
| `ActionWait` | recorded |
| `ActionMoveToObject` | recorded |
| `ActionForceMoveToObject` | recorded |
| `ActionMoveToLocation` | recorded |
| `ActionJumpToObject` | recorded |
| `ActionJumpToLocation` | recorded |
| `ActionAttack` | recorded |
| `ActionUseTalentOnObject` | recorded |
| `ActionStartConversation` | recorded |
| `ActionPauseConversation` | recorded |
| `ActionResumeConversation` | recorded |
| `ActionSpeakString` | recorded |
| `ActionEquipItem` | recorded |
| `CancelCombat` | recorded |
| `SetCommandable` | recorded |
| `SetListenPattern` | recorded |
| `SetFacing` | recorded |
| `PlaySound` | recorded |
| `SpeakString` | recorded |
| `SendMessageToPC` | recorded |
| `AurPostString` | recorded |
| `PrintString` | recorded |
| `PrintInteger` | recorded |
| `PrintFloat` | recorded |
| `PrintObject` | recorded |
| `ApplyEffectToObject` | recorded; the effect is not applied |
| `GiveXPToCreature` | recorded; the experience is not added |
