use std::{collections::BTreeMap, fmt};

use anyhow::Context;
use serde_json::{self};

#[derive(Debug, PartialEq)]
pub enum BencodedValue {
    String(BencodedString),
    Integer(i64),
    List(Vec<BencodedValue>),
    Dict(BTreeMap<BencodedString, BencodedValue>),
}

#[derive(Debug, PartialEq, Hash, Eq, PartialOrd, Ord, Clone)]
pub struct BencodedString(pub Vec<u8>);

// Impl Length for BencodedString
impl BencodedString {
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

// Convert from a byte array to a BencodedString
impl From<&[u8]> for BencodedString {
    fn from(value: &[u8]) -> Self {
        let vec = Vec::from(value);
        BencodedString(vec)
    }
}

impl From<Vec<u8>> for BencodedString {
    fn from(value: Vec<u8>) -> Self {
        BencodedString(value)
    }
}

// Convert from a String to a BencodedString
impl From<String> for BencodedString {
    fn from(value: String) -> Self {
        let string: Vec<u8> = value.into();
        BencodedString(string)
    }
}

// Convert from a BencodedString to a String
impl From<&BencodedString> for String {
    fn from(value: &BencodedString) -> Self {
        return String::from_utf8_lossy(&value.0).to_string();
    }
}

// Convert from BencodedString to a serde_json::Value
impl From<&BencodedString> for serde_json::Value {
    fn from(value: &BencodedString) -> Self {
        // If is_ascii then keep,
        // else convert to array of number
        match value.0.is_ascii() {
            true => serde_json::Value::String(String::from(value)),
            false => serde_json::Value::Array(
                value
                    .0
                    .iter()
                    .map(|&c| serde_json::Value::Number(c.into()))
                    .collect(),
            ),
        }
    }
}

impl fmt::Display for BencodedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", String::from(self))
    }
}

// Convert from a BenodedString to a byte array
impl From<&BencodedString> for Vec<u8> {
    fn from(value: &BencodedString) -> Self {
        return value.0.clone();
    }
}

// Convert from a byte array to a BencodedValue
impl From<&[u8]> for BencodedValue {
    fn from(value: &[u8]) -> Self {
        let (_, out) = decode_bencoded_value(value);
        out
    }
}

impl From<BencodedValue> for serde_json::Value {
    fn from(value: BencodedValue) -> Self {
        match value {
            BencodedValue::String(s) => serde_json::Value::from(&s),
            BencodedValue::Integer(i) => i.into(),
            BencodedValue::List(l) => {
                let mut out = Vec::new();
                for item in l {
                    out.push(serde_json::Value::from(item));
                }
                serde_json::Value::Array(out)
            }
            BencodedValue::Dict(d) => {
                let mut out = serde_json::Map::new();
                for (key, value) in d {
                    out.insert(String::from(&key), value.into());
                }
                serde_json::Value::Object(out)
            }
        }
    }
}

impl fmt::Display for BencodedValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BencodedValue::String(s) => {
                write!(f, "{}", s)
            }
            BencodedValue::Integer(i) => {
                write!(f, "{}", i)
            }
            BencodedValue::List(l) => {
                // Format the list elements and join them with ", "
                let elements: Vec<String> = l.iter().map(|e| format!("{}", e)).collect();
                write!(f, "[{}]", elements.join(", "))
            }
            BencodedValue::Dict(d) => {
                // Format the dictionary elements and join them with ", "
                let elements: Vec<String> =
                    d.iter().map(|(k, v)| format!("{}: {}", k, v)).collect();
                write!(f, "{{{}}}", elements.join(", "))
            }
        }
    }
}

// Bencodeable
pub trait Bencodeable {
    fn bencode(&self) -> Vec<u8>;
}

impl Bencodeable for BencodedValue {
    fn bencode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        match self {
            BencodedValue::String(s) => {
                let length = s.len();
                let vec: Vec<u8> = s.into();
                out.extend_from_slice(length.to_string().as_bytes());
                out.push(b':');
                out.extend(vec);
            }

            BencodedValue::Integer(i) => {
                out.push(b'i');
                out.extend_from_slice(i.to_string().as_bytes());
                out.push(b'e');
            }
            BencodedValue::List(l) => {
                out.push(b'l');
                for item in l {
                    out.extend(item.bencode());
                }
                out.push(b'e');
            }
            BencodedValue::Dict(d) => {
                out.push(b'd');
                for (key, value) in d {
                    let bencode_key = BencodedValue::String(key.clone());
                    out.extend(bencode_key.bencode());
                    out.extend(value.bencode());
                }
                out.push(b'e');
            }
        }
        return out;
    }
}

// Should take in either a string or a byte array
// Example: "5:hello" -> "hello"
pub fn decode_bencoded_string<T: AsRef<[u8]>>(encoded_value: T) -> (usize, BencodedValue) {
    let encoded_value = encoded_value.as_ref();
    let colon_index = encoded_value
        .iter()
        .position(|&c| c == b':')
        // return if found, panic with message if not
        .with_context(|| {
            format!(
                "Could not find ':' in {:?}, in string {:?}",
                encoded_value,
                String::from_utf8_lossy(encoded_value)
            )
        })
        .unwrap();
    let length_part = &encoded_value[..colon_index];
    let length = String::from_utf8_lossy(length_part)
        .parse::<usize>()
        .with_context(|| {
            format!(
                "Could not parse length: {:?} (str {}) -- input: {:?}",
                length_part,
                String::from_utf8_lossy(length_part),
                encoded_value
            )
        })
        .unwrap();
    let text_part = &encoded_value[colon_index + 1..colon_index + 1 + length as usize];
    let bencode_text = BencodedString(text_part.to_vec());
    let ending_index = colon_index + 1 + length as usize;
    return (ending_index, BencodedValue::String(bencode_text));
}

// Example: "i3e" -> 3
// Example 2: "i-3e" -> -3
pub fn decode_bencoded_integer<T: AsRef<[u8]>>(encoded_value: T) -> (usize, BencodedValue) {
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
    return (ending_index, BencodedValue::Integer(number * mult as i64));
}

// Example: "l5:helloi3ee" -> ["hello", 3]
// Example 2: "l4:spam4:eggse" -> ["spam", "eggs"]
// Example 3: "l4:spaml1:a1:bee" -> ["spam", ["a", "b"]]
pub fn decode_bencoded_list<T: AsRef<[u8]>>(encoded_value: T) -> (usize, BencodedValue) {
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
    return (ending_index, BencodedValue::List(list));
}

// Example: "d3:cow3:moo4:spam4:eggse" -> {"cow": "moo", "spam": "eggs"}
// Example 2: "d4:spaml1:a1:bee" -> {"spam": ["a", "b"]}
// Example 3: "d4:foodd1:a3:baree" -> {"food": {"a": "bar"}}
// Example 4: "d4:foodd1:a3:bare5:drinkd1:b3:bazee" -> {"food": {"a": "bar"}, "drink": {"b": "baz"}}
// -> {"publisher": "bob", "publisher-webpage": "www.example.com", "publisher.location": "home"}
pub fn decode_bencoded_dict<T: AsRef<[u8]>>(encoded_value: T) -> (usize, BencodedValue) {
    // Get string from start until 'e'
    let encoded_value = encoded_value.as_ref();
    let mut encoded_value = &encoded_value[1..];
    let mut ending_index = 1;
    let mut dict: BTreeMap<BencodedString, BencodedValue> = BTreeMap::new();
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
                let key = match key {
                    BencodedValue::String(s) => s,
                    _ => panic!("Invalid key: {:?}", key),
                };
                dict.insert(key, value);
            }
        }
    }
    ending_index += 1;
    return (ending_index, BencodedValue::Dict(dict));
}

pub fn decode_bencoded_value<T: AsRef<[u8]> + std::fmt::Debug>(
    encoded_value: T,
) -> (usize, BencodedValue) {
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

#[cfg(test)]
mod tests {
    use std::vec;

    use super::*;

    #[test]
    fn test_decode_bencoded_string() {
        let (index, value) = decode_bencoded_string("5:hello".as_bytes());
        assert_eq!(index, 7);
        assert_eq!(value, BencodedValue::String(b"hello".to_vec().into()));
    }

    #[test]
    fn test_decode_bencoded_nonutf8_string() {
        // First
        let (index, value) = decode_bencoded_string(b"4:\x80\x81\x82\x83");
        assert_eq!(index, 6);
        assert_eq!(
            value,
            BencodedValue::String(vec![0x80, 0x81, 0x82, 0x83].into())
        );

        // Second
        let mut input: Vec<u8> = Vec::new();
        let byte_vec = &[
            0xBF, 0xBD, 0x01, 0xEF, 0xBF, 0xBD, 0x3E, 0x55, 0x14, 0xEF, 0xBF, 0xBD, 0x25, 0x38,
        ];
        // Push in "14"
        input.extend_from_slice(b"14:");
        input.extend_from_slice(byte_vec);

        let (index, value) = decode_bencoded_string(input);
        assert_eq!(index, 17);
        assert_eq!(
            value,
            BencodedValue::String(BencodedString(byte_vec.into()))
        );

        // Third
        let mut input: Vec<u8> = Vec::new();
        let byte_vec = [
            0xEF, 0xBF, 0xBD, 0xEF, 0xBF, 0xBD, 0x21, 0x4D, 0xEF, 0xBF, 0xBD, 0xEF, 0xBF, 0xBD,
            0x3E, 0x52, 0x59, 0xEF,
        ];
        // Push in "18"
        input.extend_from_slice(b"18:");
        input.extend_from_slice(&byte_vec);

        let (index, value) = decode_bencoded_string(input);
        assert_eq!(index, 21);
        assert_eq!(
            value,
            BencodedValue::String(BencodedString(byte_vec.into()))
        );
    }

    #[test]
    fn test_decode_bencoded_integer() {
        let (index, value) = decode_bencoded_integer("i3e".as_bytes());
        assert_eq!(index, 3);
        assert_eq!(value, BencodedValue::Integer(3));

        let (index, value) = decode_bencoded_integer("i-3e".as_bytes());
        assert_eq!(index, 4);
        assert_eq!(value, BencodedValue::Integer(-3));
    }

    #[test]
    fn test_decode_bencoded_list() {
        let (index, value) = decode_bencoded_list("l5:helloi3ee".as_bytes());
        assert_eq!(index, 12);
        assert_eq!(
            value,
            BencodedValue::List(vec![
                BencodedValue::String(b"hello".to_vec().into()),
                BencodedValue::Integer(3)
            ])
        );

        let (index, value) = decode_bencoded_list("l4:spam4:eggse".as_bytes());
        assert_eq!(index, 14);
        assert_eq!(
            value,
            BencodedValue::List(vec![
                BencodedValue::String(b"spam".to_vec().into()),
                BencodedValue::String(b"eggs".to_vec().into())
            ])
        );

        let (index, value) = decode_bencoded_list("l4:spaml1:a1:bee".as_bytes());
        assert_eq!(index, 16);
        assert_eq!(
            value,
            BencodedValue::List(vec![
                BencodedValue::String(b"spam".to_vec().into()),
                BencodedValue::List(vec![
                    BencodedValue::String(b"a".to_vec().into()),
                    BencodedValue::String(b"b".to_vec().into())
                ])
            ])
        );
    }

    #[test]
    fn test_decode_bencoded_dict() {
        let (index, value) = decode_bencoded_dict("d3:cow3:moo4:spam4:eggse".as_bytes());
        assert_eq!(index, 24);
        let mut expected = BTreeMap::new();
        expected.insert(
            BencodedString("cow".into()),
            BencodedValue::String(b"moo".to_vec().into()),
        );
        expected.insert(
            BencodedString(b"spam".to_vec()),
            BencodedValue::String(b"eggs".to_vec().into()),
        );
        assert_eq!(value, BencodedValue::Dict(expected));

        let (index, value) = decode_bencoded_dict("d4:spaml1:a1:bee".as_bytes());
        assert_eq!(index, 16);
        let mut expected = BTreeMap::new();
        expected.insert(
            BencodedString(b"spam".to_vec()),
            BencodedValue::List(vec![
                BencodedValue::String(b"a".to_vec().into()),
                BencodedValue::String(b"b".to_vec().into()),
            ]),
        );
        assert_eq!(value, BencodedValue::Dict(expected), "d4:spaml1:a1:bee");

        let (index, value) = decode_bencoded_dict("d4:foodd1:a3:baree".as_bytes());
        assert_eq!(index, 18);
        let mut expected = BTreeMap::new();
        expected.insert(
            BencodedString(b"food".to_vec()),
            BencodedValue::Dict(BTreeMap::from([(
                BencodedString(b"a".to_vec()),
                BencodedValue::String(b"bar".to_vec().into()),
            )])),
        );
        assert_eq!(value, BencodedValue::Dict(expected), "d4:foodd1:a3:baree");

        let (index, value) = decode_bencoded_dict("d4:foodd1:a3:bare5:drinkd1:b3:bazee".as_bytes());
        assert_eq!(index, 35);
        let mut expected = BTreeMap::new();
        expected.insert(
            BencodedString(b"food".to_vec()),
            BencodedValue::Dict(BTreeMap::from([(
                BencodedString(b"a".to_vec()),
                BencodedValue::String(b"bar".to_vec().into()),
            )])),
        );
        expected.insert(
            BencodedString(b"drink".to_vec()),
            BencodedValue::Dict(BTreeMap::from([(
                BencodedString(b"b".to_vec()),
                BencodedValue::String(b"baz".to_vec().into()),
            )])),
        );
        assert_eq!(
            value,
            BencodedValue::Dict(expected),
            "d4:foodd1:a3:bare5:drinkd1:b3:bazee"
        );
    }

    #[test]
    fn test_decode_bencoded_dict_with_bytes() {
        // Some non-utf8 bytes
        let input = b"d4:foodd1:a4:\x80\x81\x82\x83ee";
        let (index, value) = decode_bencoded_dict(input);
        assert_eq!(index, 19);
        let mut expected = BTreeMap::new();
        expected.insert(
            BencodedString(b"food".to_vec()),
            BencodedValue::Dict(BTreeMap::from([(
                BencodedString(b"a".to_vec()),
                BencodedValue::String(b"\x80\x81\x82\x83".to_vec().into()),
            )])),
        );
        assert_eq!(
            value,
            BencodedValue::Dict(expected),
            "d4:foodd1:a4:<byte>ee"
        );
        assert_eq!(
            format!("{}", value),
            "{food: {a: ����}}",
            "d4:foodd1:a4:<byte>ee"
        );

        // Another
        let input = b"d12:min intervali60e5:peers18:\xa5\xe8!M\xc8\xe5\xb2>RY\xc9\x01\xb2>U\x14\xc9%8:completei3e10:incompletei1e8:intervali60ee";
        let (index, value) = decode_bencoded_dict(input);
        assert_eq!(index, 92);
        let mut expected = BTreeMap::new();
        expected.insert(
            BencodedString(b"interval".to_vec()),
            BencodedValue::Integer(60),
        );
        expected.insert(
            BencodedString(b"min interval".to_vec()),
            BencodedValue::Integer(60),
        );
        expected.insert(
            BencodedString(b"peers".to_vec()),
            BencodedValue::String(
                b"\xa5\xe8!M\xc8\xe5\xb2>RY\xc9\x01\xb2>U\x14\xc9%"
                    .to_vec()
                    .into(),
            ),
        );
        expected.insert(
            BencodedString(b"complete".to_vec()),
            BencodedValue::Integer(3),
        );
        expected.insert(
            BencodedString(b"incomplete".to_vec()),
            BencodedValue::Integer(1),
        );
        assert_eq!(
            value,
            BencodedValue::Dict(expected),
            "d8:intervali60e12:min intervali60e5:peers18:��!M��>RY��>U�%8:completei3e10:incompletei1ee"
        );
    }

    // Test encoding
    #[test]
    fn test_encode_bencoded_vec() {
        let value = BencodedValue::String(b"hello".to_vec().into());
        assert_eq!(value.bencode(), "5:hello".as_bytes());
    }

    #[test]
    fn test_encode_bencoded_integer() {
        let value = BencodedValue::Integer(3);
        assert_eq!(value.bencode(), "i3e".as_bytes(), "i3e");

        let value = BencodedValue::Integer(-3);
        assert_eq!(value.bencode(), "i-3e".as_bytes(), "i-3e");
    }

    #[test]
    fn test_encode_bencoded_list() {
        let value = BencodedValue::List(vec![
            BencodedValue::String(b"hello".to_vec().into()),
            BencodedValue::Integer(3),
        ]);
        assert_eq!(value.bencode(), "l5:helloi3ee".as_bytes(), "l5:helloi3ee");

        let value = BencodedValue::List(vec![
            BencodedValue::String(b"spam".to_vec().into()),
            BencodedValue::String(b"eggs".to_vec().into()),
        ]);
        assert_eq!(
            value.bencode(),
            "l4:spam4:eggse".as_bytes(),
            "l4:spam4:eggse"
        );

        let value = BencodedValue::List(vec![
            BencodedValue::String(b"spam".to_vec().into()),
            BencodedValue::List(vec![
                BencodedValue::String(b"a".to_vec().into()),
                BencodedValue::String(b"b".to_vec().into()),
            ]),
        ]);
        assert_eq!(
            value.bencode(),
            "l4:spaml1:a1:bee".as_bytes(),
            "l4:spaml1:a1:bee"
        );
    }

    #[test]
    fn test_encode_bencoded_dict() {
        // Test empty dict
        let dict = BTreeMap::new();
        let value = BencodedValue::Dict(dict);
        assert_eq!(value.bencode(), "de".as_bytes());

        // Test {"cow": "moo"}
        let mut dict = BTreeMap::new();
        dict.insert(
            BencodedString(b"cow".to_vec()),
            BencodedValue::String(b"moo".to_vec().into()),
        );
        let value = BencodedValue::Dict(dict);
        assert_eq!(value.bencode(), "d3:cow3:mooe".as_bytes(), "d3:cow3:mooe");

        // Test {"spam": ["a", "b"]}
        let mut dict = BTreeMap::new();
        dict.insert(
            BencodedString(b"spam".to_vec()),
            BencodedValue::List(vec![
                BencodedValue::String(b"a".to_vec().into()),
                BencodedValue::String(b"b".to_vec().into()),
            ]),
        );
        let value = BencodedValue::Dict(dict);
        assert_eq!(
            value.bencode(),
            "d4:spaml1:a1:bee".as_bytes(),
            "d4:spaml1:a1:bee"
        );

        // Test {"food": {"a": "bar"}, "drink": {"b": "baz"}}
        let mut dict = BTreeMap::new();
        dict.insert(
            BencodedString(b"food".to_vec()),
            BencodedValue::Dict(BTreeMap::from([(
                BencodedString(b"a".to_vec()),
                BencodedValue::String(b"bar".to_vec().into()),
            )])),
        );
        dict.insert(
            BencodedString(b"drink".to_vec()),
            BencodedValue::Dict(BTreeMap::from([(
                BencodedString(b"b".to_vec()),
                BencodedValue::String(b"baz".to_vec().into()),
            )])),
        );
        let value = BencodedValue::Dict(dict);
        let value_bencode_u8 = value.bencode();
        let value_bencode = String::from_utf8_lossy(&value_bencode_u8);
        // Test String
        assert_eq!(
            value_bencode, "d5:drinkd1:b3:baze4:foodd1:a3:baree",
            "d5:drinkd1:b3:baze4:foodd1:a3:baree"
        );
        // Test Bytes
        assert_eq!(
            value.bencode(),
            "d5:drinkd1:b3:baze4:foodd1:a3:baree".as_bytes(),
            "d5:drinkd1:b3:baze4:foodd1:a3:baree"
        );
    }

    // Test printing of BencodedString
    #[test]
    fn test_bencoded_string_display() {
        let bencoded_string = BencodedString(b"hello".to_vec());
        assert_eq!(format!("{}", bencoded_string), "hello");
    }

    // Test printing of BencodedValue
    #[test]
    fn test_bencoded_value_display() {
        let bencoded_value = BencodedValue::String(b"hello".to_vec().into());
        assert_eq!(format!("{}", bencoded_value), "hello");

        let bencoded_value = BencodedValue::Integer(3);
        assert_eq!(format!("{}", bencoded_value), "3");

        let bencoded_value = BencodedValue::List(vec![
            BencodedValue::String(b"hello".to_vec().into()),
            BencodedValue::Integer(3),
        ]);
        assert_eq!(format!("{}", bencoded_value), "[hello, 3]");

        let mut dict = BTreeMap::new();
        dict.insert(
            BencodedString(b"cow".to_vec()),
            BencodedValue::String(b"moo".to_vec().into()),
        );
        dict.insert(
            BencodedString(b"spam".to_vec()),
            BencodedValue::String(b"eggs".to_vec().into()),
        );
        let bencoded_value = BencodedValue::Dict(dict);
        assert_eq!(format!("{}", bencoded_value), "{cow: moo, spam: eggs}");
    }
}
