use super::*;

use quickcheck::{Arbitrary, Gen};
use rand::SeedableRng;
use rand_chacha::ChaChaRng;

pub fn arbitrary_public_key<A: AsymmetricKey, G: Gen>(g: &mut G) -> PublicKey<A> {
    arbitrary_secret_key(g).to_public()
}

pub fn arbitrary_secret_key<A, G>(g: &mut G) -> SecretKey<A>
where
    A: AsymmetricKey,
    G: Gen,
{
    let rng = ChaChaRng::seed_from_u64(Arbitrary::arbitrary(g));
    SecretKey::generate(rng)
}

impl<A> Arbitrary for PublicKey<A>
where
    A: AsymmetricKey + 'static,
    A::Public: Send,
{
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        arbitrary_public_key(g)
    }
}
impl<A> Arbitrary for SecretKey<A>
where
    A: AsymmetricKey + 'static,
    A::Secret: Send,
{
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        arbitrary_secret_key(g)
    }
}
impl<A> Arbitrary for KeyPair<A>
where
    A: AsymmetricKey + 'static,
    A::Secret: Send,
    A::Public: Send,
{
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let secret_key = SecretKey::arbitrary(g);
        KeyPair::from(secret_key)
    }
}

impl Arbitrary for Blake2b224 {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let bytes: Vec<_> = std::iter::repeat_with(|| u8::arbitrary(g))
            .take(Self::HASH_SIZE)
            .collect();
        Self::try_from_slice(&bytes).unwrap()
    }
}
impl Arbitrary for Blake2b256 {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let bytes: Vec<_> = std::iter::repeat_with(|| u8::arbitrary(g))
            .take(Self::HASH_SIZE)
            .collect();
        Self::try_from_slice(&bytes).unwrap()
    }
}

impl Arbitrary for Sha3_256 {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let bytes: Vec<_> = std::iter::repeat_with(|| u8::arbitrary(g))
            .take(Self::HASH_SIZE)
            .collect();
        Self::try_from_slice(&bytes).unwrap()
    }
}
