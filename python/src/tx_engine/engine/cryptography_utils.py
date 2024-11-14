from tx_engine import Wallet

from cryptography.hazmat.primitives.serialization import load_pem_private_key
from cryptography.hazmat.backends import default_backend
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.hazmat.primitives import serialization


def bigint_to_private_key(private_key_int: int) -> ec.EllipticCurvePrivateKey:
    '''Convert a BigInt to an ECDSA private key.
    '''
    return ec.derive_private_key(private_key_int, ec.SECP256K1())


def create_wallet_from_pem_file(pem_file_path: str, network: str) -> Wallet:
    # Load the PEM file
    with open(pem_file_path, 'rb') as pem_file:
        pem_data = pem_file.read()

    return create_wallet_from_pem_bytes(pem_data, network)


def create_wallet_from_pem_bytes(pem_data: bytes, network: str) -> Wallet:
    # Load the private key from the PEM data
    private_key = load_pem_private_key(pem_data, password=None, backend=default_backend())

    # Extract the private numbers (this includes the scalar/private key value)
    private_numbers = private_key.private_numbers()

    # The scalar value of the private key (as an integer)
    private_key_scalar = private_numbers.private_value

    # Convert the scalar value to bytes
    private_key_bytes = private_key_scalar.to_bytes((private_key_scalar.bit_length() + 7) // 8, byteorder='big')

    return Wallet.from_bytes(network, private_key_bytes)


def create_pem_from_wallet(user_wallet: Wallet) -> str:
    ec_pri_key: ec.EllipticCurvePrivateKey = bigint_to_private_key(user_wallet.to_int())
    pem = ec_pri_key.private_bytes(encoding=serialization.Encoding.PEM,
                                   format=serialization.PrivateFormat.PKCS8,
                                   encryption_algorithm=serialization.NoEncryption()
                                   )
    return pem.decode('utf-8')
