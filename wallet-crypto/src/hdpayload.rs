extern crate rcw;

use self::rcw::chacha20poly1305::{ChaCha20Poly1305};
use self::rcw::aead::{AeadEncryptor, AeadDecryptor};
use self::rcw::hmac::{Hmac};
use self::rcw::sha2::{Sha512};
use self::rcw::pbkdf2::{pbkdf2};

use std::{iter::repeat, ops::{Deref}};

use hdwallet::{XPub};
use raw_cbor::{self, de::RawCbor, se::{self, Serializer}};

const NONCE : &'static [u8] = b"serokellfore";
const SALT  : &'static [u8] = b"address-hashing";
const TAG_LEN : usize = 16;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Path(Vec<u32>);
impl AsRef<[u32]> for Path {
    fn as_ref(&self) -> &[u32] { self.0.as_ref() }
}
impl Path {
    pub fn new(v: Vec<u32>) -> Self { Path(v) }
    fn from_cbor(bytes: &[u8]) -> raw_cbor::Result<Self> {
        let mut raw = RawCbor::from(bytes);
        raw_cbor::de::Deserialize::deserialize(&mut raw)
    }
    fn cbor(&self) -> Vec<u8> {
        raw_cbor::se::Serialize::serialize(self, Serializer::new())
            .expect("Serialize the given Path in cbor")
            .finalize()
    }
}
impl raw_cbor::se::Serialize for Path {
    fn serialize(&self, serializer: Serializer) -> raw_cbor::Result<Serializer> {
        se::serialize_indefinite_array(self.0.iter(), serializer)
    }
}
impl raw_cbor::Deserialize for Path {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> raw_cbor::Result<Self> {
        Ok(Path(raw.deserialize()?))
    }
}

pub const HDKEY_SIZE : usize = 32;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct HDKey([u8;HDKEY_SIZE]);
impl AsRef<[u8]> for HDKey {
    fn as_ref(&self) -> &[u8] { self.0.as_ref() }
}
impl HDKey {
    pub fn new(root_pub: &XPub) -> Self {
        let mut mac = Hmac::new(Sha512::new(), root_pub.as_ref());
        let mut result = [0;HDKEY_SIZE];
        let iters = 500;
        pbkdf2(&mut mac, &SALT[..], iters, &mut result);
        HDKey(result)
    }

    /// create a `HDKey` by taking ownership of the given bytes
    pub fn from_bytes(bytes: [u8;HDKEY_SIZE]) -> Self { HDKey(bytes) }
    /// create a `HDKey` fromt the given slice
    pub fn from_slice(bytes: &[u8]) -> Option<Self> {
        if bytes.len() == HDKEY_SIZE {
            let mut v = [0u8;HDKEY_SIZE];
            v[0..HDKEY_SIZE].clone_from_slice(bytes);
            Some(HDKey::from_bytes(v))
        } else {
            None
        }
    }

    pub fn encrypt(&self, input: &[u8]) -> Vec<u8> {
        let mut ctx = ChaCha20Poly1305::new(self.as_ref(), &NONCE[..], &[]);

        let len = input.len();

        let mut out: Vec<u8> = repeat(0).take(len).collect();
        let mut tag = [0;TAG_LEN];

        ctx.encrypt(&input, &mut out[0..len], &mut tag);
        out.extend_from_slice(&tag[..]);
        out
    }

    pub fn decrypt(&self, input: &[u8]) -> Option<Vec<u8>> {
        let len = input.len() - TAG_LEN;
        if len <= 0 { return None; };

        let mut ctx = ChaCha20Poly1305::new(self.as_ref(), &NONCE[..], &[]);

        let mut out: Vec<u8> = repeat(0).take(len).collect();

        if ctx.decrypt(&input[..len], &mut out[..], &input[len..]) {
            Some(out)
        } else {
            None
        }
    }

    pub fn encrypt_path(&self, derivation_path: &Path) -> HDAddressPayload {
        let input = derivation_path.cbor();
        let out = self.encrypt(&input);

        HDAddressPayload::from_vec(out)
    }

    pub fn decrypt_path(&self, payload: &HDAddressPayload) -> Option<Path> {
        let out = self.decrypt(payload.as_ref())?;
        Path::from_cbor(&out).ok()
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct HDAddressPayload(Vec<u8>);
impl AsRef<[u8]> for HDAddressPayload {
    fn as_ref(&self) -> &[u8] { self.0.as_ref() }
}
impl HDAddressPayload {
    pub fn from_vec(v: Vec<u8>) -> Self { HDAddressPayload(v) }
    pub fn from_bytes(bytes: &[u8]) -> Self {
        HDAddressPayload::from_vec(bytes.iter().cloned().collect())
    }
    pub fn len(&self) -> usize { self.0.len() }
}
impl raw_cbor::se::Serialize for HDAddressPayload {
    fn serialize(&self, serializer: Serializer) -> raw_cbor::Result<Serializer> {
        se::serialize_cbor_in_cbor(self.0.as_slice(), serializer)
    }
}
impl raw_cbor::de::Deserialize for HDAddressPayload {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> raw_cbor::Result<Self> {
        let mut raw_encoded = RawCbor::from(&raw.bytes()?);
        Ok(HDAddressPayload::from_bytes(&mut raw_encoded.bytes()?))
    }
}
impl Deref for HDAddressPayload {
    type Target = [u8];
    fn deref(&self) -> &Self::Target { self.0.as_ref() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hdwallet;

    #[test]
    fn encrypt() {
        let bytes = vec![42u8; 256];
        let seed = hdwallet::Seed::from_bytes([0;hdwallet::SEED_SIZE]);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);
        let pk = sk.public();

        let key = HDKey::new(&pk);
        let payload = key.encrypt(&bytes);
        assert_eq!(Some(bytes), key.decrypt(&payload))
    }

    #[test]
    fn path_cbor_encoding() {
        let path = Path::new(vec![0,1,2]);
        let cbor = path.cbor();
        assert_eq!(path, Path::from_cbor(cbor.as_ref()).unwrap());
    }

    #[test]
    fn hdpayload() {
        let path = Path::new(vec![0,1,2]);
        let seed = hdwallet::Seed::from_bytes([0;hdwallet::SEED_SIZE]);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);
        let pk = sk.public();

        let key = HDKey::new(&pk);
        let payload = key.encrypt_path(&path);
        assert_eq!(Some(path), key.decrypt_path(&payload))
    }

    #[test]
    fn unit1() {
        let key = HDKey::from_bytes([0u8;32]);
        let dat = [0x9f, 0x00, 0x01, 0x0ff];
        let expected = [0xda, 0xac, 0x4a, 0x55, 0xfc, 0xa7, 0x48, 0xf3, 0x2f, 0xfa, 0xf4, 0x9e, 0x2b, 0x41, 0xab, 0x86, 0xf3, 0x54, 0xdb, 0x96];
        let got = key.encrypt(&dat[..]);
        assert_eq!(&expected[..], &got[..])
    }

    #[test]
    fn unit2() {
        let path = Path::new(vec![0,1]);
        let expected = [0x9f, 0x00, 0x01, 0x0ff];
        let cbor = path.cbor();
        assert_eq!(&expected[..], &cbor[..])
    }
}

#[cfg(test)]
#[cfg(feature = "with-bench")]
mod bench {
    use hdwallet;
    use hdpayload::{self, *};
    use test;

    #[bench]
    fn decrypt_fail(b: &mut test::Bencher) {
        let path = Path::new(vec![0,1]);
        let seed = hdwallet::Seed::from_bytes([0;hdwallet::SEED_SIZE]);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);
        let pk = sk.public();

        let key = HDKey::new(&pk);
        let payload = key.encrypt_path(&path);

        let seed = hdwallet::Seed::from_bytes([1;hdwallet::SEED_SIZE]);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);
        let pk = sk.public();
        let key = HDKey::new(&pk);
        b.iter(|| {
            let _ = key.decrypt(&payload);
        })
    }

    #[bench]
    fn decrypt_ok(b: &mut test::Bencher) {
        let path = Path::new(vec![0,1]);
        let seed = hdwallet::Seed::from_bytes([0;hdwallet::SEED_SIZE]);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);
        let pk = sk.public();

        let key = HDKey::new(&pk);
        let payload = key.encrypt_path(&path);

        b.iter(|| {
            let _ = key.decrypt(&payload);
        })
    }

    #[bench]
    fn decrypt_with_cbor(b: &mut test::Bencher) {
        let path = Path::new(vec![0,1]);
        let seed = hdwallet::Seed::from_bytes([0;hdwallet::SEED_SIZE]);
        let sk = hdwallet::XPrv::generate_from_seed(&seed);
        let pk = sk.public();

        let key = HDKey::new(&pk);
        let payload = key.encrypt_path(&path);

        b.iter(|| {
            let _ = key.decrypt_path(&payload);
        })
    }
}
