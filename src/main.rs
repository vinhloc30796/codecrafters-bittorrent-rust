use bittorrent_starter_rust::decoder::{
    decode_bencoded_dict,
    decode_bencoded_value,
    // Bencodeable, BencodedValue,
};
use bittorrent_starter_rust::dot_torrent::{MetainfoFile, Info};
// use sha1::{Digest, Sha1};
// use hex::ToHex;
use std::env;

// Available if you need it!
// use serde_bencode;

// Usage: your_bittorrent.sh decode "<encoded_value>"
// Usage: your_bittorrent.sh info "<torrent_file>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    // println!("Logs from your program will appear here!");

    match command as &str {
        "decode" => {
            let encoded_value = &args[2];
            let (_, decoded_value) = decode_bencoded_value(encoded_value);
            let json_value = serde_json::Value::from(decoded_value);
            println!("{}", json_value);
        }
        "info" => {
            // Open the file & read it into a string
            let filename = &args[2];
            let contents_u8: &[u8] = &std::fs::read(filename).unwrap();
            // println!("U8: {:?}", contents_u8);
            // println!("String: {}", contents);

            // Decode the bencoded dict
            let (_, decoded_value) = decode_bencoded_dict(&contents_u8);
            let json_value = serde_json::Value::from(decoded_value);

            let metainfo: MetainfoFile = serde_json::from_value(json_value).unwrap();
            let info: Info = metainfo.info;
            println!("Tracker URL: {}", metainfo.announce);
            println!("Length: {}", info.length);

            // Hash the info dict
            println!("Info Hash: {}", info.info_hash());
            println!("Piece Length: {}", info.piece_length);
            let piece_hashes: Vec<String> = info.piece_hash();
            // Print piece hashes on new line
            println!("Pieces Hashes:\n{}", piece_hashes.join("\n"));
        }
        _ => {
            println!("unknown command: {}", args[1])
        }
    }
}
