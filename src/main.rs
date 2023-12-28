use bittorrent_starter_rust::decoder::decode_bencoded_value;
use bittorrent_starter_rust::file::{Info, MetainfoFile};
use bittorrent_starter_rust::network::{ping_tracker, PeerStream};
// use sha1::{Digest, Sha1};
// use hex::ToHex;
use std::env;

// Available if you need it!
// use serde_bencode;

// Usage: your_bittorrent.sh decode "<encoded_value>"
// Usage: your_bittorrent.sh info "<torrent_file>"
#[tokio::main]
async fn main() {
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
            let filename = &args[2];
            let metainfo = MetainfoFile::read_from_file(filename).unwrap();

            // Print out the info dict
            let info: Info = metainfo.info;
            println!("Tracker URL: {}", metainfo.announce);
            println!("Length: {}", info.length);

            // Hash the info dict
            println!("Info Hash: {}", hex::encode(info.info_hash()));
            println!("Piece Length: {}", info.piece_length);
            let piece_hashes: Vec<String> = info.piece_hash();
            // Print piece hashes on new line
            println!("Pieces Hashes:\n{}", piece_hashes.join("\n"));
        }
        "peers" => {
            let filename = &args[2];
            let metainfo = MetainfoFile::read_from_file(filename).unwrap();

            match ping_tracker(
                metainfo.announce.as_str(),
                metainfo.info.info_hash(),
                metainfo.info.length,
            )
            .await
            {
                Ok(tracker_response) => {
                    println!("Peers:");
                    tracker_response.peers.iter().for_each(|peer| {
                        println!("{}", peer);
                    });
                }
                Err(e) => {
                    println!("Peers: Error: {}", e);
                }
            }
        }
        "handshake" => {
            let filename = &args[2];
            let metainfo = MetainfoFile::read_from_file(filename).unwrap();

            let peers = match ping_tracker(
                metainfo.announce.as_str(),
                metainfo.info.info_hash(),
                metainfo.info.length,
            )
            .await
            {
                Ok(tracker_response) => tracker_response.peers,
                Err(e) => {
                    println!("Peers: Error: {}", e);
                    return;
                }
            };
            let peer = peers.first().unwrap();
            let mut peer_stream = PeerStream::new(*peer);

            match peer_stream.handshake(metainfo.info.info_hash()) {
                Ok(handshake) => {
                    println!("Handshake: {:?}", handshake);
                    let hex_peer_id = handshake.peer_id.iter().map(|b| format!("{:02x}", b)).collect::<String>();
                    println!("Peer ID: {}", hex_peer_id);
                }
                Err(e) => {
                    println!("Handshake: Error: {}", e);
                }
            }
        }
        "download_piece" => {
            let filename = &args[2];
            let metainfo = MetainfoFile::read_from_file(filename).unwrap();

            let peers = match ping_tracker(
                metainfo.announce.as_str(),
                metainfo.info.info_hash(),
                metainfo.info.length,
            )
            .await
            {
                Ok(tracker_response) => tracker_response.peers,
                Err(e) => {
                    println!("Peers: Error: {}", e);
                    return;
                }
            };
            let peer = peers.first().unwrap();
            let mut peer_stream = PeerStream::new(*peer);

            match peer_stream.handshake(metainfo.info.info_hash()) {
                Ok(handshake) => {
                    println!("Handshake: {:?}", handshake);
                    let hex_peer_id = handshake.peer_id.iter().map(|b| format!("{:02x}", b)).collect::<String>();
                    println!("Peer ID: {}", hex_peer_id);
                }
                Err(e) => {
                    println!("Handshake: Error: {}", e);
                }
            }

            match peer_stream.read_bitfield() {
                Ok(bitfield) => {
                    println!("Bitfield: {:?}", bitfield);
                }
                Err(e) => {
                    println!("Bitfield: Error: {}", e);
                }
            }
        }
        _ => {
            println!("unknown command: {}", args[1])
        }
    }
}
