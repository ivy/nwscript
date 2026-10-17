; Hand-assembled; the shape nwnnsscomp -v1.00 gives a script with a global, a
; function with parameters and returns, while/break, do/while, for, switch
; with a braced case and a default, && and ||, a deferred action and ++ on a global.
    JSR          sub_15
    RETN
sub_15:
    RSADDI
    CONSTI       3
    CPDOWNSP     -8, 4
    MOVSP        -4
    SAVEBP
    JSR          sub_3d
    RESTOREBP
    MOVSP        -4
    RETN
sub_3d:
    RSADDI
    CONSTI       0
    CPDOWNSP     -8, 4
    MOVSP        -4
L_53:
    CPTOPSP      -4, 4
    CONSTI       10
    LTII
    JZ           L_a5
    CPTOPSP      -4, 4
    INCISP       -8
    MOVSP        -4
    CPTOPSP      -4, 4
    CONSTI       5
    EQUALII
    JZ           L_9f
    JMP          L_a5
    JMP          L_9f
L_9f:
    JMP          L_53
L_a5:
    CPTOPSP      -4, 4
    DECISP       -8
    MOVSP        -4
    CPTOPSP      -4, 4
    CONSTI       0
    GTII
    JZ           L_d5
    JMP          L_a5
L_d5:
    CONSTI       0
    CPDOWNSP     -8, 4
    MOVSP        -4
L_e9:
    CPTOPSP      -4, 4
    CONSTI       3
    LTII
    JZ           L_126
    CPTOPSP      -4, 4
    ACTION       4, 1
    CPTOPSP      -4, 4
    INCISP       -8
    MOVSP        -4
    JMP          L_e9
L_126:
    CPTOPBP      -4, 4
    CPTOPSP      -4, 4
    CONSTI       1
    EQUALII
    JNZ          L_160
    CPTOPSP      -4, 4
    CONSTI       2
    EQUALII
    JNZ          L_171
    JMP          L_1a0
L_160:
    CONSTI       1
    ACTION       4, 1
    JMP          L_1ab
L_171:
    RSADDI
    CONSTI       4
    CPDOWNSP     -8, 4
    MOVSP        -4
    CPTOPSP      -4, 4
    ACTION       4, 1
    MOVSP        -4
    JMP          L_1ab
L_1a0:
    CONSTI       9
    ACTION       4, 1
L_1ab:
    MOVSP        -4
    CPTOPSP      -4, 4
    CONSTI       1
    GTII
    CPTOPSP      -4, 4
    JZ           L_1e1
    CPTOPBP      -4, 4
    CONSTI       2
    LTII
    LOGANDII
L_1e1:
    CPTOPSP      -4, 4
    JZ           L_1fd
    CPTOPSP      -4, 4
    JZ           L_20d
L_1fd:
    CPTOPSP      -8, 4
    CONSTI       0
    EQUALII
L_20d:
    LOGORII
    JZ           L_238
    RSADDI
    CONSTI       1
    NEGI
    CPTOPSP      -12, 4
    JSR          sub_27e
    ACTION       4, 1
    JMP          L_238
L_238:
    STORE_STATE  4, 4
    JMP          L_257
    CPTOPSP      -4, 4
    ACTION       4, 1
    RETN
L_257:
    CONSTF       1.0
    ACTION       7, 2
    INCIBP       -4
    CPTOPBP      -4, 4
    MOVSP        -4
    MOVSP        -4
    RETN
sub_27e:
    RSADDI
    CPTOPSP      -8, 4
    CPTOPSP      -16, 4
    ADDII
    CPDOWNSP     -8, 4
    MOVSP        -4
    CPTOPSP      -4, 4
    CONSTI       10
    GTII
    JZ           L_2dc
    CONSTI       10
    CPDOWNSP     -20, 4
    MOVSP        -8
    JMP          L_340
    MOVSP        -4
    JMP          L_318
L_2dc:
    CPTOPSP      -4, 4
    CONSTI       0
    LTII
    JZ           L_318
    CONSTI       0
    CPDOWNSP     -20, 4
    MOVSP        -8
    JMP          L_340
    MOVSP        -4
    JMP          L_318
L_318:
    CPTOPSP      -4, 4
    CPDOWNSP     -20, 4
    MOVSP        -8
    JMP          L_340
    MOVSP        -4
    MOVSP        -4
L_340:
    MOVSP        -8
    RETN
