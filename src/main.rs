use serde_json;
use std::env;

// Available if you need it!
// use serde_bencode

fn decode_bencoded_string(encoded_value: &str) -> (usize, serde_json::Value) {
    // Example: "5:hello" -> "hello"
    let colon_index = encoded_value.find(':').unwrap();
    let number_string = &encoded_value[..colon_index];
    let number = number_string.parse::<i64>().unwrap();
    let string = &encoded_value[colon_index + 1..colon_index + 1 + number as usize];
    let ending_index = colon_index + 1 + number as usize;
    return (ending_index, serde_json::Value::String(string.to_string()));
}

fn decode_bencoded_integer(encoded_value: &str) -> (usize, serde_json::Value) {
    // Example: "i3e" -> 3
    // Get number string from start until 'e'
    let number_string = &encoded_value
        .chars()
        .skip(1)
        .take_while(|c| *c != 'e')
        .collect::<String>();
    let number = number_string.parse::<i64>().unwrap();
    let ending_index = 2 + number_string.len();
    return (
        ending_index,
        serde_json::Value::Number(serde_json::Number::from(number)),
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
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        // You can use print statements as follows for debugging, they'll be visible when running tests.
        // println!("Logs from your program will appear here!");

        // Uncomment this block to pass the first stage
        let encoded_value = &args[2];
        let (_, decoded_value) = decode_bencoded_value(encoded_value);
        println!("{}", decoded_value.to_string());
    } else {
        println!("unknown command: {}", args[1])
    }
}
