from engine.script import Script
from engine.context import Context


def main():
    if True:
        s = Script.parse_string("OP_10 OP_5 OP_DIV OP_0, OP_1")
        c = Context(script=s)
        ret = c.evaluate()
        print(f"ret = {ret}")
        stack = c.get_stack()
        print(f"stack = '{stack}'")

    s = Script.parse_string("OP_DROP")
    c = Context(script=s)
    ret = c.evaluate()
    print(f"ret = {ret}")
    if ret:
        stack = c.get_stack()
        print(f"stack = '{stack}'")

    serial = s.raw_serialize()
    print(f"serial = {serial.hex()}")


if __name__ == '__main__':
    main()
