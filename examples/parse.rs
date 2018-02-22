extern crate clap;
extern crate mpeg2ts;
#[macro_use]
extern crate trackable;

use std::io::Write;
use clap::{App, Arg};
use mpeg2ts::pes::{PesPacketReader, ReadPesPacket};
use mpeg2ts::ts::{ReadTsPacket, TsPacketReader};
use trackable::error::Failure;

fn main() {
    let matches = App::new("parse")
        .arg(
            Arg::with_name("OUTPUT_TYPE")
                .long("output-type")
                .takes_value(true)
                .possible_values(&["ts", "pes", "es-audio", "es-video"])
                .default_value("ts"),
        )
        .get_matches();
    match matches.value_of("OUTPUT_TYPE").unwrap() {
        "ts" => {
            let mut reader = TsPacketReader::new(std::io::stdin());
            while let Some(packet) = track_try_unwrap!(reader.read_ts_packet()) {
                println!("{:?}", packet);
            }
        }
        "pes" => {
            let mut reader = PesPacketReader::new(TsPacketReader::new(std::io::stdin()));
            while let Some(packet) = track_try_unwrap!(reader.read_pes_packet()) {
                println!("{:?} {} bytes", packet.header, packet.data.len());
            }
        }
        "es-audio" => {
            let mut reader = PesPacketReader::new(TsPacketReader::new(std::io::stdin()));
            while let Some(packet) = track_try_unwrap!(reader.read_pes_packet()) {
                if !packet.header.stream_id.is_audio() {
                    continue;
                }
                track_try_unwrap!(
                    std::io::stdout()
                        .write_all(&packet.data)
                        .map_err(Failure::from_error)
                );
            }
        }
        "es-video" => {
            let mut reader = PesPacketReader::new(TsPacketReader::new(std::io::stdin()));
            while let Some(packet) = track_try_unwrap!(reader.read_pes_packet()) {
                if !packet.header.stream_id.is_video() {
                    continue;
                }
                track_try_unwrap!(
                    std::io::stdout()
                        .write_all(&packet.data)
                        .map_err(Failure::from_error)
                );
            }
        }
        _ => unreachable!(),
    }
}
