use bittorrent_starter_rust::decoder::{decode_bencoded_dict, decode_bencoded_value};
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
            println!("{}", decoded_value.to_string());
        }
        "info" => {
            // Open the file & read it into a string
            let filename = &args[2];
            let contents_u8: &[u8] = &std::fs::read(filename).unwrap();
            // println!("U8: {:?}", contents_u8);
            // println!("String: {}", contents);
            // Decode the bencoded dict
            let (_, decoded_value) = decode_bencoded_dict(&contents_u8);
            // Convert into a map so we can access the keys
            let decoded_dict = decoded_value.as_object().unwrap();
            // Get the tracker URL and the piece length
            let tracker_url = decoded_dict.get("announce").unwrap().as_str().unwrap();
            let piece_length = decoded_dict
                .get("info")
                .unwrap()
                .get("length")
                .unwrap()
                .as_i64()
                .unwrap();
            // Print the tracker URL and the piece length
            println!("Tracker URL: {}", tracker_url);
            println!("Length: {}", piece_length);
        }
        _ => {
            println!("unknown command: {}", args[1])
        }
    }
}
