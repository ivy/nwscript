# NWScript grammar, as KotOR 1's scripts use it

NWScript has no standard. This document is the grammar that BioWare's vanilla KotOR 1 sources actually use: the 1,773 script sources in `scripts.bif` and the `nwscript.nss` that declares the engine's constants and functions, 1,774 files in all. The hand-written lexer and recursive-descent parser in `nwscript-syntax` are tested against it production by production, and the compiler cites its [Unconfirmed](#unconfirmed) list wherever it has no reference bytes.

## Evidence

Every construct carries a citation from the census: the census key in code type, then in parentheses its number of uses, the number of files using it, and one or more of those files; or a zero for a construct no file uses. For example, a key cited as `(282 in 25: k_ai_master.nss)` is used 282 times in 25 files, `k_ai_master.nss` among them. The census is a lexer and a recogniser for this grammar that count each production they take; it lives in `xtask/src/grammar_census/` and its output, numbers and file names only, is committed as [`grammar-census.txt`](grammar-census.txt).

```bash
cargo xtask grammar-census > docs/grammar-census.txt                         # regenerate
cargo xtask grammar-census --check docs/grammar-census.txt --doc docs/grammar.md  # verify this document
cargo xtask grammar-census --where stmt.switch.fallthrough                   # FILE:LINE of each use
```

`--doc` fails if any census key is not cited here, if a cited count differs from the census, or if a cited file does not use the construct. The corpus is read from `$NWSCRIPT_CORPUS` (default `~/.cache/nwscript/corpus/k1`, written by `cargo xtask corpus extract`); counts cover `nss/*.nss`, which includes `nwscript.nss`, and the census's fourth column says how many of the uses are in `nwscript.nss`. Include files are counted once each, as files, not once per includer.

The recogniser accepts every file except one line: `nwscript.nss` line 5145, BioWare's typo `void SetAvailableNPCId(INT nNPC, OBJECT_ID oidNPC);`, where `INT` and `OBJECT_ID` are not types. The front end reports it as a diagnostic with a span and loads the rest of the file; `oracle/nwscript.nss` in the corpus has it corrected to `int` and `object`.

A count says the source uses a construct. It does not by itself say how BioWare's compiler encoded it; the compiler's conformance tests compare against the NCS. Where this document says the bytes were checked, they were.

## Lexical grammar

### Characters and line ends

Source files are bytes, read as Windows-1252. Every line end in every file is CR LF `lex.line_end.crlf` (53,564 in 1,774: `e3_scripts.nss`); no file has a bare LF `lex.line_end.lf` (0) or a bare CR `lex.line_end.cr` (0). Two files end without a final line end `lex.no_final_line_end` (2 in 2: `k_inc_kas.nss`, `nwscript.nss`), so the last token may be followed directly by end of file. Tabs appear as whitespace `lex.ws.tab` (267 in 10: `k_con_classsame.nss`); vertical tab and form feed do not `lex.ws.vt_ff` (0), nor does a Ctrl-Z end-of-file byte `lex.ctrl_z` (0). Bytes above 0x7F appear only in comments `lex.non_ascii.comment` (7 in 7: `k_pman_casamad_s.nss`), never in string literals `lex.non_ascii.string` (0) or code `lex.non_ascii.code` (0). A `#` appears only as the first non-blank character of a line `lex.hash_mid_line` (0).

```ebnf
char        = ? any byte ? ;
line_end    = CR LF ;                      (* the parser also accepts LF and CR alone *)
whitespace  = " " | TAB | line_end ;
```

### Comments

```ebnf
line_comment  = "//" { char - ( CR | LF ) } ;
block_comment = "/*" { char } "*/" ;       (* the first "*/" ends it *)
```

Line comments `lex.comment.line` (10,346 in 1,035: `e3_scripts.nss`) and block comments `lex.comment.block` (1,262 in 870: `k_act_atkonend.nss`) are both common, and every block comment in the corpus spans lines `lex.comment.block.multiline` (1,262 in 870: `k_act_atkonend.nss`). A `//` inside a block comment is plain text `lex.comment.block.contains_line_comment` (75 in 25: `k_ai_master.nss`), and so is a `/*` inside a line comment `lex.comment.line.contains_block_open` (6 in 3: `k_inc_man.nss`); the include files among them compile into scripts whose NCS ship, so the lexer must not open a block comment there. No line comment contains `*/` `lex.comment.line.contains_block_close` (0) or ends in a backslash `lex.comment.line.backslash_end` (0), and no line comment runs to end of file `lex.comment.line.at_eof` (0).

Nesting and termination at end of file are unconfirmed: no block comment contains `/*` `lex.comment.block.contains_block_open` (0), and none is unterminated `lex.comment.block.unterminated` (0). The grammar above makes comments non-nesting, as in C, and an unterminated block comment an error.

### Identifiers and keywords

```ebnf
identifier = ( letter | "_" ) { letter | digit | "_" } ;
letter     = "A" .. "Z" | "a" .. "z" ;
digit      = "0" .. "9" ;
```

Identifiers `lex.ident` (39,931 in 1,774: `e3_scripts.nss`) are case-sensitive. The longest is 50 characters, in `nwscript.nss` (the census's `lex.ident.longest`). Keywords are matched exactly: `nwscript.nss` uses `Vector` and `Location` as engine function names and has the typo `INT` `lex.ident.keyword_other_case` (3 in 1: `nwscript.nss`), so a keyword in another case is an identifier.

The keywords and their use:

- Statements: `if` `lex.keyword.if` (3,135 in 651: `e3_scripts.nss`), `else` `lex.keyword.else` (1,120 in 228: `e3_scripts.nss`), `while` `lex.keyword.while` (131 in 39: `e3_scripts.nss`), `do` `lex.keyword.do` (1 in 1: `k_inc_treasure.nss`), `for` `lex.keyword.for` (37 in 19: `k_con_classdiff.nss`), `switch` `lex.keyword.switch` (33 in 25: `k_ai_master.nss`), `case` `lex.keyword.case` (282 in 25: `k_ai_master.nss`), `default` `lex.keyword.default` (6 in 3: `k_inc_lev.nss`), `break` `lex.keyword.break` (235 in 24: `k_ai_master.nss`), `return` `lex.keyword.return` (2,445 in 1,011: `e3_scripts.nss`); `continue` is unused `lex.keyword.continue` (0).
- Types: `void` `lex.keyword.void` (1,751 in 790: `e3_scripts.nss`), `int` `lex.keyword.int` (6,005 in 1,152: `e3_scripts.nss`), `float` `lex.keyword.float` (256 in 17: `e3_scripts.nss`), `string` `lex.keyword.string` (371 in 34: `e3_scripts.nss`), `object` `lex.keyword.object` (1,346 in 153: `e3_scripts.nss`), `vector` `lex.keyword.vector` (30 in 3: `k_inc_dan.nss`), `action` `lex.keyword.action` (3 in 1: `nwscript.nss`), `struct` `lex.keyword.struct` (2 in 1: `k_inc_generic.nss`); `const` is unused `lex.keyword.const` (0).
- Engine structure types, which `nwscript.nss` names (see [nwscript.nss](#nwscriptnss)): `effect` `lex.keyword.effect` (287 in 25: `k_act_com42.nss`), `event` `lex.keyword.event` (5 in 1: `nwscript.nss`), `location` `lex.keyword.location` (70 in 15: `k_amb_predator01.nss`), `talent` `lex.keyword.talent` (74 in 3: `k_inc_generic.nss`). `itemproperty` is a KotOR 2 engine structure and is not a type in KotOR 1 `lex.keyword.itemproperty` (0).
- Object constants: `OBJECT_SELF` `lex.keyword.OBJECT_SELF` (1,290 in 210: `e3_scripts.nss`) and `OBJECT_INVALID` `lex.keyword.OBJECT_INVALID` (52 in 11: `fightback.nss`). `nwscript.nss` does not declare them; they are built into the language. `TRUE` and `FALSE` are not keywords but ordinary constants that `nwscript.nss` declares.

### Integer literals

```ebnf
int_literal = decimal | hex ;
decimal     = digit { digit } ;
hex         = "0x" hex_digit { hex_digit } ;
hex_digit   = digit | "a" .. "f" | "A" .. "F" ;
```

Decimal literals `lex.int.decimal` (6,411 in 997: `e3_scripts.nss`) and hexadecimal literals `lex.int.hex` (75 in 3: `k_inc_generic.nss`) both appear. Every hex literal uses lowercase `0x` `lex.int.hex.upper_x` (0); those with letter digits use lowercase letters `lex.int.hex.lower_digits` (36 in 2: `k_inc_generic.nss`) and none uppercase `lex.int.hex.upper_digits` (0). No decimal literal has a leading zero `lex.int.leading_zero` (0), so whether `010` is octal is unconfirmed; the grammar reads it as decimal ten. The largest decimal literal is 65535 (`nwscript.nss`) and the largest hex literal 0xffff (`k_inc_generic.nss`), the census's `lex.int.decimal.largest` and `lex.int.hex.largest`. There is no sign in a literal: `-1` is unary minus applied to `1` (see [Expressions](#expressions)). No number is followed directly by a letter or underscore `lex.number.followed_by_ident_char` (0).

### Float literals

```ebnf
float_literal = digit { digit } "." digit { digit } [ "f" ] ;
```

Two forms appear: `1.0` `lex.float.digits_dot_digits` (874 in 124: `e3_scripts.nss`) and `1.0f` `lex.float.digits_dot_digits_f` (64 in 23: `k_def_bandspn.nss`). No other form is used: not `.5` `lex.float.dot_digits` (0), `1.` `lex.float.digits_dot` (0), `1f` `lex.float.digits_f` (0), an uppercase suffix `1.0F` `lex.float.digits_dot_digits_upper_f` (0), or an exponent, `1.0e3` `lex.float.digits_dot_digits_exp` (0) or `1e3` `lex.float.digits_exp` (0).

### String literals

```ebnf
string_literal = '"' { char - ( '"' | "\" | CR | LF ) } '"' ;
```

String literals `lex.string` (4,391 in 1,275: `e3_scripts.nss`), including the empty string `lex.string.empty` (167 in 47: `k_act_mispaz.nss`), never contain a backslash: there is no `\n` `lex.string.escape.n` (0), `\"` `lex.string.escape.quote` (0) or `\\` `lex.string.escape.backslash` (0), so escapes are unconfirmed. None is unterminated at a line end `lex.string.unterminated` (0), none contains `//` or `/*` `lex.string.contains_comment_marker` (0), and two literals are never adjacent `expr.string.adjacent` (0) (no C-style concatenation). The longest literal is 80 bytes (`k_sup_galaxymap.nss`, the census's `lex.string.longest`).

### Directives

```ebnf
directive = "#" ( include | define ) line_end ;
include   = "include" { " " | TAB } '"' resref '"' ;
define    = "define" { " " | TAB } identifier { " " | TAB } ( decimal | identifier ) ;
resref    = { letter | digit | "_" } ;
```

A directive is a whole line beginning with `#`, with no space after it `pp.space_after_hash` (0). Only two directives exist `pp.other` (0).

`#include` `pp.include` (1,196 in 964: `k_act_atkonend.nss`) names another source file by resource name, in double quotes `pp.include.not_quoted` (0), without the `.nss` extension `pp.include.with_extension` (0), with nothing after the closing quote `pp.include.trailing_text` (0), not even a comment `pp.include.trailing_comment` (0). Nineteen distinct files are included `pp.include.target` (1,196 in 19: `k_inc_debug.nss`), all `k_inc_*`, and every one exists in the corpus `pp.include.target_missing` (0). One include spells its target `K_inc_utility` `pp.include.not_lowercase` (1 in 1: `k_swg_hk47fix.nss`): names resolve case-insensitively, as the game's resource lookup does. Includes come before every declaration in their file `pp.include.after_declaration` (0) and never inside a function `pp.inside_function` (0). Include files include others `pp.include.in_include_file` (11 in 11: `k_inc_cheat.nss`), without cycles `pp.include.cycle` (0), and no script reaches the same file twice along its include chains `pp.include.reached_twice` (0), so whether a second inclusion is skipped or repeated is unconfirmed.

`#define` `pp.define` (5 in 1: `nwscript.nss`) appears only in `nwscript.nss`, five times, to tell the compiler the engine structure types: `ENGINE_NUM_STRUCTURES` with an integer `pp.define.value_int` (1 in 1: `nwscript.nss`), and `ENGINE_STRUCTURE_0` to `_3` with a type name `pp.define.value_word` (4 in 1: `nwscript.nss`). It is configuration, not a macro facility: nothing else uses the names, and no `#define` lacks a value `pp.define.no_value` (0).

### Punctuators

```ebnf
punctuator = "{" | "}" | "(" | ")" | "[" | "]" | ";" | "," | "." | ":" | "?"
           | "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "<<=" | ">>=" | ">>>="
           | "||" | "&&" | "|" | "^" | "&" | "==" | "!=" | "<" | ">" | "<=" | ">="
           | "<<" | ">>" | ">>>" | "+" | "-" | "*" | "/" | "%" | "!" | "~" | "++" | "--" ;
```

The lexer takes the longest match. Which of these the corpus uses is in [Expressions](#expressions); `?`, `^`, `>>`, `>>>` and most compound assignments are unconfirmed.

## Syntactic grammar

`(* U *)` marks an alternative no vanilla script uses; each is in the [Unconfirmed](#unconfirmed) list. Terminals in quotes; `identifier`, `int_literal`, `float_literal` and `string_literal` are the lexical tokens above.

### Top level

```ebnf
script         = { directive } { declaration } ;
declaration    = function_decl | global_decl | struct_decl
               | ";" ;                                           (* U *)
function_decl  = type identifier "(" [ param_list ] ")" ( ";" | block ) ;
param_list     = param { "," param }
               | "void" ;                                        (* U *)
param          = type identifier [ "=" default_value ] ;
global_decl    = [ "const" ] type declarator { "," declarator } ";" ;   (* U: const, a second declarator *)
declarator     = identifier [ "=" expression ] ;
struct_decl    = "struct" identifier "{" { type identifier ";" } "}" ";" ;
```

A script is its directives, then declarations. Function definitions `top.function.definition` (2,723 in 1,772: `e3_scripts.nss`) and prototypes `top.function.prototype` (1,206 in 31: `e3_scripts.nss`) are both used; a definition follows its own prototype in the same file `top.function.definition_after_prototype` (422 in 30: `e3_scripts.nss`). Every parameter list is either empty `()` `param.list.empty` (2,558 in 1,769: `e3_scripts.nss`) or names its parameters `param` (2,244 in 31: `e3_scripts.nss`); `(void)` `param.list.void` (0) and unnamed parameters `param.unnamed` (0) are unused.

Global variables `top.global` (1,785 in 14: `k_inc_dan.nss`) have one declarator each `top.global.declarator` (1,785 in 14: `k_inc_dan.nss`), `top.global.multi_declarator` (0). Most are initialised; some are not `top.global.uninitialized` (8 in 2: `k_inc_force.nss`). A global may follow function definitions `top.global.after_function_definition` (17 in 1: `e3_scripts.nss`). There is no `const` `decl.const` (0) and no empty declaration `;` at top level `top.empty_declaration` (0).

One struct is declared `top.struct` (1 in 1: `k_inc_generic.nss`), with one member per declaration `struct.member` (10 in 1: `k_inc_generic.nss`), `struct.member.multi_declarator` (0), of types `int` `struct.member.type.int` (9 in 1: `k_inc_generic.nss`) and `object` `struct.member.type.object` (1 in 1: `k_inc_generic.nss`); a global of that struct type `top.global.type.struct` (1 in 1: `k_inc_generic.nss`) holds it. A struct is never declared inside a function `stmt.struct_definition` (0).

### Types

```ebnf
type        = "void" | "int" | "float" | "string" | "object" | "vector" | "action"
            | engine_type
            | "struct" identifier ;
engine_type = "effect" | "event" | "location" | "talent" ;     (* the names nwscript.nss defines *)
```

Return types: `void` `function.return.void` (1,750 in 790: `e3_scripts.nss`), `int` `function.return.int` (1,799 in 1,004: `k_act_foeinsight.nss`), `float` `function.return.float` (56 in 3: `k_inc_force.nss`), `string` `function.return.string` (49 in 7: `e3_scripts.nss`), `object` `function.return.object` (135 in 10: `e3_scripts.nss`), `vector` `function.return.vector` (13 in 2: `k_inc_dan.nss`), `effect` `function.return.effect` (94 in 3: `k_inc_end.nss`), `event` `function.return.event` (4 in 1: `nwscript.nss`), `location` `function.return.location` (6 in 1: `nwscript.nss`), `talent` `function.return.talent` (23 in 3: `k_inc_generic.nss`); never a struct `function.return.struct` (0).

Parameter types: `int` `param.type.int` (1,035 in 24: `e3_scripts.nss`), `float` `param.type.float` (149 in 9: `e3_scripts.nss`), `string` `param.type.string` (220 in 18: `e3_scripts.nss`), `object` `param.type.object` (716 in 21: `e3_scripts.nss`), `vector` `param.type.vector` (16 in 1: `nwscript.nss`), `effect` `param.type.effect` (44 in 3: `k_inc_force.nss`), `event` `param.type.event` (1 in 1: `nwscript.nss`), `location` `param.type.location` (47 in 7: `k_amb_predator01.nss`), `talent` `param.type.talent` (13 in 2: `k_inc_generic.nss`), and `action` `param.type.action` (3 in 1: `nwscript.nss`), which only the engine functions `AssignCommand`, `DelayCommand` and `ActionDoCommand` take; never a struct `param.type.struct` (0).

Global types: `int` `top.global.type.int` (1,746 in 14: `k_inc_dan.nss`), `float` `top.global.type.float` (12 in 2: `k_inc_force.nss`), `string` `top.global.type.string` (26 in 5: `k_inc_dan.nss`), and the struct above.

Local variable types: `int` `stmt.decl.type.int` (1,416 in 871: `e3_scripts.nss`), `float` `stmt.decl.type.float` (39 in 13: `e3_scripts.nss`), `string` `stmt.decl.type.string` (76 in 27: `e3_scripts.nss`), `object` `stmt.decl.type.object` (494 in 149: `e3_scripts.nss`), `vector` `stmt.decl.type.vector` (1 in 1: `k_inc_generic.nss`), `effect` `stmt.decl.type.effect` (149 in 24: `k_act_com42.nss`), `location` `stmt.decl.type.location` (17 in 12: `k_amb_predator01.nss`), `talent` `stmt.decl.type.talent` (38 in 2: `k_inc_generic.nss`); never a struct `stmt.decl.type.struct` (0).

### Default values

```ebnf
default_value = [ "-" ] int_literal | float_literal | string_literal
              | identifier                                       (* a constant *)
              | "OBJECT_SELF" | "OBJECT_INVALID"
              | "[" float_literal "," float_literal "," float_literal "]" ;
```

Parameters with a default `param.default` (627 in 20: `e3_scripts.nss`) use these forms: an integer `param.default.int_literal` (81 in 8: `e3_scripts.nss`), a negative integer `param.default.negative_int_literal` (14 in 1: `nwscript.nss`), a float `param.default.float_literal` (67 in 8: `e3_scripts.nss`), a string `param.default.string_literal` (62 in 11: `e3_scripts.nss`), a constant's name such as `TRUE` `param.default.identifier` (169 in 14: `e3_scripts.nss`), `OBJECT_SELF` `param.default.object_self` (195 in 13: `k_inc_drop.nss`), `OBJECT_INVALID` `param.default.object_invalid` (36 in 4: `k_inc_generic.nss`), and the vector literal `[0.0,0.0,0.0]` `param.default.vector_literal` (3 in 1: `nwscript.nss`). No required parameter follows a defaulted one `param.required_after_default` (0).

Defaults appear in prototypes `param.default.in_prototype` (275 in 18: `e3_scripts.nss`) and in definitions `param.default.in_definition` (158 in 19: `e3_scripts.nss`). When a script declares a prototype and then defines the function, it usually repeats the defaults in both `param.default.in_prototype_and_definition` (76 in 17: `e3_scripts.nss`); once they are in the prototype only `param.default.in_prototype_only` (1 in 1: `k_inc_generic.nss`), and never in the definition only `param.default.in_definition_only` (0).

A call may omit trailing defaulted arguments, to engine functions `expr.call.engine.omits_defaults` (1,942 in 295: `e3_scripts.nss`) and to script functions `expr.call.user.omits_defaults` (196 in 89: `k_act_atkonend.nss`). Every default form above is supplied that way somewhere, so its encoding as an omitted argument is in the corpus: to engine functions, an integer `expr.call.engine.default_used.int_literal` (535 in 197: `e3_scripts.nss`), negative integer `expr.call.engine.default_used.negative_int_literal` (204 in 31: `e3_scripts.nss`), float `expr.call.engine.default_used.float_literal` (548 in 82: `e3_scripts.nss`), string `expr.call.engine.default_used.string_literal` (1,111 in 64: `k_act_hk47simul.nss`), constant `expr.call.engine.default_used.identifier` (694 in 110: `e3_scripts.nss`), `OBJECT_SELF` `expr.call.engine.default_used.object_self` (509 in 51: `e3_scripts.nss`), `OBJECT_INVALID` `expr.call.engine.default_used.object_invalid` (21 in 7: `k_act_mispaz.nss`) and vector `expr.call.engine.default_used.vector_literal` (62 in 7: `k_inc_force.nss`); to script functions, an integer `expr.call.user.default_used.int_literal` (26 in 9: `k_inc_force.nss`), float `expr.call.user.default_used.float_literal` (19 in 11: `k_inc_force.nss`), string `expr.call.user.default_used.string_literal` (37 in 21: `k_amb_prey_ude.nss`), constant `expr.call.user.default_used.identifier` (92 in 47: `k_act_electric.nss`), `OBJECT_SELF` `expr.call.user.default_used.object_self` (18 in 12: `k_def_repairsp.nss`) and `OBJECT_INVALID` `expr.call.user.default_used.object_invalid` (40 in 12: `k_act_atkonend.nss`).

### Statements

```ebnf
block          = "{" { statement } "}" ;
statement      = block
               | local_decl
               | "if" "(" expression ")" statement [ "else" statement ]
               | "while" "(" expression ")" statement
               | "do" statement "while" "(" expression ")" ";"
               | "for" "(" for_init ";" [ expression ] ";" for_step ")" statement   (* U: no condition *)
               | "switch" "(" expression ")" "{" { label | statement } "}"
               | "break" ";"
               | "continue" ";"                                  (* U *)
               | "return" [ expression ] ";"
               | expression ";"
               | ";" ;
local_decl     = [ "const" ] type declarator { "," declarator } ";" ;     (* U: const *)
for_init       = [ expression { "," expression } ]
               | type declarator { "," declarator } ;            (* U: empty, comma list, declaration *)
for_step       = [ expression { "," expression } ] ;                     (* U: empty, comma list *)
label          = "case" expression ":" | "default" ":" ;
```

Blocks `stmt.block` (3,585 in 491: `e3_scripts.nss`) may be empty `stmt.block.empty` (131 in 21: `k_ai_master.nss`), as may a function body `function.body.empty` (21 in 18: `k_act_heartoff.nss`). A statement where a body is expected need not be a block, as in C.

Local declarations `stmt.decl` (2,230 in 955: `e3_scripts.nss`) declare one or more variables `stmt.decl.declarator` (2,304 in 955: `e3_scripts.nss`), several at once in some `stmt.decl.multi_declarator` (33 in 21: `k_con_classdiff.nss`), with an optional initialiser per declarator `stmt.decl.init` (1,297 in 300: `e3_scripts.nss`). A declaration may follow other statements in its block `stmt.decl.after_statement` (366 in 53: `e3_scripts.nss`): there is no C89 rule that declarations come first. One sits directly in a `switch` case, without braces `stmt.decl.in_case_unbraced` (1 in 1: `k_pkas_randwalk.nss`).

`if` `stmt.if` (3,135 in 651: `e3_scripts.nss`) takes an optional `else` `stmt.if.else` (1,120 in 228: `e3_scripts.nss`), and `else if` chains are common `stmt.if.else_if` (752 in 57: `k_ai_master.nss`). Unbraced branches appear after `if` `stmt.if.then_unbraced` (272 in 186: `k_hcan_state07.nss`) and after `else` `stmt.if.else_unbraced` (8 in 8: `k_hjuh_p07.nss`). No unbraced `if` with an `else` is itself the unbraced body of another `if` `stmt.if.dangling_else` (0), so the dangling-else rule (an `else` binds to the nearest `if`, as in C) is unconfirmed.

`while` `stmt.while` (130 in 39: `e3_scripts.nss`), always with a braced body `stmt.while.body_unbraced` (0). `do` ... `while` `stmt.do_while` (1 in 1: `k_inc_treasure.nss`), once, braced `stmt.do_while.body_unbraced` (0).

`for` `stmt.for` (37 in 19: `k_con_classdiff.nss`) always has all three parts `stmt.for.parts.init_cond_step` (37 in 19: `k_con_classdiff.nss`), the initialiser always an expression, never a declaration `stmt.for.init_declaration` (0), with no comma lists `stmt.for.comma_list` (0), and a braced body `stmt.for.body_unbraced` (0). `for (;;)` `stmt.for.parts.none` (0) and every other omission are unconfirmed.

`switch` `stmt.switch` (33 in 25: `k_ai_master.nss`): case labels `stmt.switch.case` (282 in 25: `k_ai_master.nss`) are integer literals `stmt.switch.case.int_literal` (236 in 25: `k_ai_master.nss`) or constants declared in `nwscript.nss` `stmt.switch.case.identifier` (46 in 1: `k_inc_force.nss`); no label is a negative literal, a string or another expression. `default` `stmt.switch.default` (6 in 3: `k_inc_lev.nss`) always comes last `stmt.switch.default_not_last` (0). Labels sit at the top level of the switch body, never inside a nested block `stmt.switch.label_nested` (0), and the body starts with a label `stmt.switch.statement_before_first_label` (0); no switch is empty `stmt.switch.empty` (0), and no label ends the body without a statement after it `stmt.switch.trailing_label_without_statement` (0). Switches nest `stmt.switch.nested` (6 in 3: `k_inc_tar.nss`). Two labels share one group of statements once `stmt.switch.stacked_labels` (1 in 1: `k_pkas_randwalk.nss`). Control falls through from one group into the next `stmt.switch.fallthrough` (42 in 1: `k_qa.nss`), all in `k_qa.nss`, and out of the last group without a `break` `stmt.switch.last_group_falls_out` (9 in 8: `k_inc_force.nss`). A group may be a braced block `stmt.switch.group_braced` (47 in 5: `k_inc_force.nss`), with its `break` after the braces, never inside `stmt.switch.jump_inside_braces` (0).

`break` leaves a switch `stmt.break.in_switch` (234 in 24: `k_ai_master.nss`) or, once, a loop `stmt.break.in_loop` (1 in 1: `k_inc_tar.nss`). `continue` is unconfirmed `stmt.continue` (0). `return` with a value `stmt.return.value` (2,149 in 1,006: `e3_scripts.nss`), sometimes parenthesised like a call, `return(x);` `stmt.return.value_parenthesized` (102 in 20: `e3_scripts.nss`), and without `stmt.return.void` (296 in 16: `e3_scripts.nss`).

Expression statements `stmt.expr` (6,681 in 1,395: `e3_scripts.nss`) are calls `stmt.expr.call` (4,025 in 970: `e3_scripts.nss`), assignments `stmt.expr.assign` (2,456 in 710: `e3_scripts.nss`), compound assignments `stmt.expr.compound_assign` (27 in 8: `k_inc_force.nss`), postfix increments and decrements `stmt.expr.postfix_incdec` (167 in 33: `e3_scripts.nss`) and one prefix decrement `stmt.expr.prefix_incdec` (1 in 1: `k_inc_force.nss`). A few compute a value and discard it, such as `n == 1;` where an assignment was meant `stmt.expr.no_effect` (5 in 3: `k_hen_canduser.nss`); the parser accepts them and the lint layer flags them. The empty statement appears as a doubled `;;` `stmt.empty` (3 in 2: `k_pdan_jedi37.nss`).

### Expressions

```ebnf
expression     = assignment ;
assignment     = conditional
               | postfix assign_op assignment ;
assign_op      = "=" | "+=" | "-=" | "/="
               | "*=" | "%=" | "&=" | "|=" | "^=" | "<<=" | ">>=" | ">>>=" ;   (* U: the second line *)
conditional    = logical_or
               | logical_or "?" expression ":" conditional ;     (* U *)
logical_or     = logical_and { "||" logical_and } ;
logical_and    = bit_or { "&&" bit_or } ;
bit_or         = bit_xor { "|" bit_xor } ;
bit_xor        = bit_and { "^" bit_and } ;                       (* U: "^" *)
bit_and        = equality { "&" equality } ;
equality       = relational { ( "==" | "!=" ) relational } ;
relational     = shift { ( "<" | ">" | "<=" | ">=" ) shift } ;
shift          = additive { ( "<<" | ">>" | ">>>" ) additive } ; (* U: ">>", ">>>" *)
additive       = multiplicative { ( "+" | "-" ) multiplicative } ;
multiplicative = unary { ( "*" | "/" | "%" ) unary } ;
unary          = ( "-" | "!" | "~" | "++" | "--" ) unary
               | "+" unary                                       (* U *)
               | postfix ;
postfix        = operand { "." identifier | "++" | "--" } ;
operand        = identifier "(" [ expression { "," expression } ] ")"     (* a call *)
               | primary ;
primary        = int_literal | float_literal | string_literal
               | "OBJECT_SELF" | "OBJECT_INVALID"
               | identifier
               | "(" expression ")"
               | "[" expression "," expression "," expression "]" ;
```

#### Precedence and associativity

Tightest first. The table is C's, without the comma operator, plus `>>>`.

| Level | Operators | Associativity |
| --- | --- | --- |
| 1 | call `f(...)`, member `.`, postfix `++ --` | left |
| 2 | prefix `- ! ~ ++ --`, unary `+` (U) | right |
| 3 | `* / %` | left |
| 4 | `+ -` | left |
| 5 | `<< >> >>>` | left |
| 6 | `< > <= >=` | left |
| 7 | `== !=` | left |
| 8 | `&` | left |
| 9 | `^` (U) | left |
| 10 | `\|` | left |
| 11 | `&&` | left |
| 12 | `\|\|` | left |
| 13 | `?:` (U) | right |
| 14 | `= += -= /=`, and (U) `*= %= &= \|= ^= <<= >>= >>>=` | right |

Each operator's uses: `||` `expr.binary.logor` (742 in 94: `e3_scripts.nss`), `&&` `expr.binary.logand` (1,175 in 388: `e3_scripts.nss`), `|` `expr.binary.bitor` (18 in 3: `k_inc_dan.nss`), `&` `expr.binary.bitand` (3 in 2: `k_inc_gensupport.nss`), `==` `expr.binary.eq` (3,238 in 811: `e3_scripts.nss`), `!=` `expr.binary.ne` (181 in 58: `e3_scripts.nss`), `<` `expr.binary.lt` (185 in 106: `e3_scripts.nss`), `>` `expr.binary.gt` (325 in 143: `k_ai_master.nss`), `<=` `expr.binary.le` (128 in 31: `k_ai_master.nss`), `>=` `expr.binary.ge` (91 in 55: `k_ai_master.nss`), `<<` `expr.binary.shl` (1 in 1: `k_inc_dan.nss`), `+` `expr.binary.add` (539 in 110: `e3_scripts.nss`), `-` `expr.binary.sub` (57 in 26: `e3_scripts.nss`), `*` `expr.binary.mul` (25 in 9: `k_hen_canduser.nss`), `/` `expr.binary.div` (50 in 11: `k_ai_master.nss`), `%` `expr.binary.mod` (1 in 1: `k_inc_force.nss`); prefix `-` `expr.unary.neg` (85 in 17: `e3_scripts.nss`), `!` `expr.unary.not` (303 in 58: `e3_scripts.nss`), `~` `expr.unary.bitnot` (1 in 1: `e3_scripts.nss`), `--` `expr.unary.predec` (1 in 1: `k_inc_force.nss`); postfix `++` `expr.postfix.inc` (142 in 40: `e3_scripts.nss`) and `--` `expr.postfix.dec` (58 in 5: `e3_scripts.nss`); `=` `expr.assign.assign` (2,515 in 711: `e3_scripts.nss`), `+=` `expr.assign.add_assign` (12 in 6: `k_inc_ebonhawk.nss`), `-=` `expr.assign.sub_assign` (7 in 1: `k_sup_grenade.nss`), `/=` `expr.assign.div_assign` (8 in 3: `k_inc_force.nss`). Parentheses `expr.paren` (1,458 in 579: `e3_scripts.nss`).

`+` also concatenates strings; a string literal is an operand of `+` in many places `expr.binary.add.string_literal_operand` (272 in 40: `e3_scripts.nss`).

Unary minus applies to integer literals `expr.unary.neg.int_literal` (83 in 17: `e3_scripts.nss`), once to a float literal `expr.unary.neg.float_literal` (1 in 1: `k_inc_generic.nss`) and once to a variable `expr.unary.neg.other` (1 in 1: `k_inc_utility.nss`). A parser keeps `-1` as negation of a literal; whether BioWare folded it into a negative constant is the compiler's question.

Which precedence relations the corpus exercises is counted wherever an operator's unparenthesised operand is an operator from another level, written `outer_over_inner`:

- `||` over `==` `prec.logor_over_equality` (795 in 30: `k_ai_master.nss`) and over relational `prec.logor_over_relational` (34 in 10: `e3_scripts.nss`); `&&` over `==` `prec.logand_over_equality` (525 in 89: `e3_scripts.nss`) and over relational `prec.logand_over_relational` (256 in 34: `e3_scripts.nss`).
- Relational over additive `prec.relational_over_additive` (3 in 1: `k_inc_utility.nss`) and over multiplicative `prec.relational_over_multiplicative` (1 in 1: `k_pkas_freyyuser.nss`); equality over multiplicative `prec.equality_over_multiplicative` (1 in 1: `k_inc_force.nss`); additive over multiplicative `prec.additive_over_multiplicative` (1 in 1: `k_inc_force.nss`).
- Prefix `!` as the operand of a comparison, `!n > 2`, once `prec.not_operand_of_comparison` (1 in 1: `k_swg_jolee12.nss`). The bytes were checked: `k_swg_jolee12.ncs` applies `NOTI` to the variable and then compares with `GTII`, so `!` binds tighter than `>` as in C, and the script's test is `(!n) > 2`.

Unconfirmed, because BioWare always parenthesised: `||` against `&&` `prec.logor_over_logand` (0); `&&` against `|` `prec.logand_over_bitor` (0); `|` against `^` `prec.bitor_over_bitxor` (0), `==` `prec.bitor_over_equality` (0) and relational `prec.bitor_over_relational` (0); `^` against `&` `prec.bitxor_over_bitand` (0); `&` against `==` `prec.bitand_over_equality` (0) and relational `prec.bitand_over_relational` (0) (C's trap, where `a & b == c` is `a & (b == c)`); `==` against relational `prec.equality_over_relational` (0); relational against shift `prec.relational_over_shift` (0); shift against additive `prec.shift_over_additive` (0).

Left associativity is exercised by chains of `||` `assoc.left.logor` (266 in 40: `k_act_bandatt.nss`), `&&` `assoc.left.logand` (300 in 100: `e3_scripts.nss`) and `+` `assoc.left.additive` (79 in 14: `e3_scripts.nss`), where grouping does not change the result. No chain mixes `-` into `+`, or uses `-` or `/` twice, where it would `assoc.left.additive.order_matters` (0), `assoc.left.multiplicative` (0), `assoc.left.multiplicative.order_matters` (0); no comparison is chained `assoc.left.equality.order_matters` (0), `assoc.left.relational.order_matters` (0); no shift is chained `assoc.left.shift` (0).

#### Assignment

The target of every assignment is a variable or a struct member `expr.assign.target_not_lvalue` (0); struct members are assigned `expr.assign.to_member` (11 in 1: `k_inc_generic.nss`). An assignment is an expression with a value: once, parenthesised, as an operand, `if ((i = f(s)) >= 0)` `expr.assign.as_operand` (1 in 1: `k_inc_treasure.nss`). Unparenthesised assignment inside an operand `expr.assign.operand_unparenthesized` (0) and chains `a = b = c` `expr.assign.chained` (0) are unused.

#### Calls

Calls `expr.call` (12,343 in 1,730: `e3_scripts.nss`) go to engine functions declared in `nwscript.nss` `expr.call.engine` (9,724 in 1,469: `e3_scripts.nss`) and to script functions `expr.call.user` (2,619 in 500: `e3_scripts.nss`); many take no arguments `expr.call.no_arguments` (1,666 in 622: `e3_scripts.nss`). The callee is always a plain name. A script function may call itself `sem.recursive_call` (7 in 6: `k_inc_dan.nss`).

An `action` argument is the call expression that the engine stores and runs later, not a value computed now. Every `action` argument in the corpus is a call `action_arg` (520 in 127: `e3_scripts.nss`), to an engine function `action_arg.engine_call` (461 in 117: `e3_scripts.nss`) or to a script function `action_arg.user_call` (59 in 25: `e3_scripts.nss`), never another expression `action_arg.not_a_call` (0).

#### Members and vectors

The `.` operator `expr.member` (21 in 1: `k_inc_generic.nss`) is used only on the struct in `k_inc_generic.nss`, by field name `expr.member.named_field` (21 in 1: `k_inc_generic.nss`). A vector's `.x`, `.y` and `.z` are never accessed `expr.member.xyz` (0), a member of a call's result is never taken `expr.member.of_call` (0), and members never chain `expr.member.chained` (0).

The vector literal `[x, y, z]` `expr.vector_literal` (3 in 1: `nwscript.nss`) appears only as a parameter default in `nwscript.nss`, always three elements `expr.vector_literal.elements_3` (3 in 1: `nwscript.nss`), each a float literal `expr.vector_literal.element.float_literal` (9 in 1: `nwscript.nss`). Scripts build vectors with the engine function `Vector(x, y, z)`.

`OBJECT_SELF` `expr.object_self` (1,290 in 210: `e3_scripts.nss`) and `OBJECT_INVALID` `expr.object_invalid` (52 in 11: `fightback.nss`) are expressions of type `object`.

## Semantics a parser must know

**Entry points.** A script with `void main()` `entry.main.returns_void` (767 in 767: `fightback.nss`) is run by the engine; one with `int StartingConditional()` `entry.starting_conditional.returns_int` (983 in 983: `k_act_foeinsight.nss`) is a dialogue condition. The rest have neither `entry.none` (24 in 24: `k_inc_debug.nss`): the include files, `nwscript.nss`, and `e3_scripts.nss`, a library left over from a demo. No file has both `entry.both` (0), and neither entry point takes parameters `entry.has_params` (0).

**Declaration before use.** Every call to a script function comes after that function's prototype or definition in its file `sem.call_before_declaration` (0), so one pass over the tokens suffices to resolve names. Functions are not redefined `top.function.redefinition` (0), and no prototype follows its definition `top.function.prototype_after_definition` (0). One prototype is repeated `top.function.prototype_repeated` (1 in 1: `k_inc_ebonhawk.nss`), and two definitions disagree with their prototypes `top.function.definition_differs_from_prototype` (2 in 1: `k_inc_ebonhawk.nss`) (`void` prototypes, `string` definitions), which BioWare's compiler accepted; `--compat bioware` must too.

**Globals.** A global's initialiser is usually a literal: integer `top.global.init.int_literal` (1,724 in 13: `e3_scripts.nss`), negative integer `top.global.init.negative_int_literal` (12 in 3: `k_inc_end.nss`), float `top.global.init.float_literal` (12 in 2: `k_inc_force.nss`), string `top.global.init.string_literal` (24 in 4: `k_inc_dan.nss`), or another constant's name `top.global.init.identifier` (3 in 2: `k_inc_treasure.nss`). It need not be constant: two include files initialise a global with a call, `string RACE_DEFAULT = GetStringByStrRef(...)` `top.global.init.call` (2 in 2: `k_inc_man.nss`, `k_inc_tat.nss`), so globals are initialised by code that runs before the entry point. No initialiser is any other expression `top.global.init.expression` (0).

**Stale declarations.** The shipped `nwscript.nss` is not exactly the one BioWare compiled against. One call has more or fewer arguments than its engine prototype allows `expr.call.engine.arity_mismatch` (1 in 1: `k_act_bastadd.nss`); script calls always match their script prototypes `expr.call.user.arity_mismatch` (0). Three files call functions declared nowhere in KotOR 1 `expr.call.user.undeclared` (42 in 3: `e3_scripts.nss`, `nw_g0_conversat.nss`, `nw_s0_lghtnbolt.nss`): Neverwinter Nights functions such as `SetLocalFloat` in leftovers from that game.

**Constants.** Case labels name constants such as `FORCE_POWER_CHOKE` from `nwscript.nss`, and defaults name constants too, so the compiler must fold an identifier that names a literal-initialised global to its value where a constant is required. Whether any initialised global may stand there, or only those of `nwscript.nss`, is a semantic choice the corpus does not settle.

## nwscript.nss

`nwscript.nss` is parsed by the same parser as any script; it is data for the compiler, not a script. What it contains:

- A comment header, then five `#define` lines naming the engine structures: `ENGINE_NUM_STRUCTURES 4` and `ENGINE_STRUCTURE_0` to `_3` as `effect`, `event`, `location`, `talent`. These four names are types only because of these lines; KotOR 2's file adds `itemproperty` the same way.
- Constants as plain initialised globals, no `const`: 1,490 of them (the `nwscript.nss` column of `top.global`), of type `int`, `float` and `string`, initialised with literals, negative literals, or the name of an earlier constant, as `PLAYER_CHAR_IS_PC = TRUE`. `TRUE`, `FALSE` and `PI` are among them.
- 771 function prototypes that parse (the `nwscript.nss` column of `top.function.prototype`), plus the typo line, 772 in all. An engine function's number in the bytecode's `ACTION` instruction is its position among the prototypes, from zero, so the typo line keeps its slot (767) and the front end must count it. The bytes were checked by searching each script's NCS for `ACTION` with the number of every engine function at position 600 or above that its source calls: 129 of 131 calls were found, and the other two are in `k_trg_transfail1.nss`, whose shipped NCS predates two lines of its source. No vanilla script calls a function past position 759, so the typo's own slot has no reference bytes.
- Line comments, no block comments, and no function bodies, structs or statements.
- No line end after the last line.

## Unconfirmed

Constructs the language plausibly has, from Neverwinter Nights' documentation, from `nwscript.nss`'s declarations, or from the C syntax NWScript borrows, that no vanilla KotOR 1 script uses. Encoding unconfirmed for each: the compiler has no reference bytes, chooses the encoding nwnnsscomp's `-v1.00` output uses, and says so where it relies on one.

- `const` declarations `decl.const` (0).
- `continue` `stmt.continue` (0).
- The conditional operator `?:` `expr.conditional` (0).
- `^` `expr.binary.bitxor` (0), `>>` `expr.binary.shr` (0), `>>>` `expr.binary.ushr` (0), unary `+` `expr.unary.plus` (0).
- Compound assignments `*=` `expr.assign.mul_assign` (0), `%=` `expr.assign.mod_assign` (0), `&=` `expr.assign.bitand_assign` (0), `|=` `expr.assign.bitor_assign` (0), `^=` `expr.assign.bitxor_assign` (0), `<<=` `expr.assign.shl_assign` (0), `>>=` `expr.assign.shr_assign` (0), `>>>=` `expr.assign.ushr_assign` (0).
- Chained assignment `expr.assign.chained` (0) and unparenthesised assignment as an operand `expr.assign.operand_unparenthesized` (0).
- The precedence relations listed as unconfirmed under [Precedence and associativity](#precedence-and-associativity), and associativity where grouping changes the result.
- `for` with any part omitted, `for (;;)` `stmt.for.parts.none` (0); a declaration as the initialiser `stmt.for.init_declaration` (0); comma-separated initialisers or steps `stmt.for.comma_list` (0)., and unbraced bodies of `while`, `do` and `for` (unbraced bodies are confirmed only after `if` and `else`).
- The dangling `else` `stmt.if.dangling_else` (0).
- `switch` with `default` before another label, a statement before the first label, a label inside a nested block, an empty body; case labels that are negative literals, strings or other constant expressions.
- Structs as parameters `param.type.struct` (0), return values `function.return.struct` (0) and local variables `stmt.decl.type.struct` (0); a struct declared inside a function `stmt.struct_definition` (0); several members in one declaration `struct.member.multi_declarator` (0); a member of a call's result `expr.member.of_call` (0); chained members `expr.member.chained` (0).
- Vector components `.x`, `.y`, `.z` `expr.member.xyz` (0); a vector literal anywhere but a parameter default, with elements other than float literals.
- Several declarators in one global declaration `top.global.multi_declarator` (0); a global initialised with an operator expression `top.global.init.expression` (0); a negative float default.
- `(void)` parameter lists `param.list.void` (0); unnamed parameters `param.unnamed` (0); defaults only on a definition whose prototype has none `param.default.in_definition_only` (0).
- An empty declaration `;` at top level `top.empty_declaration` (0).
- Float literals `.5`, `1.`, `1f`, `1.0F` and exponents; hex literals with `0X` or uppercase digits; decimal literals with a leading zero.
- String escapes `\n`, `\"`, `\\` and any other; adjacent string literals.
- Nested block comments and a block comment unterminated at end of file; a line comment ending in a backslash.
- `#define` outside `nwscript.nss`, a `#define` with no value, `#include` with an extension, angle brackets or trailing text, an include reached twice along include chains, a space after `#`, and any other directive `pp.other` (0).
- The `itemproperty` type, which is KotOR 2's fifth engine structure `lex.keyword.itemproperty` (0).

### Encodings taken from nwnnsscomp

Where the corpus has no reference bytes, `nwscript-codegen` emits what nwnnsscomp `-v1.00` emits for the same source, observed by compiling probe scripts with it as a black box. Each is pinned by a test in `crates/nwscript-codegen/tests/probes.rs`.

- `continue` jumps without removing the locals of the blocks it leaves: in `while` to the jump back, after the body's cleanup; in `do` to the condition; in `for` to the step.
- `for` with no condition tests a constant: `CONSTI 1`, `JZ` past the loop. A declaration as the initialiser is rejected by nwnnsscomp; `nws` accepts it and removes the variable after the loop.
- `?:` is the condition, `JZ` to the second branch, the first branch, `JMP` past the second.
- An assignment used as a value leaves the value on the stack after its `CPDOWNSP`, so chained assignment stores the same value twice.
- Unary `+` emits nothing.
- `++` and `--` on a struct field or vector component read the value as any member is read, the whole variable then a `DESTRUCT`, and change it with `INCISP`/`DECISP` at the member's offset; prefix changes before reading, postfix after.
- A vector literal is three `CONSTF`, `x` first. nwnnsscomp rejects a negated element in one.
- Vector `==` and `!=` are `EQUALTT 12` and `NEQUALTT 12`.
- A struct parameter or return value takes one `RSADD` per field, and a member of a call's result is read with `DESTRUCT`.
- A struct declaration makes a globals routine, `SAVEBP`, `JSR` to the entry point, `RESTOREBP`, even when the script declares no global.
- Functions called from global initialisers come before the entry point in the bytecode.

### Never seen, and errors

These are not constructs but mistakes, and no vanilla script makes them: an assignment or increment whose target is not a variable or member (`expr.assign.target_not_lvalue` (0), `expr.postfix.target_not_lvalue` (0)), `break` outside a loop or switch `stmt.break.outside` (0), `continue` outside a loop `stmt.continue.outside_loop` (0), a required parameter after a defaulted one `param.required_after_default` (0), an include cycle `pp.include.cycle` (0) or a missing include, a `#` in the middle of a line `lex.hash_mid_line` (0), an unterminated string `lex.string.unterminated` (0), a byte above 0x7F outside comments and strings `lex.non_ascii.code` (0), a number running into an identifier `lex.number.followed_by_ident_char` (0), a function defined twice or prototyped after its definition, and a file with both entry points. Their diagnostics have no corpus file to calibrate against.
