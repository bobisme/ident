use ::serde::de::Visitor;

use crate::{decode, encode_array, Id};

#[cfg(feature = "serde")]
impl serde::Serialize for Id {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            let arr = encode_array(self.0);
            let s = unsafe { std::str::from_utf8_unchecked(&arr[..]) };
            serializer.serialize_str(s)
        } else {
            serializer.serialize_u128(self.0)
        }
    }
}

#[cfg(feature = "serde")]
struct IdVisitor;

#[cfg(feature = "serde")]
impl<'a> Visitor<'a> for IdVisitor {
    type Value = Id;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("&str or u128")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Id::from_u128(
            decode(v.as_bytes()).map_err(|e| serde::de::Error::custom(e))?,
        ))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Id::from_u128(
            decode(v).map_err(|e| serde::de::Error::custom(e))?,
        ))
    }

    fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Id::from_u128(v))
    }
}

impl<'de> serde::Deserialize<'de> for Id {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            deserializer.deserialize_str(IdVisitor)
        } else {
            deserializer.deserialize_u128(IdVisitor)
        }
    }
}

#[cfg(test)]
#[cfg(feature = "serde")]
mod test_serde {
    use super::*;
    use assert2::assert;

    #[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    struct TestStruct {
        a: u8,
        id: Id,
        b: u8,
    }

    #[test]
    fn serializes_to_string_if_human_read_fmt() {
        let x = TestStruct {
            id: Id::from_u128(32),
            a: 1,
            b: 2,
        };
        let result = serde_json::to_string(&x).unwrap();
        assert!(result == "{\"a\":1,\"id\":\"000000-00000000-000010\",\"b\":2}");
    }

    #[test]
    fn serializes_to_u128_if_not_human_read_fmt() {
        let x = TestStruct {
            id: Id::from_u128(1 << 32),
            a: 1,
            b: 2,
        };
        let mut buf = [0u8; 128];
        let result = postcard::to_slice(&x, &mut buf).unwrap();
        assert!(result == [1, 128, 128, 128, 128, 16, 2]);
        let result: TestStruct = postcard::from_bytes(result).unwrap();
        assert!(result.id == Id::from_u128(1 << 32));
    }

    #[test]
    fn deserializes_from_u128_if_not_human_read_fmt() {
        let bytes = [1, 128, 128, 128, 128, 16, 2];
        let result: TestStruct = postcard::from_bytes(&bytes[..]).unwrap();
        assert!(result.id == Id::from_u128(1 << 32));
    }
}
