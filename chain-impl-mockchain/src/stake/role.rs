use crate::key::{deserialize_public_key, serialize_public_key, Hash};
use crate::leadership::genesis::GenesisPraosLeader;
use chain_core::mempack::{ReadBuf, ReadError, Readable};
use chain_core::property;
use chain_crypto::{Ed25519Extended, PublicKey, SecretKey};

/// Information related to a stake key
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
pub struct StakeKeyInfo {
    pub(crate) pool: Option<StakePoolId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
pub struct StakePoolId(Hash);

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
pub struct StakePoolInfo {
    pub serial: u128,
    pub owners: Vec<StakeKeyId>,
    pub initial_key: GenesisPraosLeader,
}

impl StakePoolInfo {
    pub fn to_id(&self) -> StakePoolId {
        let mut v = Vec::new();
        v.extend_from_slice(&self.serial.to_be_bytes());
        for o in &self.owners {
            v.extend_from_slice(o.0.as_ref())
        }
        v.extend_from_slice(self.initial_key.kes_public_key.as_ref());
        v.extend_from_slice(self.initial_key.vrf_public_key.as_ref());
        StakePoolId(Hash::hash_bytes(&v))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
pub struct StakeKeyId(pub(crate) PublicKey<Ed25519Extended>);

impl From<PublicKey<Ed25519Extended>> for StakeKeyId {
    fn from(key: PublicKey<Ed25519Extended>) -> Self {
        StakeKeyId(key)
    }
}

impl From<&SecretKey<Ed25519Extended>> for StakeKeyId {
    fn from(key: &SecretKey<Ed25519Extended>) -> Self {
        StakeKeyId(key.to_public())
    }
}

impl property::Serialize for StakeKeyId {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        serialize_public_key(&self.0, writer)
    }
}

impl Readable for StakeKeyId {
    fn read<'a>(reader: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        deserialize_public_key(reader).map(StakeKeyId)
    }
}

impl property::Serialize for StakePoolId {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, mut writer: W) -> Result<(), Self::Error> {
        writer.write_all(self.0.as_ref())
    }
}

impl Readable for StakePoolId {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        Hash::read(buf).map(StakePoolId)
    }
}

impl property::Serialize for GenesisPraosLeader {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, mut writer: W) -> Result<(), Self::Error> {
        serialize_public_key(&self.kes_public_key, &mut writer)?;
        serialize_public_key(&self.vrf_public_key, &mut writer)?;
        Ok(())
    }
}

impl Readable for GenesisPraosLeader {
    fn read<'a>(reader: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let kes_public_key = deserialize_public_key(reader)?;
        let vrf_public_key = deserialize_public_key(reader)?;
        Ok(GenesisPraosLeader {
            vrf_public_key,
            kes_public_key,
        })
    }
}

impl property::Serialize for StakePoolInfo {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        assert!(self.owners.len() < 256);

        use chain_core::packer::Codec;

        let mut codec = Codec::from(writer);
        codec.put_u128(self.serial)?;
        codec.put_u8(self.owners.len() as u8)?;
        for o in &self.owners {
            serialize_public_key(&o.0, &mut codec)?;
        }
        self.initial_key.serialize(&mut codec)?;
        Ok(())
    }
}

impl Readable for StakePoolInfo {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let serial = buf.get_u128()?;
        let owner_nb = buf.get_u8()? as usize;
        let mut owners = Vec::with_capacity(owner_nb);
        for _ in 0..owner_nb {
            let pub_key = deserialize_public_key(buf)?;
            owners.push(StakeKeyId(pub_key))
        }
        let initial_key = GenesisPraosLeader::read(buf)?;

        Ok(StakePoolInfo {
            serial,
            owners,
            initial_key,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for StakeKeyId {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            StakeKeyId::from(&Arbitrary::arbitrary(g))
        }
    }

    impl Arbitrary for StakePoolId {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            StakePoolId(Arbitrary::arbitrary(g))
        }
    }
}
