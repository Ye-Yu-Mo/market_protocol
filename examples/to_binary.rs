//! Encode a Protobuf TransportFrame as hexadecimal bytes.

use market_protocol::v1;
use prost::Message as _;

fn main() {
    let quote = v1::Quote {
        ts: Some(1_787_904_896_000),
        price: Some(3.208),
        open: Some(3.211),
        high: Some(3.245),
        low: Some(3.204),
        pre_close: Some(3.220),
        volume: Some(132_886_000.0),
        amount: Some(428_272_326.0),
        in_vol: Some(69_090_700.0),
        out_vol: Some(63_795_300.0),
    };
    let envelope = v1::MarketEnvelope {
        symbol: Some(v1::Symbol {
            market: Some(v1::Market::A as i32),
            code: Some("560010".to_owned()),
        }),
        payload: Some(v1::market_envelope::Payload::Quote(quote)),
    };
    let frame = v1::TransportFrame {
        payload: Some(v1::transport_frame::Payload::Market(envelope)),
    };

    let bytes = frame.encode_to_vec();
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    println!("{hex}");
}
