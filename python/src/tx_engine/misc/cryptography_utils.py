from tx_engine import Wallet

from cryptography.hazmat.primitives.serialization import load_pem_private_key
from cryptography.hazmat.backends import default_backend

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

    return Wallet.create_wallet_from_bytes(network, private_key_bytes)
