#!/usr/bin/python3
import unittest
import sys
sys.path.append("..")


from typing import Optional
from tx_engine import Script, Context, encode_num
import sys


def pick(position: int, nElements: int) -> Script:
    '''
    Script to pick nElements starting from position.
    Position is the stack position, so we star counting from 0.
    Example:
        nElements = 2, position = 2 --> OP_2 OP_PICK OP_2 OP_PICK
    '''
    if position < 17:
        return Script.parse_string(' '.join([str(position), 'OP_PICK'] * nElements))
    else:
        return Script.parse_string(' '.join(['0x' + encode_num(position).hex(), 'OP_PICK'] * nElements))


class EllipticCurveFq:
    """
    Elliptic curve arithemtic over Fq
    """

    def __init__(self, q: int, curve_a: int):
        # Characteristic of the field over which the curve is defined
        self.MODULUS = q
        # A coffiecient of the curve over which we are performing the operations
        self.CURVE_A = curve_a

    def point_addition_with_unknown_points(self, take_modulo: bool, check_constant: Optional[bool] = None, clean_constant: Optional[bool] = None) -> Script:
        '''
        Input Parameters:
            - Stack: q .. <lambda> P Q
            - Altstack: []
        Output:
            - P + Q
        Assumption on parameters:
            - P and Q are points on E(F_q), passed as couple of integers (minimally encoded, little endian), with coordinates in F_q
            - If P != -Q, then lambda is the gradient of the line through P and Q, passed as an integers (minimally encoded, little endian)
            - If P = -Q or P is the point at infinity, or Q is the point at infinity, then do not put lambda
        REMARKS:
            - If take_modulo = True, the coordinates of P + Q are in F_q
            - If P = -Q, then we return 0x00 0x00, i.e., we encode the point at infinity as (0x00,0x00) (notice that these are data payloads, they are not numbers - points are assumed to be passed as numbers, which means that (0,0) would have to be passed as OP_0 OP_0)
        '''

        if check_constant:
            out = Script.parse_string('OP_DEPTH OP_1SUB OP_PICK') + Script.parse_string('0x' + encode_num(self.MODULUS).hex()) + Script.parse_string('OP_EQUALVERIFY')
        else:
            out = Script()

        curve_a = self.CURVE_A

        # Check if Q or P is point at infinity or if P = - Q --------------------------------------------------------------------------
        # After this, the stack is: <lambda> P Q
        # Check if Q is (0x00,0x00), in that case, terminate and return P
        out += Script.parse_string('OP_2DUP OP_CAT 0x0000 OP_EQUAL OP_NOT')
        out += Script.parse_string('OP_IF')

        # Check if P is (0x00,0x00), in that case, terminate and return Q
        out += Script.parse_string('OP_2OVER OP_CAT 0x0000 OP_EQUAL OP_NOT')
        out += Script.parse_string('OP_IF')

        # Check if P = -Q, in that case terminate and return (0x00,0x00)
        out += Script.parse_string('OP_DUP')																# Duplicate yQ
        out += pick(position=3, nElements=1)																	# Pick yP
        out += Script.parse_string('OP_ADD')
        out += Script.parse_string('OP_DEPTH OP_1SUB OP_PICK OP_MOD OP_0 OP_NUMNOTEQUAL')
        out += Script.parse_string('OP_IF')

        # End of initial checks  ------------------------------------------------------------------------------------------------------

        # Validate lambda -------------------------------------------------------------------------------------------------------------
        # After this, the stack is: <lambda> P Q, altstack = [Verify(lambda)]

        # Check if P = Q:
        out += Script.parse_string('OP_2OVER OP_2OVER')														# Roll P and Q
        out += Script.parse_string('OP_CAT')																# Concatenate xQ||yQ
        out += Script.parse_string('OP_ROT OP_ROT')															# Rotate xP and xQ
        out += Script.parse_string('OP_CAT')																# Concatenate xP||yP
        out += Script.parse_string('OP_EQUAL')																# Check xP||yP = xQ||yQ

        # If P = Q:
        out += Script.parse_string('OP_IF')
        out += Script.parse_string('OP_DUP')																# Duplicate y_P
        out += Script.parse_string('OP_2 OP_MUL')															# Compute 2y_P
        out += pick(position=3, nElements=1)																	# Pick lambda
        out += Script.parse_string('OP_MUL')																# Compute 2 lambda y_P
        out += pick(position=2, nElements=1)																	# Pick x_P
        out += Script.parse_string('OP_DUP')																# Duplicate x_P
        out += Script.parse_string('OP_MUL')																# Compute x_P^2
        out += Script.parse_string('OP_3 OP_MUL')															# Compute 3 x_P^2
        if curve_a != 0:
            out += Script.parse_string('0x' + encode_num(curve_a).hex() + ' OP_ADD')											# Compute 3 x_P^2 + a if a != 0
        out += Script.parse_string('OP_SUB')

        # If P != Q:
        out += Script.parse_string('OP_ELSE')
        out += pick(position=4, nElements=2)																	# Pick lambda and x_P
        out += Script.parse_string('OP_MUL OP_ADD')
        out += Script.parse_string('OP_OVER OP_5 OP_PICK OP_MUL OP_3 OP_PICK OP_ADD')						# compute lambda x_Q + y_P
        out += Script.parse_string('OP_SUB')
        out += Script.parse_string('OP_ENDIF')

        # Place on the altstack
        out += Script.parse_string('OP_TOALTSTACK')

        # End of lambda validation ---------------------------------------------------------------------------------------------------

        # Calculation of P + Q
        # After this, the stack is: (P+Q), altstack = [Verify(lambda)]

        # Compute x_(P+Q) = lambda^2 - x_P - x_Q
        # After this, the base stack is: <lambda> x_P y_P x_(P+Q), altstack = [Verify(lambda)]
        compute_x_coordinate = Script.parse_string('OP_2OVER')
        compute_x_coordinate += Script.parse_string('OP_SWAP')
        compute_x_coordinate += Script.parse_string('OP_DUP OP_MUL')										# Compute lambda^2
        compute_x_coordinate += Script.parse_string('OP_ROT OP_ROT OP_ADD OP_SUB')							# Compute lambda^2 - (x_P + x_Q)

        # Compute y_(P+Q) = lambda (x_P - x_(P+Q)) - y_P
        # After this, the stack is: x_(P+Q) y_(P+Q), altstack = [Verify(lambda)]
        compute_y_coordinate = Script.parse_string('OP_TUCK')
        compute_y_coordinate += Script.parse_string('OP_2SWAP')
        compute_y_coordinate += Script.parse_string('OP_SUB')												# Compute xP - x_(P+Q)
        compute_y_coordinate += Script.parse_string('OP_2SWAP OP_TOALTSTACK')
        compute_y_coordinate += Script.parse_string('OP_MUL OP_FROMALTSTACK OP_SUB')						# Compute lambda (x_P - x_(P+Q)) - y_P

        fetch_q = Script.parse_string('OP_DEPTH OP_1SUB OP_PICK')

        # After this, the stack is: (P+Q) q, altstack = [Verify(lambda)]
        out += compute_x_coordinate + compute_y_coordinate + fetch_q

        batched_modulo = Script.parse_string('OP_TUCK OP_MOD OP_OVER OP_ADD OP_OVER OP_MOD')				# Mod y
        batched_modulo += Script.parse_string('OP_TOALTSTACK')
        batched_modulo += Script.parse_string('OP_TUCK OP_MOD OP_OVER OP_ADD OP_OVER OP_MOD')				# Mod x
        batched_modulo += Script.parse_string('OP_FROMALTSTACK OP_ROT')

        # If needed, mod out
        # After this, the stack is: (P+Q) q, altstack = [Verify(lambda)] with the coefficients in Fq (if executed)
        if take_modulo:
            out += batched_modulo

        check_lambda = Script.parse_string('OP_FROMALTSTACK')
        check_lambda += Script.parse_string('OP_OVER OP_MOD OP_OVER OP_ADD OP_SWAP OP_MOD')					# Mod lambda * (yP - yQ) - (xP- xQ)
        check_lambda += Script.parse_string('OP_0 OP_EQUALVERIFY')

        # Check lambda was correct
        out += check_lambda

        # Termination conditions  ------------------------------------------------------------------------------------------------------

        # Termination because P = -Q
        out += Script.parse_string('OP_ELSE')
        out += Script.parse_string('OP_2DROP OP_2DROP')
        out += Script.parse_string('0x00 0x00')
        out += Script.parse_string('OP_ENDIF')

        # Termination because P = (0x00,0x00)
        out += Script.parse_string('OP_ELSE')
        out += Script.parse_string('OP_2SWAP OP_2DROP')
        out += Script.parse_string('OP_ENDIF')

        # Termination because Q = (0x00,0x00)
        out += Script.parse_string('OP_ELSE')
        out += Script.parse_string('OP_2DROP')
        out += Script.parse_string('OP_ENDIF')

        # End of termination conditions ------------------------------------------------------------------------------------------------

        if clean_constant:
            out += Script.parse_string('OP_DEPTH OP_1SUB OP_ROLL OP_DROP')
        return out


secp256k1_MODULUS = 115792089237316195423570985008687907853269984665640564039457584007908834671663
secp256k1_script = EllipticCurveFq(q=secp256k1_MODULUS, curve_a=0)


class FedTest(unittest.TestCase):

    def test_scripts(self):
        # unlocking script
        unlock = Script.parse(b'\xca!/\xfc\xff\xff\xfe\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\x00!\x97\x1d\t\xaf\xd2t\xee\xc5[\xb5\x10\xce\x8fql\xc9\x13\xe8+\xdb\x89\x06\xcfc\x042X&\xaa\xb2\x85\xc2\x00 \xd9\x02a\xaf\x87Q\xe9j\xe2k)J\x0b\xff\xc4\xe1\xc2\x8c\xd1h\xb4\x84\xff\xd4\x07\xfa\xa9fk\x1d\xc7!!a\xca\xb3|F]\x16\x11\xd6<\xdf\xd5\xfa\rA~\xbd)`\x7f\xee\x01/\xca\x04Z\xcb\x97\xba%\xdf\xd4\x00 \x0c\xbeT\xbeq\xa4\xed\xd3fn"X\x99\x06\x93\x1f\x8f\x8c? \x0b\xe9^\xd4\xa9\x9c-e\xfd\x16\xe8K!\xb2\x07\x817*y5\x90\xe2od\xc0\xc9\xcc\xe4)\xc1\x14\xc3\xdc\xdaL\xb1\xce\x97D5\n\x80\xd0\xb3\xeb\x00')

        lock = secp256k1_script.point_addition_with_unknown_points(take_modulo=True, check_constant=True, clean_constant=True)
        lock += Script.parse(b"E!\xf8\xbf\x9c\xe6@\xd7\xc0'\x1d\x16$\xb2\x1f\xcb(ti\x02\xdf6\xe7m\x91\x88[#\xca'B<M\x86\x00\x88 \x19*:\x14\xe2C\xa8\xb1\xae\xfc\xa8\xba\xc7?F\x02\xd0\\\xe0\xa4\xb5\x0b\xddp\xa0o\xbe\xed\xbb\x1b\xc9\x7f\x87")

        context = Context(script=unlock + lock)
        self.assertTrue(context.evaluate())
        self.assertEqual(len(context.get_stack()), 1)
        self.assertEqual(len(context.get_altstack()), 0)


if __name__ == '__main__':
    unittest.main()
