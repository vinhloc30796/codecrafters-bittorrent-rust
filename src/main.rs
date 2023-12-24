use anyhow::Context;
use serde_json;
use std::env;

// Available if you need it!
// use serde_bencode

// Should take in either a string or a byte array
// Example: "5:hello" -> "hello"
fn decode_bencoded_string<T: AsRef<[u8]>>(encoded_value: T) -> (usize, serde_json::Value) {
    let encoded_value = encoded_value.as_ref();
    let colon_index = encoded_value
        .iter()
        .position(|&c| c == b':')
        // return if found, panic with message if not
        .with_context(|| format!("Could not find ':' in {:?}", encoded_value))
        .unwrap();
    let length_part = &encoded_value[..colon_index];
    let length = length_part
        .iter()
        .map(|&c| (c - b'0') as usize)
        .fold(0, |acc, x| acc * 10 + x);
    let text_part = &encoded_value[colon_index + 1..colon_index + 1 + length as usize];
    let text = String::from_utf8_lossy(text_part);
    let ending_index = colon_index + 1 + length as usize;
    return (ending_index, serde_json::Value::String(text.to_string()));
}

// Example: "i3e" -> 3
// Example 2: "i-3e" -> -3
fn decode_bencoded_integer<T: AsRef<[u8]>>(encoded_value: T) -> (usize, serde_json::Value) {
    // Get number string from start until 'e'
    let encoded_value = encoded_value.as_ref();
    let mut ending_index = 2;
    let mut number = 0;
    let mut mult = 1;
    for (_, &c) in encoded_value[1..].iter().enumerate() {
        match c {
            b'e' => break,
            b'-' => {
                ending_index += 1;
                mult = -1;
            }
            b'0'..=b'9' => {
                number = number * 10 + (c - b'0') as i64;
                ending_index += 1;
            }
            _ => panic!("Invalid bencoded integer: {:?}", encoded_value),
        }
    }
    return (
        ending_index,
        serde_json::Value::Number(serde_json::Number::from(number * mult)),
    );
}

// Example: "l5:helloi3ee" -> ["hello", 3]
// Example 2: "l4:spam4:eggse" -> ["spam", "eggs"]
// Example 3: "l4:spaml1:a1:bee" -> ["spam", ["a", "b"]]
fn decode_bencoded_list(encoded_value: &str) -> (usize, serde_json::Value) {
    // Get string from start until 'e'
    let mut list = Vec::new();
    let mut encoded_value = &encoded_value[1..];
    let mut ending_index = 1;
    while encoded_value.chars().next().unwrap() != 'e' {
        let (child_index, decoded_value) = decode_bencoded_value(encoded_value);
        list.push(decoded_value);
        encoded_value = &encoded_value[child_index..];
        ending_index += child_index;
    }
    ending_index += 1;
    return (ending_index, serde_json::Value::Array(list));
}

// Example: "d3:cow3:moo4:spam4:eggse" -> {"cow": "moo", "spam": "eggs"}
// Example 2: "d4:spaml1:a1:bee" -> {"spam": ["a", "b"]}
// Example 3: "d4:foodd1:a3:baree" -> {"food": {"a": "bar"}}
// -> {"publisher": "bob", "publisher-webpage": "www.example.com", "publisher.location": "home"}
fn decode_bencoded_dict(encoded_value: &str) -> (usize, serde_json::Value) {
    // Get string from start until 'e'
    let mut dict = serde_json::Map::new();
    let mut encoded_value = &encoded_value[1..];
    let mut ending_index = 1;
    while encoded_value.chars().next().unwrap() != 'e' {
        let (key_index, key) = decode_bencoded_string(encoded_value);
        encoded_value = &encoded_value[key_index..];
        ending_index += key_index;
        let (value_index, value) = decode_bencoded_value(encoded_value);
        encoded_value = &encoded_value[value_index..];
        ending_index += value_index;
        dict.insert(key.as_str().unwrap().to_string(), value);
    }
    ending_index += 1;
    return (ending_index, serde_json::Value::Object(dict));
}

#[allow(dead_code)]
fn decode_bencoded_value(encoded_value: &str) -> (usize, serde_json::Value) {
    // If encoded_value starts with a digit, it's a number
    let first_char = encoded_value.chars().next().unwrap();
    match first_char {
        '0'..='9' => return decode_bencoded_string(encoded_value),
        'i' => return decode_bencoded_integer(encoded_value),
        'l' => return decode_bencoded_list(encoded_value),
        'd' => return decode_bencoded_dict(encoded_value),
        _ => panic!("Unhandled bencoded value: {}", encoded_value),
    }
}

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
            println!("Original contents: {:?}", contents_u8);
            let contents = String::from_utf8_lossy(contents_u8).to_string();
            println!("Parsed contents: {:?}", contents);
            // Decode the bencoded dict
            let (_, decoded_value) = decode_bencoded_dict(&contents);
            // Convert into a map so we can access the keys
            let decoded_dict = decoded_value.as_object().unwrap();
            // Get the tracker URL and the piece length
            let tracker_url = decoded_dict.get("announce").unwrap().as_str().unwrap();
            let piece_length = decoded_dict
                .get("info")
                .unwrap()
                .get("piece length")
                .unwrap()
                .as_u64()
                .unwrap();
            // Print the tracker URL and the piece length
            println!("Tracker URL: {}", tracker_url);
            println!("Piece length: {}", piece_length);
        }
        _ => {
            println!("unknown command: {}", args[1])
        }
    }
}
