use bittorrent_starter_rust::decoder::decode_bencoded_value;
use bittorrent_starter_rust::file::{Info, MetainfoFile};
use bittorrent_starter_rust::network::{ping_tracker, PeerMessage, PeerStream};
use hex::ToHex;
use sha1::{Digest, Sha1};
use std::path::PathBuf;
use clap::{Parser, Subcommand};


#[derive(Debug, Parser)]
#[clap(name = "your_bittorrent", version = "0.1.0", author = "Your Name")]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Debug, Subcommand)]
enum SubCommand {
    Decode{
        #[clap(name = "ENCODED_VALUE")]
        encoded_value: String,
    },
    Info{
        #[clap(name = "TORRENT_FILE")]
        torrent_file: PathBuf,
    },
    Peers{
        #[clap(name = "TORRENT_FILE")]
        torrent_file: PathBuf,
    },
    Handshake{
        #[clap(name = "TORRENT_FILE")]
        torrent_file: PathBuf,
    },
    #[clap(name = "download_piece")]
    DownloadPiece {
        #[arg(short = 'o', default_value = "/tmp/test-piece-0")]
        output: PathBuf,
        torrent_file: PathBuf,
        #[arg(default_value = "0")]
        piece_index: usize,
    }
}

#[tokio::main]
async fn main() {
    let opts: Opts = Opts::parse();
    let command = opts.subcmd;
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    // println!("Logs from your program will appear here!");

    match command {
        // Usage: your_bittorrent.sh decode "<encoded_value>"
        SubCommand::Decode{encoded_value} => {
            let (_, decoded_value) = decode_bencoded_value(encoded_value);
            let json_value = serde_json::Value::from(decoded_value);
            println!("{}", json_value);
        }
        // Usage: your_bittorrent.sh info "<torrent_file>"
        SubCommand::Info{torrent_file} => {
            let metainfo = MetainfoFile::read_from_file(torrent_file).unwrap();

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
        SubCommand::Peers{torrent_file} => {
            let metainfo = MetainfoFile::read_from_file(torrent_file).unwrap();

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
        SubCommand::Handshake{torrent_file} => {
            let metainfo = MetainfoFile::read_from_file(torrent_file).unwrap();

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
        SubCommand::DownloadPiece {
            output,
            torrent_file,
            piece_index,
        } => {
            let metainfo = MetainfoFile::read_from_file(torrent_file).unwrap();
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
            let selected_piece_hash = &info.piece_hash()[piece_index];
            let downloads = peer_stream
                .download_piece(piece_index as u32, &info.piece_length)
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
                println!("Piece {} downloaded to {}.", piece_index, output_str);
            } else {
                println!(
                    "Downloaded piece {} hash {} does not match expected hash {}.",
                    piece_index, downloaded_hash, selected_piece_hash
                );
            }
        }
    }
}
