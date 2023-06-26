use hex::FromHexError;

#[cfg(feature = "std")]
use thiserror::Error;

const HEX_ENCODING_PREFIX: &str = "0x";

#[derive(Debug)]
#[cfg(feature = "std")]
#[derive(Error)]
pub enum HexError {
    #[error("{0}")]
    Hex(#[from] FromHexError),
    #[error("missing prefix `{HEX_ENCODING_PREFIX}` when deserializing hex data")]
    MissingPrefix,
}

fn try_bytes_from_hex_str(s: &str) -> Result<Vec<u8>, HexError> {
    let target = s.strip_prefix(HEX_ENCODING_PREFIX).ok_or(HexError::MissingPrefix)?;
    let data = hex::decode(target)?;
    Ok(data)
}

pub mod as_hex {
    use super::*;
    use serde::Deserialize;

    pub fn serialize<S, T: AsRef<[u8]>>(data: T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let encoding = hex::encode(data.as_ref());
        let output = format!("{HEX_ENCODING_PREFIX}{encoding}");
        serializer.collect_str(&output)
    }

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: serde::Deserializer<'de>,
        T: for<'a> TryFrom<&'a [u8]>,
    {
        let s = <String>::deserialize(deserializer)?;

        let data = try_bytes_from_hex_str(&s).map_err(serde::de::Error::custom)?;

        let inner = T::try_from(&data)
            .map_err(|_| serde::de::Error::custom("could not parse instance from byte data"))?;
        Ok(inner)
    }
}

#[cfg(test)]
mod test {
    use crate::prelude::*;

    #[derive(
        PartialEq, Eq, Debug, Default, Clone, SimpleSerialize, serde::Serialize, serde::Deserialize,
    )]
    struct FixedTestStruct {
        a: u8,
        b: u64,
        c: u32,
    }

    #[derive(
        PartialEq, Eq, Debug, Default, Clone, SimpleSerialize, serde::Serialize, serde::Deserialize,
    )]
    struct VarTestStruct {
        a: u16,
        b: List<u16, 1024>,
        c: u8,
    }

    #[derive(
        PartialEq, Eq, Debug, Default, SimpleSerialize, serde::Serialize, serde::Deserialize,
    )]
    struct ComplexTestStruct {
        a: u16,
        b: List<u16, 128>,
        c: u8,
        d: List<u8, 256>,
        e: VarTestStruct,
        f: Vector<FixedTestStruct, 4>,
        g: Vector<VarTestStruct, 2>,
        h: Bitvector<9>,
        i: Bitlist<32>,
        j: U256,
    }

    #[test]
    fn test_roundtrip() {
        let value = ComplexTestStruct {
            a: 51972,
            b: List::<u16, 128>::try_from(vec![48645]).unwrap(),
            c: 46,
            d: List::<u8, 256>::try_from(vec![105]).unwrap(),
            e: VarTestStruct {
                a: 1558,
                b: List::<u16, 1024>::try_from(vec![39947]).unwrap(),
                c: 65,
            },
            f: Vector::<FixedTestStruct, 4>::try_from(vec![
                FixedTestStruct { a: 70, b: 905948488145107787, c: 2675781419 },
                FixedTestStruct { a: 3, b: 12539792087931462647, c: 4719259 },
                FixedTestStruct { a: 73, b: 13544872847030609257, c: 2819826618 },
                FixedTestStruct { a: 159, b: 16328658841145598323, c: 2375225558 },
            ])
            .unwrap(),
            g: Vector::<VarTestStruct, 2>::try_from(vec![
                VarTestStruct {
                    a: 30336,
                    b: List::<u16, 1024>::try_from(vec![30909]).unwrap(),
                    c: 240,
                },
                VarTestStruct {
                    a: 64263,
                    b: List::<u16, 1024>::try_from(vec![38121]).unwrap(),
                    c: 100,
                },
            ])
            .unwrap(),
            h: Bitvector::from_iter([true, false, false, true, false, false, false, true, true]),
            i: Bitlist::from_iter([true, false, true, true]),
            j: U256::from_bytes_le([12u8; 32]),
        };
        let json_repr = serde_json::to_value(&value).unwrap();
        let roundtrip_value: ComplexTestStruct = serde_json::from_value(json_repr).unwrap();
        assert_eq!(value, roundtrip_value);
    }
}
