from typing import List, Union, Sequence

Command = Union[int, bytes]
Commands = Sequence[Command]

StackElement = Union[int, bytes]
Stack = List[StackElement]
