use crate::decoder::{BencodedString, BencodedValue};
use anyhow::{anyhow, Error};
use serde::Serialize;
use std::{
    fmt::{self, Display, Formatter},
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddrV4, TcpStream},
};

const CHUNK_SIZE: i64 = 16 * 1024;
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

#[derive(Debug, PartialEq)]
pub enum PeerMessage {
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have,
    Bitfield(Vec<u8>),
    Request {
        index: u32,
        begin: u32,
        length: u32,
    },
    Piece {
        index: u32,
        begin: u32,
        block: [u8; 16 * 1024],
    },
    Cancel {
        index: u32,
        begin: u32,
        length: u32,
    },
}

impl From<Vec<u8>> for PeerMessage {
    fn from(value: Vec<u8>) -> Self {
        match value[4] {
            0 => PeerMessage::Choke,
            1 => PeerMessage::Unchoke,
            2 => PeerMessage::Interested,
            3 => PeerMessage::NotInterested,
            4 => PeerMessage::Have,
            5 => PeerMessage::Bitfield(value[5..].to_vec()),
            6 => PeerMessage::Request {
                index: u32::from_be_bytes(value[5..9].try_into().unwrap()), // [5, 6, 7, 8]
                begin: u32::from_be_bytes(value[9..13].try_into().unwrap()), // [9, 10, 11, 12]
                length: u32::from_be_bytes(value[13..].try_into().unwrap()), // [13, 14, 15, 16]
            },
            7 => {
                let mut block = [0; 16 * 1024];
                // fill in block with the rest of the bytes & pad with 0s
                block[..value.len() - 13].copy_from_slice(&value[13..]);
                PeerMessage::Piece {
                    index: u32::from_be_bytes(value[5..9].try_into().unwrap()), // [5, 6, 7, 8]
                    begin: u32::from_be_bytes(value[9..13].try_into().unwrap()), // [9, 10, 11, 12]
                    block,
                }
            }
            8 => PeerMessage::Cancel {
                index: u32::from_be_bytes(value[5..9].try_into().unwrap()), // [5, 6, 7, 8]
                begin: u32::from_be_bytes(value[9..13].try_into().unwrap()), // [9, 10, 11, 12]
                length: u32::from_be_bytes([value[13], value[14], value[15], value[16]]),
            },
            _ => panic!("Invalid message type"),
        }
    }
}

impl From<&PeerMessage> for Vec<u8> {
    fn from(value: &PeerMessage) -> Self {
        let mut message: Vec<u8> = Vec::new();
        match value {
            PeerMessage::Choke => {
                let length = 1 as u32;
                message.extend(length.to_be_bytes().to_vec());
                message.push(0)
            }
            PeerMessage::Unchoke => {
                let length = 1 as u32;
                message.extend(length.to_be_bytes().to_vec());
                message.push(1)
            }
            PeerMessage::Interested => {
                let length = 1 as u32;
                message.extend(length.to_be_bytes().to_vec());
                message.push(2)
            }
            PeerMessage::NotInterested => {
                let length = 1 as u32;
                message.extend(length.to_be_bytes().to_vec());
                message.push(3)
            }
            PeerMessage::Have => {
                let length = 5 as u32;
                message.extend(length.to_be_bytes().to_vec());
                message.push(4)
            }
            PeerMessage::Bitfield(payload) => {
                let length = payload.len() as u32 + 1;
                message.extend(length.to_be_bytes().to_vec());
                message.push(5);
                message.extend(payload);
            }
            PeerMessage::Request {
                index,
                begin,
                length,
            } => {
                message.extend(length.to_be_bytes().to_vec());
                message.push(6);
                message.extend(index.to_be_bytes().to_vec());
                message.extend(begin.to_be_bytes().to_vec());
                message.extend(length.to_be_bytes().to_vec());
            }
            PeerMessage::Piece {
                index,
                begin,
                block,
            } => {
                let length = 9 + block.len() as u32;
                message.extend(length.to_be_bytes().to_vec());
                message.push(7);
                message.extend(index.to_be_bytes().to_vec());
                message.extend(begin.to_be_bytes().to_vec());
                message.extend(block.to_vec());
            }
            PeerMessage::Cancel {
                index,
                begin,
                length,
            } => {
                message.extend(length.to_be_bytes().to_vec());
                message.push(8);
                message.extend(index.to_be_bytes().to_vec());
                message.extend(begin.to_be_bytes().to_vec());
                message.extend(length.to_be_bytes().to_vec());
            }
        }
        message
    }
}

impl Display for PeerMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            PeerMessage::Choke => write!(f, "Choke"),
            PeerMessage::Unchoke => write!(f, "Unchoke"),
            PeerMessage::Interested => write!(f, "Interested"),
            PeerMessage::NotInterested => write!(f, "NotInterested"),
            PeerMessage::Have => write!(f, "Have"),
            PeerMessage::Bitfield(_) => write!(f, "Bitfield"),
            PeerMessage::Request {
                index,
                begin,
                length,
            } => write!(
                f,
                "Request {{ index: {}, begin: {}, length: {} }}",
                index, begin, length
            ),
            PeerMessage::Piece {
                index,
                begin: _,
                block,
            } => write!(
                f,
                "Piece {{ index: {}, block: {:?}... }}",
                index,
                // trim the block to the first 10 bytes
                &block[..10]
            ),
            PeerMessage::Cancel {
                index,
                begin,
                length,
            } => write!(
                f,
                "Cancel {{ index: {}, begin: {}, length: {} }}",
                index, begin, length
            ),
        }
    }
}

pub struct PeerStream {
    stream: TcpStream,
    state: PeerState,
}

enum PeerState {
    Init = 0,
    Handshake,
    Bitfield,
    Interested,
    Unchoke,
}

impl PeerStream {
    pub fn new(peer_addr: SocketAddrV4) -> Self {
        let stream = TcpStream::connect(peer_addr).unwrap();
        PeerStream {
            stream,
            state: PeerState::Init,
        }
    }

    pub fn handshake(&mut self, info_hash: &[u8; 20]) -> Result<PeerHandshake, Error> {
        let handshake = PeerHandshake::new(info_hash.to_vec(), PEER_ID.as_bytes().to_vec());
        let handshake_bytes: Vec<u8> = handshake.into();
        self.stream.write_all(&handshake_bytes)?;

        // Read the handshake response
        let mut buf = [0; 68];
        self.stream.read(&mut buf)?;
        let peer_handshake = PeerHandshake::from(buf.to_vec());
        self.state = PeerState::Handshake;
        // println!("Peer Handshake: {:?}", peer_handshake);
        Ok(peer_handshake)
    }

    pub fn read(&mut self) -> Result<PeerMessage, Error> {
        // Assert that we are at least in the handshake state
        match self.state {
            PeerState::Init => panic!("Cannot read if not yet handshaked"),
            _ => {}
        }

        // Read the length prefix
        let mut length_prefix: [u8; 4] = [0; 4];
        self.stream.read_exact(&mut length_prefix)?;
        let length = u32::from_be_bytes(length_prefix);

        // Read the message type
        let mut message_type: [u8; 1] = [0; 1];
        self.stream.read_exact(&mut message_type)?;

        // Read the payload
        let mut payload: Vec<u8> = vec![0; length as usize - 1];
        self.stream.read_exact(&mut payload)?;

        let mut full_msg: Vec<u8> = Vec::new();
        full_msg.extend(length_prefix.to_vec());
        full_msg.extend(message_type.to_vec());
        full_msg.extend(payload.to_vec());
        let msg = PeerMessage::from(full_msg);
        Ok(msg)
    }

    pub fn write(&mut self, message: &PeerMessage) -> Result<(), Error> {
        // Assert that we are in the handshake state
        match self.state {
            PeerState::Init => return Err(anyhow!("Cannot write if not yet handshaked")),
            _ => {}
        }

        // Write the message
        let message_bytes: Vec<u8> = message.into();
        self.stream.write_all(&message_bytes)?;
        Ok(())
    }

    // Specific steps
    pub fn read_bitfield(&mut self) -> Result<PeerMessage, Error> {
        // Assert that we are in the handshake state
        match self.state {
            PeerState::Handshake => {}
            _ => return Err(anyhow!("Bitfield can only be read from Handshake")),
        }

        // Read the bitfield message
        let message = self.read()?;
        match message {
            PeerMessage::Bitfield(_) => {
                self.state = PeerState::Bitfield;
                Ok(message)
            }
            _ => Err(anyhow!("Expected bitfield message")),
        }
    }

    pub fn write_interested(&mut self) -> Result<(), Error> {
        // Assert that we are in the Bitfield state
        match self.state {
            PeerState::Bitfield => {}
            _ => return Err(anyhow!("Not in bitfield state")),
        }

        // Write the interested message
        let message = PeerMessage::Interested;
        self.write(&message)?;
        self.state = PeerState::Interested;
        Ok(())
    }

    pub fn read_unchoke(&mut self) -> Result<PeerMessage, Error> {
        // Assert that we are in the Interested state
        match self.state {
            PeerState::Interested => {}
            _ => return Err(anyhow!("Not in interested state")),
        }

        // Read the unchoke message
        let message = self.read()?;
        match message {
            PeerMessage::Unchoke => {
                self.state = PeerState::Unchoke;
                Ok(message)
            }
            _ => Err(anyhow!("Expected unchoke message")),
        }
    }

    pub fn download_piece(
        &mut self,
        piece_id: u32,
        piece_length: &i64,
    ) -> Result<Vec<PeerMessage>, Error> {
        // Assert that we are in the Unchoke state
        match self.state {
            PeerState::Unchoke => {}
            _ => return Err(anyhow!("Not in unchoke state")),
        }

        // Make a Vec of requests to cover piece_length with chunk
        let n_reqs = (piece_length / CHUNK_SIZE) as usize;
        let reqs = (0..n_reqs)
            .map(|i| PeerMessage::Request {
                index: piece_id,
                begin: (i * CHUNK_SIZE as usize) as u32,
                length: CHUNK_SIZE as u32,
            })
            .collect::<Vec<PeerMessage>>();

        // Iter & map over the requests
        let responses = reqs
            .iter()
            .map(|req| {
                // Send the request
                self.write(req)?;

                // Wait for the piece response
                let resp = self.read()?;
                match resp {
                    PeerMessage::Piece {
                        index: _,
                        begin: _,
                        block: _,
                    } => Ok(resp),
                    _ => Err(anyhow!("Expected piece message")),
                }
            })
            .collect::<Result<Vec<PeerMessage>, Error>>()?;

        Ok(responses)
    }
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

    #[test]
    fn test_peer_message_from() {
        // Choke
        let message_bytes = vec![0, 0, 0, 1, 0];
        let message = PeerMessage::from(message_bytes);
        assert_eq!(message, PeerMessage::Choke);

        // Bitfield
        let message_bytes = vec![0, 0, 0, 6, 5, 1, 2, 3, 4, 5];
        let message = PeerMessage::from(message_bytes);
        assert_eq!(message, PeerMessage::Bitfield(vec![1, 2, 3, 4, 5]));
    }
}
