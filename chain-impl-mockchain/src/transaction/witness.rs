use super::transaction::*;
use crate::account;
use crate::key::{
    deserialize_public_key, deserialize_signature, serialize_public_key, serialize_signature,
    AccountSecretKey, AccountSignature, SpendingPublicKey, SpendingSecretKey, SpendingSignature,
};
use chain_core::mempack::{ReadBuf, ReadError, Readable};
use chain_core::property;
use chain_crypto::{Ed25519Bip32, PublicKey, Signature, Verification};

/// Structure that proofs that certain user agrees with
/// some data. This structure is used to sign `Transaction`
/// and get `SignedTransaction` out.
///
/// It's important that witness works with opaque structures
/// and may not know the contents of the internal transaction.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
pub enum Witness {
    Utxo(SpendingSignature<TransactionId>),
    Account(SpendingSignature<TransactionIdSpendingCounter>),
    OldUtxo(
        PublicKey<Ed25519Bip32>,
        Signature<TransactionId, Ed25519Bip32>,
    ),
}

impl PartialEq for Witness {
    fn eq(&self, rhs: &Self) -> bool {
        match (self, rhs) {
            (Witness::Utxo(s1), Witness::Utxo(s2)) => s1.as_ref() == s2.as_ref(),
            (Witness::Account(s1), Witness::Account(s2)) => s1.as_ref() == s2.as_ref(),
            (Witness::OldUtxo(p1, s1), Witness::OldUtxo(p2, s2)) => {
                s1.as_ref() == s2.as_ref() && p1 == p2
            }
            (_, _) => false,
        }
    }
}
impl Eq for Witness {}

pub struct TransactionIdSpendingCounter(Vec<u8>);

impl TransactionIdSpendingCounter {
    pub fn new(
        transaction_id: &TransactionId,
        spending_counter: &account::SpendingCounter,
    ) -> Self {
        let mut v = Vec::new();
        v[0] = WITNESS_TAG_ACCOUNT;
        v.extend_from_slice(transaction_id.as_ref());
        v.extend_from_slice(&spending_counter.to_bytes());
        TransactionIdSpendingCounter(v)
    }
}

impl AsRef<[u8]> for TransactionIdSpendingCounter {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl Witness {
    /// Creates new `Witness` value.
    pub fn new_utxo(transaction_id: &TransactionId, secret_key: &SpendingSecretKey) -> Self {
        Witness::Utxo(SpendingSignature::generate(secret_key, transaction_id))
    }

    pub fn new_account(
        transaction_id: &TransactionId,
        spending_counter: &account::SpendingCounter,
        secret_key: &AccountSecretKey,
    ) -> Self {
        Witness::Account(AccountSignature::generate(
            secret_key,
            &TransactionIdSpendingCounter::new(transaction_id, spending_counter),
        ))
    }

    /// Verify the given `TransactionId` using the witness.
    pub fn verify_utxo(
        &self,
        public_key: &SpendingPublicKey,
        transaction_id: &TransactionId,
    ) -> Verification {
        match self {
            Witness::OldUtxo(_xpub, _signature) => unimplemented!(),
            Witness::Utxo(signature) => signature.verify(public_key, transaction_id),
            Witness::Account(_) => Verification::Failed,
        }
    }
}

const WITNESS_TAG_OLDUTXO: u8 = 0u8;
const WITNESS_TAG_UTXO: u8 = 1u8;
const WITNESS_TAG_ACCOUNT: u8 = 2u8;

impl property::Serialize for Witness {
    type Error = std::io::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        //use chain_core::property::Serialize;

        let mut codec = Codec::from(writer);
        match self {
            Witness::OldUtxo(xpub, sig) => {
                codec.put_u8(WITNESS_TAG_OLDUTXO)?;
                serialize_public_key(xpub, &mut codec)?;
                serialize_signature(sig, &mut codec)
            }
            Witness::Utxo(sig) => {
                codec.put_u8(WITNESS_TAG_UTXO)?;
                serialize_signature(sig, codec.into_inner())
            }
            Witness::Account(sig) => {
                codec.put_u8(WITNESS_TAG_ACCOUNT)?;
                serialize_signature(sig, codec.into_inner())
            }
        }
    }
}

impl Readable for Witness {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        match buf.get_u8()? {
            WITNESS_TAG_OLDUTXO => {
                let xpub = deserialize_public_key(buf)?;
                let sig = deserialize_signature(buf)?;
                Ok(Witness::OldUtxo(xpub, sig))
            }
            WITNESS_TAG_UTXO => deserialize_signature(buf).map(Witness::Utxo),
            WITNESS_TAG_ACCOUNT => deserialize_signature(buf).map(Witness::Account),
            i => Err(ReadError::UnknownTag(i as u32)),
        }
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    #[derive(Clone)]
    pub struct TransactionSigningKey(pub SpendingSecretKey);

    impl std::fmt::Debug for TransactionSigningKey {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "TransactionSigningKey(<secret-key>)")
        }
    }

    impl Arbitrary for TransactionSigningKey {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            use rand_chacha::ChaChaRng;
            use rand_core::SeedableRng;
            let mut seed = [0; 32];
            for byte in seed.iter_mut() {
                *byte = Arbitrary::arbitrary(g);
            }
            let mut rng = ChaChaRng::from_seed(seed);
            TransactionSigningKey(SpendingSecretKey::generate(&mut rng))
        }
    }

    impl Arbitrary for Witness {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let sk = TransactionSigningKey::arbitrary(g);
            let txid = TransactionId::arbitrary(g);
            Witness::Utxo(SpendingSignature::generate(&sk.0, &txid))
        }
    }

    quickcheck! {

        /// ```
        /// \forall w=Witness(tx) => w.verifies(tx)
        /// ```
        fn prop_witness_verifies_own_tx(sk: TransactionSigningKey, tx:TransactionId) -> bool {
            let pk = sk.0.to_public();
            let witness = Witness::new_utxo(&tx, &sk.0);
            witness.verify_utxo(&pk, &tx) == Verification::Success
        }
    }
}
