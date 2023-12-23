use serde_json;
use std::env;

// Available if you need it!
// use serde_bencode

fn decode_bencoded_string(encoded_value: &str) -> serde_json::Value {
    // Example: "5:hello" -> "hello"
    let colon_index = encoded_value.find(':').unwrap();
    let number_string = &encoded_value[..colon_index];
    let number = number_string.parse::<i64>().unwrap();
    let string = &encoded_value[colon_index + 1..colon_index + 1 + number as usize];
    return serde_json::Value::String(string.to_string());
}

fn decode_bencoded_integer(encoded_value: &str) -> serde_json::Value {
    // Example: "i3e" -> 3
    let end_index = encoded_value.find('e').unwrap();
    let number_string = &encoded_value[1..end_index];
    let number = number_string.parse::<i64>().unwrap();
    return serde_json::Value::Number(serde_json::Number::from(number));
}

#[allow(dead_code)]
fn decode_bencoded_value(encoded_value: &str) -> serde_json::Value {
    // If encoded_value starts with a digit, it's a number
    let first_char = encoded_value.chars().next().unwrap();
    match first_char {
        '0'..='9' => return decode_bencoded_string(encoded_value),
        'i' => return decode_bencoded_integer(encoded_value),
        _ => panic!("Unhandled bencoded value: {}", encoded_value),
    }
}

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        // You can use print statements as follows for debugging, they'll be visible when running tests.
        // println!("Logs from your program will appear here!");

        // Uncomment this block to pass the first stage
        let encoded_value = &args[2];
        let decoded_value = decode_bencoded_value(encoded_value);
        println!("{}", decoded_value.to_string());
    } else {
        println!("unknown command: {}", args[1])
    }
}
