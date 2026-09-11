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
fn history_request_and_response_preserve_query_metadata() {
    let request = v1::HistoryRequest {
        request_id: Some("req-1".to_owned()),
        symbols: vec![v1::Symbol {
            market: Some(v1::Market::A as i32),
            code: Some("000001".to_owned()),
        }],
        period: Some(v1::Period::D1 as i32),
        start_date: Some("2026-01-01".to_owned()),
        end_date: Some("2026-01-31".to_owned()),
        adjustment: Some(v1::Adjustment::Raw as i32),
        page_size: Some(500),
        page_token: None,
    };
    let request_frame = v1::TransportFrame {
        payload: Some(v1::transport_frame::Payload::HistoryRequest(request)),
    };
    let decoded_request =
        v1::TransportFrame::decode(request_frame.encode_to_vec().as_slice()).unwrap();
    let Some(v1::transport_frame::Payload::HistoryRequest(request)) = decoded_request.payload
    else {
        panic!("expected history request");
    };
    assert_eq!(request.request_id.as_deref(), Some("req-1"));
    assert_eq!(request.symbols[0].code.as_deref(), Some("000001"));
    assert_eq!(request.adjustment, Some(v1::Adjustment::Raw as i32));

    let response = v1::HistoryResponse {
        payload: Some(v1::history_response::Payload::Chunk(v1::HistoryChunk {
            request_id: Some("req-1".to_owned()),
            records: vec![v1::HistoryRecord {
                symbol: Some(v1::Symbol {
                    market: Some(v1::Market::A as i32),
                    code: Some("000001".to_owned()),
                }),
                trade_date: Some("2026-01-02".to_owned()),
                kline: Some(v1::Kline {
                    ts: Some(1_767_283_200_000),
                    period: Some(v1::Period::D1 as i32),
                    open: None,
                    high: Some(10.5),
                    low: Some(9.5),
                    close: Some(10.0),
                    volume: Some(100.0),
                    amount: None,
                    turnover: None,
                    pre_close: None,
                }),
            }],
            next_page_token: Some("page-2".to_owned()),
            end: Some(false),
            snapshot_id: Some("snapshot-1".to_owned()),
            adjustment: Some(v1::Adjustment::Raw as i32),
        })),
    };
    let response_frame = v1::TransportFrame {
        payload: Some(v1::transport_frame::Payload::HistoryResponse(response)),
    };
    let decoded_response =
        v1::TransportFrame::decode(response_frame.encode_to_vec().as_slice()).unwrap();
    let Some(v1::transport_frame::Payload::HistoryResponse(response)) = decoded_response.payload
    else {
        panic!("expected history response");
    };
    let Some(v1::history_response::Payload::Chunk(chunk)) = response.payload else {
        panic!("expected history chunk");
    };
    assert_eq!(chunk.records.len(), 1);
    assert_eq!(chunk.records[0].trade_date.as_deref(), Some("2026-01-02"));
    assert_eq!(chunk.next_page_token.as_deref(), Some("page-2"));
    assert_eq!(chunk.end, Some(false));
}

#[test]
fn history_error_is_a_response_variant() {
    let frame = v1::TransportFrame {
        payload: Some(v1::transport_frame::Payload::HistoryResponse(
            v1::HistoryResponse {
                payload: Some(v1::history_response::Payload::Error(v1::HistoryError {
                    request_id: Some("req-2".to_owned()),
                    code: Some("invalid_range".to_owned()),
                    message: Some("end date precedes start date".to_owned()),
                    retryable: Some(false),
                })),
            },
        )),
    };
    let decoded = v1::TransportFrame::decode(frame.encode_to_vec().as_slice()).unwrap();
    let Some(v1::transport_frame::Payload::HistoryResponse(response)) = decoded.payload else {
        panic!("expected history response");
    };
    assert!(matches!(
        response.payload,
        Some(v1::history_response::Payload::Error(_))
    ));
}
