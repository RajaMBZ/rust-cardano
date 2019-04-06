use crate::config::ConfigParam;
use chain_core::mempack::{ReadBuf, ReadError, Readable};
use chain_core::property;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "generic-serialization",
    derive(Serialize, Deserialize),
    serde(transparent)
)]
pub struct InitialEnts(Vec<ConfigParam>);

impl InitialEnts {
    pub fn new() -> Self {
        InitialEnts(Vec::new())
    }

    pub fn push(&mut self, config: ConfigParam) {
        self.0.push(config)
    }

    pub fn iter(&self) -> std::slice::Iter<ConfigParam> {
        self.0.iter()
    }
}

impl property::Serialize for InitialEnts {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, mut writer: W) -> Result<(), Self::Error> {
        for config in &self.0 {
            config.serialize(&mut writer)?
        }
        Ok(())
    }
}

impl Readable for InitialEnts {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let mut configs = vec![];
        while !buf.is_end() {
            configs.push(ConfigParam::read(buf)?);
        }
        Ok(InitialEnts(configs))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen, TestResult};

    quickcheck! {
        fn initial_ents_serialization_bijection(b: InitialEnts) -> TestResult {
            property::testing::serialization_bijection_r(b)
        }
    }

    impl Arbitrary for InitialEnts {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let size = u8::arbitrary(g) as usize;
            InitialEnts(
                std::iter::repeat_with(|| ConfigParam::arbitrary(g))
                    .take(size)
                    .collect(),
            )
        }
    }
}
