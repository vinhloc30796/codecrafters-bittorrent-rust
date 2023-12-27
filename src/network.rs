use crate::decoder::{BencodedString, BencodedValue};
use anyhow::{anyhow, Error};
use serde::Serialize;
use std::{
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddrV4, TcpStream},
};

const PEER_ID: &str = "-TR2940-2b3b6b4b5b6b";

// Serialize the payload to a query string
#[derive(Serialize)]
pub struct TrackerPayload {
    // the info hash of the torrent
    // pub info_hash: Vec<u8>,
    // a unique identifier for this client
    pub peer_id: String,
    // port: the port this client is listening on (default: 6881)
    pub port: u64,
    // uploaded: the total amount uploaded so far (default: 0)
    pub uploaded: u64,
    // downloaded: the total amount downloaded so far (default: 0)
    pub downloaded: u64,
    // left: the number of bytes left to download
    pub left: u64,
    // compact: setting this to 1 indicates that we would like to receive a compact response
    #[serde(serialize_with = "serde_bool_to_int")]
    pub compact: bool,
}

// Input: d69f91e6b2ae4c542468d1073a71d4ea13879a7f;
// Output: %d6%9f%91%e6%b2%ae%4c%54%24%68%d1%07%3a%71%d4%ea%13%87%9a%7f
pub fn urlencode(t: &[u8; 20]) -> anyhow::Result<String> {
    let mut s = String::new();
    for b in t {
        s.push('%');
        s.push_str(&format!("{:02x}", b));
    }
    Ok(s)
}

// serialize to 1 if true, 0 if false
pub fn serde_bool_to_int<S>(x: &bool, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_u8(if *x { 1 } else { 0 })
}

pub struct TrackerResponse {
    // interval: An integer, indicating how often
    // this client should make a request to the tracker
    pub interval: u64,
    // peers: A string, which contains list of peers that your client can connect to.
    // A string, which contains list of peers that your client can connect to.
    // Each peer is represented using 6 bytes.
    // The first 4 bytes are the peer's IP address and the last 2 bytes are the peer's port number
    // pub peers: Vec<String>,
    pub peers: Vec<SocketAddrV4>,
}

impl TryFrom<&BencodedValue> for TrackerResponse {
    type Error = Error;

    fn try_from(value: &BencodedValue) -> Result<Self, Self::Error> {
        let mut interval: u64 = 0;
        // let mut peers: Vec<String> = Vec::new();
        let mut peers: Vec<SocketAddrV4> = Vec::new();

        // Error if not a BencodedValue::Dict
        match value {
            BencodedValue::Dict(dict) => {
                // Error if no interval
                match dict.get(&BencodedString(b"interval".to_vec())) {
                    Some(BencodedValue::Integer(i)) => {
                        if *i < 0 {
                            return Err(anyhow!("Interval is negative"));
                        }
                        interval = *i as u64;
                    }
                    _ => {
                        // print out warning
                        println!("No interval");
                    }
                }
                // Error if no peers
                match dict.get(&BencodedString(b"peers".to_vec())) {
                    Some(BencodedValue::String(s)) => {
                        let peer_bytes: Vec<u8> = s.into();
                        let peer_chunks: Vec<&[u8]> = peer_bytes.chunks(6).collect();

                        peer_chunks.iter().for_each(|chunk| {
                            let ip = &chunk[0..4];
                            let port = &chunk[4..6];
                            let port_str = format!("{}", u16::from_be_bytes([port[0], port[1]]));
                            // std::net
                            let new_ip = Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]);
                            let new_peer = SocketAddrV4::new(new_ip, port_str.parse().unwrap());
                            peers.push(new_peer);
                        });
                    }
                    _ => return Err(anyhow!("No peers")),
                }
            }
            _ => return Err(anyhow!("Not a dict")),
        }

        Ok(TrackerResponse { interval, peers })
    }
}

// default values for the tracker payload
impl Default for TrackerPayload {
    fn default() -> Self {
        TrackerPayload {
            // info_hash: vec![],
            peer_id: "".to_string(),
            port: 6881,
            uploaded: 0,
            downloaded: 0,
            left: 0,
            compact: true,
        }
    }
}

#[derive(Debug)]
pub struct PeerHandshake {
    // length of the protocol string (BitTorrent protocol) which is 19 (1 byte)
    length: u64,
    // protocol string (19 bytes) -- default: 'BitTorrent protocol'
    protocol: String,
    // 8 reserved bytes (all 0) (8 bytes)
    reserved: Vec<u8>,
    // info hash (20 bytes)
    info_hash: Vec<u8>,
    // peer id (20 bytes)
    pub peer_id: Vec<u8>,
}

impl Default for PeerHandshake {
    fn default() -> Self {
        PeerHandshake {
            length: 19,
            protocol: "BitTorrent protocol".to_string(),
            reserved: vec![0; 8],
            info_hash: vec![],
            peer_id: PEER_ID.as_bytes().to_vec(),
        }
    }
}

impl PeerHandshake {
    pub fn new(info_hash: Vec<u8>, peer_id: Vec<u8>) -> Self {
        PeerHandshake {
            info_hash,
            peer_id,
            // Rest is default
            ..Default::default()
        }
    }
}

impl From<PeerHandshake> for Vec<u8> {
    fn from(value: PeerHandshake) -> Self {
        let mut handshake: Vec<u8> = Vec::new();
        handshake.push(value.length as u8);
        handshake.extend(value.protocol.as_bytes());
        handshake.extend(&value.reserved);
        handshake.extend(&value.info_hash);
        handshake.extend(&value.peer_id);
        handshake
    }
}

impl From<Vec<u8>> for PeerHandshake {
    fn from(value: Vec<u8>) -> Self {
        let mut handshake = PeerHandshake::default();
        handshake.length = value[0] as u64;
        handshake.protocol = String::from_utf8(value[1..20].to_vec()).unwrap();
        handshake.reserved = value[20..28].to_vec();
        handshake.info_hash = value[28..48].to_vec();
        handshake.peer_id = value[48..68].to_vec();
        handshake
    }
}

pub async fn ping_tracker(
    tracker_url: &str,
    info_hash: [u8; 20],
    length: i64,
) -> Result<TrackerResponse, Error> {
    let payload = TrackerPayload {
        // info_hash: metainfo.info.info_hash().as_bytes().to_vec(),
        peer_id: PEER_ID.to_string(),
        port: 6881,
        uploaded: 0,
        downloaded: 0,
        left: length as u64,
        compact: true,
    };

    // Just add a % in front of each byte (2 chars) by iter String
    let url = format!(
        "{}?{}&info_hash={}",
        tracker_url,
        serde_urlencoded::to_string(&payload)?,
        url_encode(&info_hash).expect("Failed to encode info hash")
    );
    // Preview the url
    println!("URL: {}", url);
    let resp_bytes = reqwest::get(&url).await?.bytes().await?;
    let resp_u8: &[u8] = &resp_bytes;
    println!("Body Bytes: {:?}", resp_bytes);

    let de_bencoded: BencodedValue = BencodedValue::from(resp_u8);
    println!("Bencoded Response: {}", de_bencoded);
    let tracker_response = TrackerResponse::try_from(&de_bencoded)?;

    Ok(tracker_response)
}

pub fn url_encode(t: &[u8; 20]) -> anyhow::Result<String> {
    let mut s = String::new();
    for b in t {
        s.push('%');
        s.push_str(&format!("{:02x}", b));
    }
    Ok(s)
}

pub fn shake_hands(peer_addr: SocketAddrV4, info_hash: [u8; 20]) -> Result<PeerHandshake, Error> {
    let mut stream = TcpStream::connect(peer_addr)?;
    let handshake = PeerHandshake::new(info_hash.to_vec(), PEER_ID.as_bytes().to_vec());
    let handshake_bytes: Vec<u8> = handshake.into();
    stream.write_all(&handshake_bytes)?;
    let mut buf = [0; 1024];
    stream.read(&mut buf)?;
    let peer_handshake = PeerHandshake::from(buf.to_vec());
    Ok(peer_handshake)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_urlencode() {
        let info_hash = [
            0xd6, 0x9f, 0x91, 0xe6, 0xb2, 0xae, 0x4c, 0x54, 0x24, 0x68, 0xd1, 0x07, 0x3a, 0x71,
            0xd4, 0xea, 0x13, 0x87, 0x9a, 0x7f,
        ];
        let encoded = urlencode(&info_hash).unwrap();
        assert_eq!(
            encoded,
            "%d6%9f%91%e6%b2%ae%4c%54%24%68%d1%07%3a%71%d4%ea%13%87%9a%7f"
        );
    }

    #[test]
    fn test_tracker_payload_default() {
        let payload = TrackerPayload::default();
        assert_eq!(payload.port, 6881);
        assert_eq!(payload.uploaded, 0);
        assert_eq!(payload.downloaded, 0);
        assert_eq!(payload.left, 0);
        assert_eq!(payload.compact, true);
    }

    #[test]
    fn test_tracker_payload_serialize() {
        let payload = TrackerPayload {
            // info_hash: vec![],
            peer_id: "peer_id".to_string(),
            port: 6881,
            uploaded: 0,
            downloaded: 0,
            left: 0,
            compact: true,
        };
        let serialized = serde_urlencoded::to_string(&payload).unwrap();
        assert_eq!(
            serialized,
            "peer_id=peer_id&port=6881&uploaded=0&downloaded=0&left=0&compact=1"
        );
    }

    #[test]
    fn test_tracker_response_try_from() {
        let bencoded = BencodedValue::from(
            b"d8:intervali1800e5:peers12:\x7f\x00\x00\x01\x1a\x90\x7f\x00\x00\x01\x1b\x90e"
            .as_slice(),
        );
        let tracker_response = TrackerResponse::try_from(&bencoded).unwrap();
        assert_eq!(tracker_response.interval, 1800);
        // Test without ordering
        assert!(tracker_response
            .peers
            .contains(&SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 6800)));
        assert!(tracker_response
            .peers
            .contains(&SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 7056)));
    }

    #[test]
    fn test_peer_handshake_default() {
        let handshake = PeerHandshake::default();
        assert_eq!(handshake.length, 19);
        assert_eq!(handshake.protocol, "BitTorrent protocol");
        assert_eq!(handshake.reserved, vec![0; 8]);
        assert_eq!(handshake.info_hash, Vec::<u8>::new());
        assert_eq!(handshake.peer_id, PEER_ID.as_bytes());
    }

    #[test]
    fn test_peer_handshake_from() {
        let handshake_bytes = vec![
            19, 66, 105, 116, 84, 111, 114, 114, 101, 110, 116, 32, 112, 114, 111, 116, 111, 99,
            111, 108, 0, 0, 0, 0, 0, 0, 0, 0, 214, 159, 145, 230, 178, 174, 76, 84, 36, 104, 209,
            7, 58, 113, 212, 234, 19, 135, 154, 127, 45, 84, 82, 50, 57, 52, 48, 45, 50, 98, 51,
            98, 54, 98, 52, 98, 53, 98, 54, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let handshake = PeerHandshake::from(handshake_bytes);
        assert_eq!(handshake.length, 19);
        assert_eq!(handshake.protocol, "BitTorrent protocol");
        assert_eq!(handshake.reserved, vec![0; 8]);
        assert_eq!(
            handshake.info_hash,
            vec![
                214, 159, 145, 230, 178, 174, 76, 84, 36, 104, 209, 7, 58, 113, 212, 234, 19, 135,
                154, 127
            ]
        );
        assert_eq!(
            handshake.peer_id,
            vec![45, 84, 82, 50, 57, 52, 48, 45, 50, 98, 51, 98, 54, 98, 52, 98, 53, 98, 54, 0]
        );
    }

    #[test]
    fn test_peer_handshake_into() {
        let handshake = PeerHandshake {
            length: 19,
            protocol: "BitTorrent protocol".to_string(),
            reserved: vec![0; 8],
            info_hash: vec![
                214, 159, 145, 230, 178, 174, 76, 84, 36, 104, 209, 7, 58, 113, 212, 234, 19, 135,
                154, 127,
            ],
            peer_id: vec![
                45, 84, 82, 50, 57, 52, 48, 45, 50, 98, 51, 98, 54, 98, 52, 98, 53, 98, 54,
            ],
        };
        let handshake_bytes: Vec<u8> = handshake.into();
        assert_eq!(
            handshake_bytes,
            vec![
                19, 66, 105, 116, 84, 111, 114, 114, 101, 110, 116, 32, 112, 114, 111, 116, 111,
                99, 111, 108, 0, 0, 0, 0, 0, 0, 0, 0, 214, 159, 145, 230, 178, 174, 76, 84, 36,
                104, 209, 7, 58, 113, 212, 234, 19, 135, 154, 127, 45, 84, 82, 50, 57, 52, 48, 45,
                50, 98, 51, 98, 54, 98, 52, 98, 53, 98, 54
            ]
        );
    }
}