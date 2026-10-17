// The declarations BioWare's own KotOR 1 scripts were compiled against,
// where they differ from the nwscript.nss that shipped. Each prototype
// replaces the shipped one of the same name and keeps its routine number.
//
// ActionStartConversation: every call in BioWare's bytecode passes eleven
// arguments; the shipped declaration adds a twelfth, bUseLeader.
void ActionStartConversation(object oObjectToConverse, string sDialogResRef = "", int bPrivateConversation = FALSE, int nConversationType = CONVERSATION_TYPE_CINEMATIC, int bIgnoreStartRange = FALSE, string sNameObjectToIgnore1 = "", string sNameObjectToIgnore2 = "", string sNameObjectToIgnore3 = "", string sNameObjectToIgnore4 = "", string sNameObjectToIgnore5 = "", string sNameObjectToIgnore6 = "");
