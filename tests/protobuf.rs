use market_protocol::v1;
use prost::Message;

fn quote() -> v1::Quote {
    v1::Quote {
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
    }
}

fn market_frame() -> v1::TransportFrame {
    v1::TransportFrame {
        payload: Some(v1::transport_frame::Payload::Market(v1::MarketEnvelope {
            symbol: Some(v1::Symbol {
                market: Some(v1::Market::A as i32),
                code: Some("560010".to_owned()),
            }),
            payload: Some(v1::market_envelope::Payload::Quote(quote())),
        })),
    }
}

fn hex_bytes(value: &str) -> Vec<u8> {
    value
        .trim()
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = (pair[0] as char).to_digit(16).unwrap();
            let low = (pair[1] as char).to_digit(16).unwrap();
            ((high << 4) | low) as u8
        })
        .collect()
}

#[test]
fn rust_fixture_is_stable_for_the_pinned_encoder() {
    let expected = hex_bytes(include_str!("fixtures/quote_transport.hex"));
    assert_eq!(market_frame().encode_to_vec(), expected);
}

#[test]
fn market_data_round_trips_through_transport_frame() {
    let encoded = market_frame().encode_to_vec();
    let decoded = v1::TransportFrame::decode(encoded.as_slice()).unwrap();

    let Some(v1::transport_frame::Payload::Market(envelope)) = decoded.payload else {
        panic!("expected market transport payload");
    };
    assert_eq!(envelope.symbol.unwrap().code.as_deref(), Some("560010"));
    let Some(v1::market_envelope::Payload::Quote(quote)) = envelope.payload else {
        panic!("expected quote payload");
    };
    assert_eq!(quote.price, Some(3.208));
    assert_eq!(quote.in_vol, Some(69_090_700.0));
}

#[test]
fn all_market_payloads_use_the_same_envelope() {
    let symbol = v1::Symbol {
        market: Some(v1::Market::Tw as i32),
        code: Some("2330".to_owned()),
    };
    let payloads = [
        v1::market_envelope::Payload::Quote(quote()),
        v1::market_envelope::Payload::Tick(v1::Tick {
            ts: Some(1),
            price: Some(2.0),
            volume: Some(3.0),
            buy_price: None,
            sell_price: None,
            inout_flag: None,
            serial: Some(4),
        }),
        v1::market_envelope::Payload::Kline(v1::Kline {
            ts: Some(1),
            period: Some(v1::Period::D1 as i32),
            open: Some(1.0),
            high: Some(1.0),
            low: Some(1.0),
            close: Some(1.0),
            volume: Some(1.0),
            amount: Some(1.0),
            turnover: None,
            pre_close: None,
        }),
        v1::market_envelope::Payload::QuoteSnapshot(v1::QuoteSnapshot {
            ts: Some(1),
            bids: vec![],
            asks: vec![],
        }),
    ];

    for payload in payloads {
        let frame = v1::TransportFrame {
            payload: Some(v1::transport_frame::Payload::Market(v1::MarketEnvelope {
                symbol: Some(symbol.clone()),
                payload: Some(payload),
            })),
        };
        let decoded = v1::TransportFrame::decode(frame.encode_to_vec().as_slice()).unwrap();
        assert!(matches!(
            decoded.payload,
            Some(v1::transport_frame::Payload::Market(_))
        ));
    }
}

#[test]
fn control_frames_are_binary_oneof_variants() {
    let heartbeat = v1::TransportFrame {
        payload: Some(v1::transport_frame::Payload::Heartbeat(v1::Heartbeat {})),
    };
    assert_eq!(
        v1::TransportFrame::decode(heartbeat.encode_to_vec().as_slice())
            .unwrap()
            .payload
            .map(|payload| match payload {
                v1::transport_frame::Payload::Heartbeat(_) => "heartbeat",
                _ => "other",
            }),
        Some("heartbeat")
    );

    let resume = v1::TransportFrame {
        payload: Some(v1::transport_frame::Payload::Resume(v1::Resume {
            since_ts: Some(123),
            since_serial: Some(456),
        })),
    };
    let decoded = v1::TransportFrame::decode(resume.encode_to_vec().as_slice()).unwrap();
    let Some(v1::transport_frame::Payload::Resume(resume)) = decoded.payload else {
        panic!("expected resume payload");
    };
    assert_eq!(resume.since_ts, Some(123));
    assert_eq!(resume.since_serial, Some(456));
}

#[test]
fn optional_fields_preserve_missing_and_explicit_zero() {
    let missing = v1::Quote::default();
    assert_eq!(missing.price, None);

    let explicit_zero = v1::Quote {
        price: Some(0.0),
        ..Default::default()
    };
    assert_eq!(explicit_zero.price, Some(0.0));
}

#[test]
fn malformed_wire_is_rejected() {
    assert!(v1::TransportFrame::decode([0x80].as_slice()).is_err());
}
