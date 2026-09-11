//! Decode a hexadecimal Protobuf TransportFrame.

use market_protocol::v1;
use prost::Message as _;

fn main() {
    let hex = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: from_binary <hexadecimal-protobuf-frame>");
        std::process::exit(2);
    });

    let bytes = match decode_hex(&hex) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("invalid binary frame hex: {error}");
            std::process::exit(2);
        }
    };

    let frame = match v1::TransportFrame::decode(bytes.as_slice()) {
        Ok(frame) => frame,
        Err(error) => {
            eprintln!("cannot decode binary frame: {error}");
            std::process::exit(1);
        }
    };

    match frame.payload {
        Some(v1::transport_frame::Payload::Market(envelope)) => {
            let symbol = envelope
                .symbol
                .as_ref()
                .and_then(|value| value.code.as_deref());
            let kind = match envelope.payload {
                Some(v1::market_envelope::Payload::Quote(_)) => "quote",
                Some(v1::market_envelope::Payload::Tick(_)) => "tick",
                Some(v1::market_envelope::Payload::Kline(_)) => "kline",
                Some(v1::market_envelope::Payload::QuoteSnapshot(_)) => "quote_snapshot",
                None => "missing_payload",
            };
            println!("kind={kind} symbol={symbol:?}");
        }
        Some(v1::transport_frame::Payload::Heartbeat(_)) => println!("kind=heartbeat"),
        Some(v1::transport_frame::Payload::Resume(resume)) => println!(
            "kind=resume since_ts={:?} since_serial={:?}",
            resume.since_ts, resume.since_serial
        ),
        None => {
            eprintln!("binary frame has no payload");
            std::process::exit(1);
        }
    }
}

fn decode_hex(hex: &str) -> Result<Vec<u8>, &'static str> {
    if hex.is_empty() {
        return Err("frame is empty");
    }
    if hex.len() % 2 != 0 {
        return Err("hex input must have an even number of characters");
    }

    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = hex_digit(pair[0]).ok_or("invalid hex byte")?;
            let low = hex_digit(pair[1]).ok_or("invalid hex byte")?;
            Ok((high << 4) | low)
        })
        .collect()
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
