//! Representation of the block in the mockchain.
use crate::key::Hash;
use crate::message::{Message, MessageRaw};
use chain_core::mempack::read_from_raw;
use chain_core::property::{self, Serialize};

mod builder;
//mod cstruct;
mod header;
mod headerraw;
mod version;

pub use self::version::{AnyBlockVersion, BlockVersion, ConsensusVersion};

pub use self::builder::BlockBuilder;

pub use self::header::{
    BftProof, BftSignature, BlockContentHash, BlockContentSize, BlockId, ChainLength, Common,
    GenesisPraosProof, Header, HeaderHash, KESSignature, Proof,
};
pub use self::headerraw::HeaderRaw;
pub use self::version::*;

pub use crate::date::{BlockDate, BlockDateParseError, Epoch, SlotId};

/// `Block` is an element of the blockchain it contains multiple
/// transaction and a reference to the parent block. Alongside
/// with the position of that block in the chain.
#[derive(Debug, Clone)]
pub struct Block {
    pub header: Header,
    pub contents: BlockContents,
}

impl PartialEq for Block {
    fn eq(&self, rhs: &Self) -> bool {
        self.header.hash() == rhs.header.hash()
    }
}
impl Eq for Block {}

#[derive(Debug, Clone)]
pub struct BlockContents(Vec<Message>);

impl PartialEq for BlockContents {
    fn eq(&self, rhs: &Self) -> bool {
        self.compute_hash_size() == rhs.compute_hash_size()
    }
}
impl Eq for BlockContents {}

impl BlockContents {
    #[inline]
    pub fn new(messages: Vec<Message>) -> Self {
        BlockContents(messages)
    }
    #[inline]
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a Message> {
        self.0.iter()
    }
    pub fn compute_hash_size(&self) -> (BlockContentHash, usize) {
        let mut bytes = Vec::with_capacity(4096);

        for message in self.iter() {
            message.to_raw().serialize(&mut bytes).unwrap();
        }

        let hash = Hash::hash_bytes(&bytes);
        (hash, bytes.len())
    }
}

impl Block {
    pub fn is_consistent(&self) -> bool {
        let (content_hash, content_size) = self.contents.compute_hash_size();

        &content_hash == self.header.block_content_hash()
            && content_size == self.header.common.block_content_size as usize
    }
}

impl property::Block for Block {
    type Id = BlockId;
    type Date = BlockDate;
    type Version = AnyBlockVersion;
    type ChainLength = ChainLength;

    /// Identifier of the block, currently the hash of the
    /// serialized transaction.
    fn id(&self) -> Self::Id {
        self.header.hash()
    }

    /// Id of the parent block.
    fn parent_id(&self) -> Self::Id {
        *self.header.block_parent_hash()
    }

    /// Date of the block.
    fn date(&self) -> Self::Date {
        *self.header.block_date()
    }

    fn version(&self) -> Self::Version {
        self.header.block_version()
    }

    fn chain_length(&self) -> Self::ChainLength {
        self.header.chain_length()
    }
}

impl property::Serialize for Block {
    type Error = std::io::Error;

    fn serialize<W: std::io::Write>(&self, mut writer: W) -> Result<(), Self::Error> {
        let header_raw = {
            let mut v = Vec::new();
            self.header.serialize(&mut v)?;
            HeaderRaw(v)
        };
        header_raw.serialize(&mut writer)?;

        for message in self.contents.iter() {
            let message_raw = message.to_raw();
            message_raw.serialize(&mut writer)?;
        }
        Ok(())
    }
}

impl property::Deserialize for Block {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(mut reader: R) -> Result<Self, Self::Error> {
        let header_raw = HeaderRaw::deserialize(&mut reader)?;
        let header = read_from_raw::<Header>(header_raw.as_ref())?;

        let mut serialized_content_size = header.common.block_content_size;
        let mut contents = BlockContents(Vec::with_capacity(4));

        while serialized_content_size > 0 {
            let message_raw = MessageRaw::deserialize(&mut reader)?;
            let message_size = message_raw.size_bytes_plus_size();

            // return error here if message serialize sized is bigger than remaining size

            let message = Message::from_raw(&message_raw)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
            contents.0.push(message);

            serialized_content_size -= message_size as u32;
        }

        Ok(Block {
            header: header,
            contents: contents,
        })
    }
}

impl property::HasMessages for Block {
    type Message = Message;
    fn messages<'a>(&'a self) -> Box<Iterator<Item = &Message> + 'a> {
        Box::new(self.contents.iter())
    }

    fn for_each_message<F>(&self, mut f: F)
    where
        F: FnMut(&Self::Message),
    {
        self.contents.iter().for_each(|msg| f(msg))
    }
}

impl property::HasHeader for Block {
    type Header = Header;
    fn header(&self) -> Self::Header {
        self.header.clone()
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use quickcheck::{Arbitrary, Gen, TestResult};

    quickcheck! {
        fn headerraw_serialization_bijection(b: HeaderRaw) -> TestResult {
            property::testing::serialization_bijection(b)
        }

        fn header_serialization_bijection(b: Header) -> TestResult {
            property::testing::serialization_bijection_r(b)
        }

        fn block_serialization_bijection(b: Block) -> TestResult {
            property::testing::serialization_bijection(b)
        }
    }

    impl Arbitrary for HeaderRaw {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let len = u16::arbitrary(g);
            let mut v = Vec::new();
            for _ in 0..len {
                v.push(u8::arbitrary(g))
            }
            HeaderRaw(v)
        }
    }

    impl Arbitrary for BlockContents {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let len = u8::arbitrary(g) % 12;
            BlockContents(
                std::iter::repeat_with(|| Arbitrary::arbitrary(g))
                    .take(len as usize)
                    .collect(),
            )
        }
    }
    impl Arbitrary for Block {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let content = BlockContents::arbitrary(g);
            let (hash, size) = content.compute_hash_size();
            let mut header = Header::arbitrary(g);
            header.common.block_content_size = size as u32;
            header.common.block_content_hash = hash;
            Block {
                header: header,
                contents: content,
            }
        }
    }
}
