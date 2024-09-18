""" This contains the base types used by tx-engine
"""
from typing import List, Union, MutableSequence

Command = Union[int, bytes]
Commands = MutableSequence[Command]

StackElement = Union[int, bytes]
Stack = List[StackElement]
