from unittest import TestCase

from scripts.core.auth_7702 import _hash_auth7702Message, _auth_sig

class Auth7702Testcase(TestCase):
    """
    auth_7702.py tests
    """
    def setUp(self):
        #https://github.com/ethereum/eth-account/blob/a8faa8e40c0e3c7761169a3c1e022db37643d004/eth_account/account.py#L1097
        self.chain_id = 1337
        self.delegate_to = "0x5ce9454909639d2d17a3f753ce7d93fa0b9ab12e"
        self.nonce = 1

        self.auth_hash = "9026f77ed6740d6d08f0cdc0591a86b2232700020a816718fbf760785e9ca2f2"

        self.r=52163433520757118830640642673035732532535423029712132518776649895118143897479
        self.s=57576671166887700066365341925867052133948674355067837907255957076179513983345
        self.v=27
        self.auth_sig = "0x735375048fc96b87390b5a11c411fc57245d8e55038bf49e659d048a0d1a3f877f4b3db448845cb217812f23c6b345e99f2d21c44ec10e93a8f039814167417100"    

        self.authority = '0xFbC5037aE6ebf7Fdd794BCB188B212084a580E0b'


    def test_hash_auth7702Message(self):
        """Testing hash_auth7702Message"""
        msg_hash = _hash_auth7702Message(self.chain_id, self.delegate_to, self.nonce)
        self.assertEqual(msg_hash.hex(), self.auth_hash)
    

    def test_auth_sig(self):
        """Testing auth_sig"""
        sig = _auth_sig(self.r, self.s, self.v)
        self.assertEqual(sig.to_hex(), self.auth_sig)
    

    def test_authority(self):
        """Testing authority"""
        msg_hash = _hash_auth7702Message(self.chain_id, self.delegate_to, self.nonce)
        sig = _auth_sig(self.r, self.s, self.v)
        pubk = sig.recover_public_key_from_msg(msg_hash)
        self.assertEqual(pubk.to_checksum_address(), self.authority)
        
        