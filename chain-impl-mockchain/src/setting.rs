//! define the Blockchain settings
//!

use crate::{block::ConsensusVersion, fee::LinearFee, key::Hash, leadership::bft};
use chain_core::mempack::{read_vec, ReadBuf, ReadError, Readable};
use chain_core::property;
use std::sync::Arc;

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

// FIXME: sign UpdateProposals, add voting, execute updates at an
// epoch boundary.

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
pub struct UpdateProposal {
    pub max_number_of_transactions_per_block: Option<u32>,
    pub bootstrap_key_slots_percentage: Option<u8>,
    pub consensus_version: Option<ConsensusVersion>,
    pub bft_leaders: Option<Vec<bft::LeaderId>>,
    /// update to trigger allowing the creation of accounts without
    /// publishing a certificate
    pub allow_account_creation: Option<bool>,
    /// update the LinearFee settings
    pub linear_fees: Option<LinearFee>,
    /// setting the slot duration (in seconds, max value is 255sec -- 4min)
    pub slot_duration: Option<u8>,
    /// Todo
    pub epoch_stability_depth: Option<u32>,
}

impl UpdateProposal {
    pub fn new() -> Self {
        UpdateProposal {
            max_number_of_transactions_per_block: None,
            bootstrap_key_slots_percentage: None,
            consensus_version: None,
            bft_leaders: None,
            allow_account_creation: None,
            linear_fees: None,
            slot_duration: None,
            epoch_stability_depth: None,
        }
    }
}

#[derive(FromPrimitive)]
enum UpdateTag {
    End = 0,
    MaxNumberOfTransactionsPerBlock = 1,
    BootstrapKeySlotsPercentage = 2,
    ConsensusVersion = 3,
    BftLeaders = 4,
    AllowAccountCreation = 5,
    LinearFee = 6,
    SlotDuration = 7,
    EpochStabilityDepth = 8,
}

impl property::Serialize for UpdateProposal {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(writer);
        if let Some(max_number_of_transactions_per_block) =
            self.max_number_of_transactions_per_block
        {
            codec.put_u16(UpdateTag::MaxNumberOfTransactionsPerBlock as u16)?;
            codec.put_u32(max_number_of_transactions_per_block)?;
        }
        if let Some(bootstrap_key_slots_percentage) = self.bootstrap_key_slots_percentage {
            codec.put_u16(UpdateTag::BootstrapKeySlotsPercentage as u16)?;
            codec.put_u8(bootstrap_key_slots_percentage)?;
        }
        if let Some(consensus_version) = self.consensus_version {
            codec.put_u16(UpdateTag::ConsensusVersion as u16)?;
            codec.put_u16(consensus_version as u16)?;
        }
        if let Some(leaders) = &self.bft_leaders {
            codec.put_u16(UpdateTag::BftLeaders as u16)?;
            codec.put_u8(leaders.len() as u8)?;
            for leader in leaders.iter() {
                leader.serialize(&mut codec)?;
            }
        }
        if let Some(allow_account_creation) = &self.allow_account_creation {
            codec.put_u16(UpdateTag::AllowAccountCreation as u16)?;
            codec.put_u8(if *allow_account_creation { 1 } else { 0 })?;
        }
        if let Some(linear_fees) = &self.linear_fees {
            codec.put_u16(UpdateTag::LinearFee as u16)?;
            codec.put_u64(linear_fees.constant)?;
            codec.put_u64(linear_fees.coefficient)?;
            codec.put_u64(linear_fees.certificate)?;
        }
        if let Some(slot_duration) = self.slot_duration {
            codec.put_u16(UpdateTag::SlotDuration as u16)?;
            codec.put_u8(slot_duration)?;
        }
        if let Some(epoch_stability_depth) = self.epoch_stability_depth {
            codec.put_u16(UpdateTag::EpochStabilityDepth as u16)?;
            codec.put_u32(epoch_stability_depth)?;
        }
        codec.put_u16(UpdateTag::End as u16)?;
        Ok(())
    }
}

impl Readable for UpdateProposal {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let mut update = UpdateProposal::new();
        loop {
            let tag = buf.get_u16()?;
            match UpdateTag::from_u16(tag) {
                Some(UpdateTag::End) => {
                    return Ok(update);
                }
                Some(UpdateTag::MaxNumberOfTransactionsPerBlock) => {
                    update.max_number_of_transactions_per_block = Some(buf.get_u32()?);
                }
                Some(UpdateTag::BootstrapKeySlotsPercentage) => {
                    update.bootstrap_key_slots_percentage = Some(buf.get_u8()?);
                }
                Some(UpdateTag::ConsensusVersion) => {
                    let version_u16 = buf.get_u16()?;
                    let version = ConsensusVersion::from_u16(version_u16).ok_or_else(|| {
                        ReadError::StructureInvalid(format!(
                            "Unrecognized consensus version {}",
                            version_u16
                        ))
                    })?;
                    update.consensus_version = Some(version);
                }
                Some(UpdateTag::BftLeaders) => {
                    let len = buf.get_u8()? as usize;
                    let leaders = read_vec(buf, len)?;
                    update.bft_leaders = Some(leaders);
                }
                Some(UpdateTag::AllowAccountCreation) => {
                    let boolean = buf.get_u8()? != 0;
                    update.allow_account_creation = Some(boolean);
                }
                Some(UpdateTag::LinearFee) => {
                    update.linear_fees = Some(LinearFee {
                        constant: buf.get_u64()?,
                        coefficient: buf.get_u64()?,
                        certificate: buf.get_u64()?,
                    });
                }
                Some(UpdateTag::SlotDuration) => {
                    update.slot_duration = Some(buf.get_u8()?);
                }
                Some(UpdateTag::EpochStabilityDepth) => {
                    update.epoch_stability_depth = Some(buf.get_u32()?);
                }
                None => panic!("Unrecognized update tag {}.", tag),
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub max_number_of_transactions_per_block: u32,
    pub bootstrap_key_slots_percentage: u8, // == d * 100
    pub consensus_version: ConsensusVersion,
    pub bft_leaders: Arc<Vec<bft::LeaderId>>,
    /// allow for the creation of accounts without the certificate
    pub allow_account_creation: bool,
    pub linear_fees: Arc<LinearFee>,
    pub slot_duration: u8,
    pub epoch_stability_depth: usize,
}

pub const SLOTS_PERCENTAGE_RANGE: u8 = 100;

impl Settings {
    pub fn new() -> Self {
        Self {
            max_number_of_transactions_per_block: 100,
            bootstrap_key_slots_percentage: SLOTS_PERCENTAGE_RANGE,
            consensus_version: ConsensusVersion::Bft,
            bft_leaders: Arc::new(Vec::new()),
            allow_account_creation: false,
            linear_fees: Arc::new(LinearFee::new(0, 0, 0)),
            slot_duration: 10,         // 10 sec
            epoch_stability_depth: 10, // num of block
        }
    }

    pub fn allow_account_creation(&self) -> bool {
        self.allow_account_creation
    }

    pub fn linear_fees(&self) -> LinearFee {
        *self.linear_fees
    }

    pub fn apply(&self, update: &UpdateProposal) -> Self {
        let mut new_state = self.clone();
        if let Some(max_number_of_transactions_per_block) =
            update.max_number_of_transactions_per_block
        {
            new_state.max_number_of_transactions_per_block = max_number_of_transactions_per_block;
        }
        if let Some(bootstrap_key_slots_percentage) = update.bootstrap_key_slots_percentage {
            new_state.bootstrap_key_slots_percentage = bootstrap_key_slots_percentage;
        }
        if let Some(consensus_version) = update.consensus_version {
            new_state.consensus_version = consensus_version;
        }
        if let Some(ref leaders) = update.bft_leaders {
            new_state.bft_leaders = Arc::new(leaders.clone());
        }
        if let Some(allow_account_creation) = update.allow_account_creation {
            new_state.allow_account_creation = allow_account_creation;
        }
        if let Some(linear_fees) = update.linear_fees {
            new_state.linear_fees = Arc::new(linear_fees);
        }
        if let Some(slot_duration) = update.slot_duration {
            new_state.slot_duration = slot_duration;
        }
        if let Some(epoch_stability_depth) = update.epoch_stability_depth {
            new_state.epoch_stability_depth = epoch_stability_depth as usize;
        }
        new_state
    }
}

#[derive(Debug)]
pub enum Error {
    InvalidCurrentBlockId(Hash, Hash),
    UpdateIsInvalid,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::InvalidCurrentBlockId(current_one, update_one) => {
                write!(f, "Cannot apply Setting Update. Update needs to be applied to from block {:?} but received {:?}", update_one, current_one)
            }
            Error::UpdateIsInvalid => write!(
                f,
                "Update does not apply to current state"
            ),
        }
    }
}
impl std::error::Error for Error {}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for UpdateProposal {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            UpdateProposal {
                max_number_of_transactions_per_block: Arbitrary::arbitrary(g),
                bootstrap_key_slots_percentage: Arbitrary::arbitrary(g),
                consensus_version: Arbitrary::arbitrary(g),
                bft_leaders: None,
                allow_account_creation: None,
                linear_fees: None,
                slot_duration: Arbitrary::arbitrary(g),
                epoch_stability_depth: Arbitrary::arbitrary(g),
            }
        }
    }
}
