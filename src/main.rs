use bittorrent_starter_rust::decoder::decode_bencoded_value;
use bittorrent_starter_rust::file::{Info, MetainfoFile};
use bittorrent_starter_rust::network::{ping_tracker, PeerMessage, PeerStream};
use clap::{Parser, Subcommand};
use std::io::Write;
use std::{net::SocketAddrV4, path::PathBuf};

#[derive(Debug, Parser)]
#[clap(
    name = "your_bittorrent",
    version = "0.1.0",
    author = "Loc Nguyen",
    about = "A BitTorrent client written in Rust."
)]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Debug, Subcommand)]
enum SubCommand {
    Decode {
        #[clap(name = "ENCODED_VALUE")]
        encoded_value: String,
    },
    Info {
        #[clap(name = "TORRENT_FILE")]
        torrent_file: PathBuf,
    },
    Peers {
        #[clap(name = "TORRENT_FILE")]
        torrent_file: PathBuf,
    },
    Handshake {
        #[clap(name = "TORRENT_FILE")]
        torrent_file: PathBuf,
        peer_ip: SocketAddrV4,
    },
    #[clap(name = "download_piece")]
    DownloadPiece {
        #[arg(short = 'o', default_value = "/tmp/test-piece-0")]
        output: PathBuf,
        torrent_file: PathBuf,
        #[arg(default_value = "0")]
        piece_index: usize,
    },
    Download {
        #[arg(short = 'o', default_value = "/tmp/test-piece-0")]
        output: PathBuf,
        torrent_file: PathBuf,
    },
}

#[tokio::main]
async fn main() {
    let opts: Opts = Opts::parse();
    let command = opts.subcmd;
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    // println!("Logs from your program will appear here!");

    match command {
        // Usage: your_bittorrent.sh decode "<encoded_value>"
        SubCommand::Decode { encoded_value } => {
            let (_, decoded_value) = decode_bencoded_value(encoded_value);
            let json_value = serde_json::Value::from(decoded_value);
            println!("{}", json_value);
        }
        // Usage: your_bittorrent.sh info "<torrent_file>"
        SubCommand::Info { torrent_file } => {
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
        SubCommand::Peers { torrent_file } => {
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
        SubCommand::Handshake {
            torrent_file,
            peer_ip,
        } => {
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
            // Check that peer_ip is in peers
            assert!(peers.contains(&peer_ip), "Peer IP not in peers.");

            let mut peer_stream = PeerStream::new(peer_ip);

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
            // Prepare the peer stream
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

            match peer_stream.prep_download(&info.info_hash()) {
                Ok(prepped) => {
                    println!("Prepped: {:?}", prepped);
                }
                Err(e) => {
                    println!("Prepped: Error: {}", e);
                }
            }

            // Chunk pieces into 16 * 1024 byte chunks with index
            // then download each chunk
            let piece_hashes = info.piece_hash();
            let piece_length = if piece_index == piece_hashes.len() - 1 {
                info.length - (piece_index as i64 * info.piece_length)
            } else {
                info.piece_length
            };
            println!(
                "Downloading piece {}/{} (length {})",
                piece_index + 1,
                piece_hashes.len(),
                piece_length,
            );
            let downloads = peer_stream
                .download_piece(piece_index as u32, &piece_length)
                .unwrap();
            // Zip the downloads with the piece hashes & map to download::save_piece into /tmp/test-piece-{idx}
            let downloaded_payload: Vec<u8> =
                downloads
                    .iter()
                    .enumerate()
                    .fold(vec![], |mut acc, (_index, download)| {
                        match download {
                            PeerMessage::Piece {
                                index: _,
                                begin: _,
                                block,
                            } => {
                                acc.extend_from_slice(block);
                            }
                            _ => {
                                panic!("Expected Piece message, got {:?}", download);
                            }
                        }
                        acc
                    });
            assert_eq!(
                downloaded_payload.len(),
                piece_length as usize,
                "Downloaded payload length {} does not match expected length {}.",
                downloaded_payload.len(),
                piece_length
            );
            let verified = info.verify_piece(piece_index, &downloaded_payload);
            if verified {
                // Save the piece to /tmp/test-piece-{idx}
                std::fs::write(&output, downloaded_payload).unwrap();
                let output_str = output.to_str().unwrap();
                println!("Piece {} downloaded to {}.", piece_index, output_str);
            } else {
                panic!("Downloaded piece failed verification.");
            }
        }
        SubCommand::Download {
            output,
            torrent_file,
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

            match peer_stream.prep_download(&info.info_hash()) {
                Ok(prepped) => {
                    println!("Prepped: {:?}", prepped);
                }
                Err(e) => {
                    println!("Prepped: Error: {}", e);
                }
            }

            // Download all the pieces
            let all_downloads: Vec<Vec<PeerMessage>> = (0..info.piece_hash().len())
                .map(|piece_index| {
                    let piece_hashes = info.piece_hash();
                    let piece_length = if piece_index == piece_hashes.len() - 1 {
                        info.length - (piece_index as i64 * info.piece_length)
                    } else {
                        info.piece_length
                    };
                    println!(
                        "Downloading piece {}/{} (length {})",
                        piece_index + 1,
                        piece_hashes.len(),
                        piece_length,
                    );
                    let downloads = peer_stream
                        .download_piece(piece_index as u32, &piece_length)
                        .unwrap();
                    downloads
                })
                .collect();

            // Combine the downloads into a single payload
            let downloaded_payloads: Vec<Vec<u8>> = all_downloads
                .iter()
                .map(|downloads| {
                    downloads
                        .iter()
                        .enumerate()
                        .fold(vec![], |mut acc, (_index, download)| {
                            match download {
                                PeerMessage::Piece {
                                    index: _,
                                    begin: _,
                                    block,
                                } => {
                                    acc.extend_from_slice(block);
                                }
                                _ => {
                                    panic!("Expected Piece message, got {:?}", download);
                                }
                            }
                            acc
                        })
                })
                .collect();

            // Verify the payload
            downloaded_payloads
                .iter()
                .enumerate()
                .all(
                    |(piece_index, payload)| match info.verify_piece(piece_index, payload) {
                        true => true,
                        false => {
                            println!("Piece {} failed verification.", piece_index);
                            panic!("Downloaded piece {} failed verification.", piece_index);
                        }
                    },
                );

            // Combine all the payload & save to output
            let mut output_file = std::fs::File::create(&output).unwrap();
            downloaded_payloads.iter().for_each(|payload| {
                output_file.write_all(payload).unwrap();
            });
            println!("Downloaded file saved to {}.", output.to_str().unwrap());
        }
    }
}
