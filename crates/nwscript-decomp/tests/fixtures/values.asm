; Hand-assembled: strings with an escape, objects, a vector local and its
; component, a float function, unary operators, a negative constant, and
; AssignCommand's deferred action.
    JSR          sub_15
    RETN
sub_15:
    RSADDS
    CONSTS       "a\"b"
    CPDOWNSP     -8, 4
    MOVSP        -4
    RSADDO
    CONSTO       0
    CPDOWNSP     -8, 4
    MOVSP        -4
    RSADDF
    RSADDF
    RSADDF
    CONSTF       3.0
    CONSTF       2.0
    CONSTF       1.0
    ACTION       8, 3
    CPDOWNSP     -24, 12
    MOVSP        -12
    RSADDF
    RSADDF
    CPTOPSP      -20, 12
    DESTRUCT     12, 4, 4
    JSR          sub_118
    CPDOWNSP     -8, 4
    MOVSP        -4
    CONSTI       9
    CONSTI       18
    CPTOPSP      -12, 4
    ACTION       2, 3
    CPTOPSP      -24, 4
    CONSTS       "c"
    ADDSS
    ACTION       1, 1
    CONSTI       4
    ACTION       0, 1
    NOTI
    CONSTI       1
    COMPI
    ADDII
    CONSTI       -1
    SUBII
    ACTION       4, 1
    STORE_STATE  0, 24
    JMP          L_103
    CONSTO       1
    ACTION       5, 1
    RETN
L_103:
    CPTOPSP      -20, 4
    ACTION       6, 2
    MOVSP        -24
    RETN
sub_118:
    CPTOPSP      -4, 4
    CONSTF       2.0
    DIVFF
    CPDOWNSP     -12, 4
    MOVSP        -4
    JMP          L_142
    MOVSP        -4
L_142:
    MOVSP        -4
    RETN
