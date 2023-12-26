use std::collections::BTreeMap;

use hex::ToHex;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

use crate::decoder::{Bencodeable, BencodedString, BencodedValue};

#[derive(Debug, Deserialize)]
pub struct MetainfoFile {
    pub announce: String,
    pub info: Info,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Info {
    pub length: i64,
    pub name: String,
    #[serde(rename = "piece length")]
    pub piece_length: i64,
    pub pieces: Vec<u8>,
}

impl From<Info> for BencodedValue {
    fn from(value: Info) -> Self {
        let mut out = BTreeMap::new();
        let name_bytes: Vec<u8> = value.name.into_bytes();
        out.insert(
            BencodedString(b"length".to_vec()),
            BencodedValue::Integer(value.length),
        );
        out.insert(
            BencodedString(b"name".to_vec()),
            BencodedValue::String(name_bytes.into()),
        );
        out.insert(
            BencodedString(b"piece length".to_vec()),
            BencodedValue::Integer(value.piece_length),
        );
        out.insert(
            BencodedString(b"pieces".to_vec()),
            BencodedValue::String(value.pieces.into()),
        );
        BencodedValue::Dict(out)
    }
}

impl Info {
    pub fn info_hash(&self) -> String {
        let name_bytes = self.name.clone().into_bytes();
        let hashmap = BTreeMap::from([
            (
                BencodedString(b"length".to_vec()),
                BencodedValue::Integer(self.length),
            ),
            (
                BencodedString(b"name".to_vec()),
                BencodedValue::String(name_bytes.into()),
            ),
            (
                BencodedString(b"piece length".to_vec()),
                BencodedValue::Integer(self.piece_length),
            ),
            (
                BencodedString(b"pieces".to_vec()),
                BencodedValue::String(self.pieces.clone().into()),
            ),
        ]);
        let bencode = BencodedValue::Dict(hashmap.into());
        println!("Bencode: {:?}", bencode);

        let mut hasher = Sha1::new();
        hasher.update(bencode.bencode());
        hasher.finalize().encode_hex::<String>()
    }
}
