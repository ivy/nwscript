// Inferred from BioWare's bytecode, not from any shipped declaration.
//
// nw_g0_conversat, a Neverwinter Nights script left in KotOR 1's BIF,
// calls StartConversation(), which KotOR's nwscript.nss does not declare.
// Its bytecode is `ACTION 255, 0` followed by `MOVSP -4`: routine 255 with
// no arguments and a four-byte result, discarded. In the shipped table 255
// is BeginConversation, whose two parameters have defaults that a compiler
// would push. The result's type is read as int, as BeginConversation's is.

// nwscript: routine 255
int StartConversation();
