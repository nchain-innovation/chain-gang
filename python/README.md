

Hi John, this is the repo I'm working on: https://bitbucket.stressedsharks.com/users/f.barbacovi_nchain.com/repos/script_libraries/browse, 

disregarding the fact that tx_engine is in there, 

if you enter lib/ellipticcurves/ec_arithmetic_Fq, 
you will see that I actually only use Script, pick and roll 

(which I defined to return OP_i OP_PICK as many times as I need, e.g., pick(position=3,nElements=2) -> OP_3 OP_PICK OP_3 OP_PICK, 

you can find them in tx_engine/engine/utility_scripts.py).

I think that the only thing I need is:
Script
Script + Script = Script
Script.parse_string
Script.raw_serialize
The debugger (or, more generally, the Context class)

Built using Python3 version 3.11.2
