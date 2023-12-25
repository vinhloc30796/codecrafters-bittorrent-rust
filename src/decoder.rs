use anyhow::Context;
use serde_json;

// Should take in either a string or a byte array
// Example: "5:hello" -> "hello"
pub fn decode_bencoded_string<T: AsRef<[u8]>>(encoded_value: T) -> (usize, serde_json::Value) {
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
pub fn decode_bencoded_integer<T: AsRef<[u8]>>(encoded_value: T) -> (usize, serde_json::Value) {
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
pub fn decode_bencoded_list<T: AsRef<[u8]>>(encoded_value: T) -> (usize, serde_json::Value) {
    // Get string from start until 'e'
    let encoded_value = encoded_value.as_ref();
    let mut encoded_value = &encoded_value[1..];
    let mut list = Vec::new();
    let mut ending_index = 1;
    loop {
        match encoded_value.iter().next().unwrap() {
            b'e' => break,
            _ => {
                let (child_index, decoded_value) = decode_bencoded_value(encoded_value);
                list.push(decoded_value);
                encoded_value = &encoded_value[child_index..];
                ending_index += child_index;
            }
        }
    }
    ending_index += 1;
    return (ending_index, serde_json::Value::Array(list));
}

// Example: "d3:cow3:moo4:spam4:eggse" -> {"cow": "moo", "spam": "eggs"}
// Example 2: "d4:spaml1:a1:bee" -> {"spam": ["a", "b"]}
// Example 3: "d4:foodd1:a3:baree" -> {"food": {"a": "bar"}}
// Example 4: "d4:foodd1:a3:bare5:drinkd1:b3:bazee" -> {"food": {"a": "bar"}, "drink": {"b": "baz"}}
// -> {"publisher": "bob", "publisher-webpage": "www.example.com", "publisher.location": "home"}
pub fn decode_bencoded_dict<T: AsRef<[u8]>>(encoded_value: T) -> (usize, serde_json::Value) {
    // Get string from start until 'e'
    let encoded_value = encoded_value.as_ref();
    let mut encoded_value = &encoded_value[1..];
    let mut ending_index = 1;
    let mut dict = serde_json::Map::new();
    loop {
        match encoded_value.iter().next().unwrap() {
            b'e' => break,
            _ => {
                let (key_index, key) = decode_bencoded_string(encoded_value);
                encoded_value = &encoded_value[key_index..];
                ending_index += key_index;
                let (value_index, value) = decode_bencoded_value(encoded_value);
                encoded_value = &encoded_value[value_index..];
                ending_index += value_index;
                dict.insert(key.as_str().unwrap().to_string(), value);
            }
        }
    }
    ending_index += 1;
    return (ending_index, serde_json::Value::Object(dict));
}

pub fn decode_bencoded_value<T: AsRef<[u8]> + std::fmt::Debug>(
    encoded_value: T,
) -> (usize, serde_json::Value) {
    // If encoded_value starts with a digit, it's a number
    let first_char = encoded_value.as_ref()[0] as char;
    match first_char {
        '0'..='9' => return decode_bencoded_string(encoded_value),
        'i' => return decode_bencoded_integer(encoded_value),
        'l' => return decode_bencoded_list(encoded_value),
        'd' => return decode_bencoded_dict(encoded_value),
        _ => panic!("Unhandled bencoded value: {:?}", encoded_value),
    }
}
