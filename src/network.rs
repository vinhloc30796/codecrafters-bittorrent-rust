

use crate::decoder::{BencodedString, BencodedValue};
use anyhow::{anyhow, Error};
use serde::{Serialize};


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
    pub peers: Vec<String>,
}

impl TryFrom<&BencodedValue> for TrackerResponse {
    type Error = Error;

    fn try_from(value: &BencodedValue) -> Result<Self, Self::Error> {
        let mut interval: u64 = 0;
        let mut peers: Vec<String> = Vec::new();

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
                            let ip_str = format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3]);
                            let port_str = format!("{}", u16::from_be_bytes([port[0], port[1]]));
                            let peer_str = format!("{}:{}", ip_str, port_str);
                            peers.push(peer_str);
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

pub async fn ping_tracker(tracker_url: &str, info_hash: [u8; 20], length: i64) -> Result<TrackerResponse, Error> {
    let payload = TrackerPayload {
        // info_hash: metainfo.info.info_hash().as_bytes().to_vec(),
        peer_id: "-TR2940-2b3b6b4b5b6b".to_string(),
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
