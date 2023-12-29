use bittorrent_starter_rust::decoder::decode_bencoded_value;
use bittorrent_starter_rust::file::{Info, MetainfoFile};
use bittorrent_starter_rust::network::{ping_tracker, PeerMessage, PeerStream};
use hex::ToHex;
use sha1::{Digest, Sha1};
use std::env;
use std::path::PathBuf;

// Available if you need it!
// use serde_bencode;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    // println!("Logs from your program will appear here!");

    match command as &str {
        // Usage: your_bittorrent.sh decode "<encoded_value>"
        "decode" => {
            let encoded_value = &args[2];
            let (_, decoded_value) = decode_bencoded_value(encoded_value);
            let json_value = serde_json::Value::from(decoded_value);
            println!("{}", json_value);
        }
        // Usage: your_bittorrent.sh info "<torrent_file>"
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
        // Usage: your_bittorrent.sh peers "<torrent_file>"
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
        // Usage: your_bittorrent.sh handshake "<torrent_file>"
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

            match peer_stream.handshake(&metainfo.info.info_hash()) {
                Ok(handshake) => {
                    println!("Handshake: {:?}", handshake);
                    let hex_peer_id = handshake
                        .peer_id
                        .iter()
                        .map(|b| format!("{:02x}", b))
                        .collect::<String>();
                    println!("Peer ID: {}", hex_peer_id);
                }
                Err(e) => {
                    println!("Handshake: Error: {}", e);
                }
            }
        }
        // Usage: your_bittorrent.sh download_piece -o /tmp/test-piece-0 "<torrent_file>" <piece_index>
        "download_piece" => {
            let filename = &args[2];
            let selected_index = args[3].parse::<usize>().unwrap();
            let metainfo = MetainfoFile::read_from_file(filename).unwrap();
            let info: Info = metainfo.info;

            let peers =
                match ping_tracker(metainfo.announce.as_str(), info.info_hash(), info.length).await
                {
                    Ok(tracker_response) => tracker_response.peers,
                    Err(e) => {
                        println!("Peers: Error: {}", e);
                        return;
                    }
                };
            let peer = peers.first().unwrap();
            let mut peer_stream = PeerStream::new(*peer);

            match peer_stream.handshake(&info.info_hash()) {
                Ok(handshake) => {
                    println!("Handshake: {:?}", handshake);
                    let hex_peer_id = handshake
                        .peer_id
                        .iter()
                        .map(|b| format!("{:02x}", b))
                        .collect::<String>();
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

            match peer_stream.write_interested() {
                Ok(_) => {
                    println!("Interested: Sent");
                }
                Err(e) => {
                    println!("Interested: Error: {}", e);
                }
            }

            match peer_stream.read_unchoke() {
                Ok(_) => {
                    println!("Unchoke: Received");
                }
                Err(e) => {
                    println!("Unchoke: Error: {}", e);
                }
            }

            // Chunk pieces into 16 * 1024 byte chunks with index
            // then download each chunk
            let output = PathBuf::from(format!("/tmp/test-piece-{}", &selected_index));
            let selected_piece_hash = &info.piece_hash()[selected_index];
            let downloads = peer_stream
                .download_piece(selected_index as u32, &info.piece_length)
                .unwrap();
            // Zip the downloads with the piece hashes & map to download::save_piece into /tmp/test-piece-{idx}
            let downloaded_payload: Vec<u8> = downloads.iter().fold(vec![], |mut acc, download| {
                match download {
                    PeerMessage::Piece {
                        index: _,
                        begin: _,
                        block,
                    } => {
                        // append the block to the acc
                        acc.extend_from_slice(block);
                    }
                    _ => {}
                }
                acc
            });
            let mut hashers = Sha1::new();
            hashers.update(&downloaded_payload);
            let downloaded_hash: String = hashers.finalize().encode_hex::<String>();
            if &downloaded_hash == selected_piece_hash {
                // Save the piece to /tmp/test-piece-{idx}
                std::fs::write(&output, downloaded_payload).unwrap();
                let output_str = output.to_str().unwrap();
                println!("Piece {} downloaded to {}.", selected_index, output_str);
            } else {
                println!(
                    "Downloaded piece {} hash {} does not match expected hash {}.",
                    selected_index, downloaded_hash, selected_piece_hash
                );
            }
        }
        _ => {
            println!("unknown command: {}", args[1])
        }
    }
}
