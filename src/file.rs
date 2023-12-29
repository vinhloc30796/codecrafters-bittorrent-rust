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
    pub fn info_hash(&self) -> [u8; 20] {
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
        // println!("Bencode: {:?}", bencode);

        let mut hasher = Sha1::new();
        hasher.update(bencode.bencode());
        hasher.finalize().into()
    }

    pub fn pieces(&self) -> Vec<[u8; 20]> {
        return self
            .pieces
            .chunks(20)
            .map(|chunk| {
                let mut array = [0; 20];
                array.copy_from_slice(chunk);
                array
            })
            .collect();
    }

    pub fn piece_hash(&self) -> Vec<String> {
        // Pieces is a byte string, so we need to split it into 20 byte chunks
        let piece_chunks = self.pieces.chunks(20);

        // Return
        piece_chunks
            .map(|chunk| chunk.encode_hex::<String>())
            .collect::<Vec<String>>()
    }
}

impl MetainfoFile {
    // Can take either PathBuf or &str
    pub fn read_from_file<T: AsRef<std::path::Path>>(filename: T) -> std::io::Result<Self> {
        // Open the file & read it into a string
        let contents_u8: &[u8] = &std::fs::read(filename).unwrap();
        // println!("U8: {:?}", contents_u8);
        // println!("String: {}", contents);

        // Decode the bencoded dict
        let decoded_value = BencodedValue::from(contents_u8);
        let json_value = serde_json::Value::from(decoded_value);
        match serde_json::from_value(json_value) {
            Ok(metainfo) => Ok(metainfo),
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
        }
    }
}
