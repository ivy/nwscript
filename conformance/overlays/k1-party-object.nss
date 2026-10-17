// Inferred from BioWare's bytecode, not from any shipped declaration.
//
// Ten KotOR 1 scripts call AddPartyMember or RemovePartyMember with one
// object, GetObjectByTag's result, where the shipped nwscript.nss declares
// `int AddPartyMember(int nNPC, object oCreature)` at routine 574 and
// `int RemovePartyMember(int nNPC)` at 575. Their bytecode calls routines
// 569 and 570 with one argument (k_act_bastadd: `ACTION 569, 1`; the nine
// k_act_*rmv: `ACTION 570, 1`), and pops one four-byte result after each
// call. So when they were compiled the two routines took an object and sat
// five places earlier. The result's type is read as int, as the shipped
// declarations have it; the bytes do not distinguish int from float or
// object. Other scripts call the shipped forms, so this applies only to
// the ten (conformance/known.toml, cause stale-prototype).

// nwscript: routine 569
int AddPartyMember(object oCreature);

// nwscript: routine 570
int RemovePartyMember(object oCreature);
