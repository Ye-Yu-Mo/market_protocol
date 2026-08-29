//! M3 集成测试：内存交换验证协议管道。
//!
//! 用 `Vec<String>` 模拟 WS 帧流，不做真实网络：
//! spider 端生成样本帧流 → webserver 端解析 → fan-out 到订阅者。
//! 断线续推使用 `(ts, serial)` 双键游标，Tick 精确、快照类幂等。

use market_protocol::Message;
use market_protocol::frame::{heartbeat_json, is_control_frame, resume_json};
use market_protocol::message::MarketMessage;
use market_protocol::types::{Kline, Level, Period, Quote, QuoteSnapshot, Tick};
use market_protocol::{Market, Symbol};

fn a_symbol() -> Symbol {
    Symbol::new(Market::A, "560010")
}

fn tw_symbol(code: &str) -> Symbol {
    Symbol::new(Market::Tw, code)
}

fn sample_quote(ts: i64) -> Quote {
    Quote {
        ts,
        price: 3.208,
        open: 3.211,
        high: 3.245,
        low: 3.204,
        pre_close: 3.220,
        volume: 132_886_000.0,
        amount: 428_272_326.0,
        in_vol: Some(69_090_700.0),
        out_vol: Some(63_795_300.0),
    }
}

fn sample_tick(ts: i64, serial: i64) -> Tick {
    Tick {
        ts,
        price: 1420.0,
        volume: 2_000.0,
        buy_price: Some(1420.0),
        sell_price: Some(1425.0),
        inout_flag: Some(1),
        serial,
    }
}

fn sample_kline(ts: i64) -> Kline {
    Kline {
        ts,
        period: Period::D1,
        open: 32.3,
        high: 32.30,
        low: 31.70,
        close: 31.8,
        volume: 70_951.0,
        amount: 2_260_790.0,
        turnover: Some(74.0),
        pre_close: Some(32.0),
    }
}

fn sample_snapshot(ts: i64) -> QuoteSnapshot {
    QuoteSnapshot {
        ts,
        bids: vec![Level {
            price: 3.207,
            volume: 10_000.0,
        }],
        asks: vec![Level {
            price: 3.208,
            volume: 5_000.0,
        }],
    }
}

/// 构造一段有代表性的样本流：
/// A 股 Quote、台股 2330 连续多笔 Tick（同 ts 靠 serial 区分）、
/// 台股 Kline、A 股 QuoteSnapshot。
fn sample_messages() -> Vec<MarketMessage> {
    vec![
        MarketMessage::Quote {
            symbol: a_symbol(),
            data: sample_quote(1_000),
        },
        MarketMessage::Tick {
            symbol: tw_symbol("2330"),
            data: sample_tick(2_000, 1),
        },
        MarketMessage::Tick {
            symbol: tw_symbol("2330"),
            data: sample_tick(2_000, 2),
        },
        MarketMessage::Tick {
            symbol: tw_symbol("2330"),
            data: sample_tick(2_000, 3),
        },
        MarketMessage::Kline {
            symbol: tw_symbol("1437"),
            data: sample_kline(3_000),
        },
        MarketMessage::QuoteSnapshot {
            symbol: a_symbol(),
            data: sample_snapshot(4_000),
        },
    ]
}

fn to_frames(messages: &[MarketMessage]) -> Vec<String> {
    messages
        .iter()
        .map(|m| serde_json::to_string(m).expect("serialize MarketMessage"))
        .collect()
}

fn parse_frames(frames: &[String]) -> Vec<MarketMessage> {
    frames
        .iter()
        .map(|f| serde_json::from_str(f).expect("parse MarketMessage frame"))
        .collect()
}

type DynSink<'a> = &'a mut dyn FnMut(&dyn Message);
type EnumSink<'a> = &'a mut dyn FnMut(&MarketMessage);

/// 统一消费异构订阅者：`&dyn Message` 版本（trait 裁决：保留）。
fn fanout_dyn(msg: &dyn Message, subscribers: &mut [DynSink<'_>]) {
    for subscriber in subscribers {
        subscriber(msg);
    }
}

/// 直接消费 enum 版本（反序列化入口和需要具体类型时使用）。
fn fanout_enum(msg: &MarketMessage, subscribers: &mut [EnumSink<'_>]) {
    for subscriber in subscribers {
        subscriber(msg);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Cursor {
    ts: i64,
    serial: i64,
}

fn cursor_of(msg: &MarketMessage) -> Cursor {
    let serial = match msg {
        MarketMessage::Tick { data, .. } => data.serial,
        _ => 0,
    };
    Cursor {
        ts: msg.timestamp(),
        serial,
    }
}

/// 服务器端续推过滤：Tick 用 `(ts, serial)` 精确定位；
/// 非 Tick 用 `ts >= since_ts`——宁可重推同 ts 的快照/Kline（幂等），
/// 也不允许同 ts 非 Tick 因“流顺序不理想”变成缺口。
fn resume_after(messages: &[MarketMessage], cursor: Cursor) -> Vec<MarketMessage> {
    messages
        .iter()
        .filter(|m| match m {
            MarketMessage::Tick { data, .. } => {
                data.ts > cursor.ts || (data.ts == cursor.ts && data.serial > cursor.serial)
            }
            _ => m.timestamp() >= cursor.ts,
        })
        .cloned()
        .collect()
}

#[test]
fn normal_pipeline_delivers_all_messages_to_both_subscribers() {
    let messages = sample_messages();
    let frames = to_frames(&messages);

    // webserver 端逐帧解析。
    let parsed = parse_frames(&frames);
    assert_eq!(parsed, messages);

    // fan-out 到两个订阅者，顺序与类型正确。
    let mut subscriber_a = Vec::new();
    let mut subscriber_b = Vec::new();
    for msg in &parsed {
        fanout_dyn(
            msg,
            &mut [
                &mut |m: &dyn Message| {
                    subscriber_a.push((
                        m.kind().to_string(),
                        m.symbol().code.clone(),
                        m.timestamp(),
                    ));
                },
                &mut |m: &dyn Message| {
                    subscriber_b.push((
                        m.kind().to_string(),
                        m.symbol().code.clone(),
                        m.timestamp(),
                    ));
                },
            ],
        );
    }

    let expected: Vec<_> = parsed
        .iter()
        .map(|m| (m.kind().to_string(), m.symbol().code.clone(), m.timestamp()))
        .collect();
    assert_eq!(subscriber_a, expected);
    assert_eq!(subscriber_b, expected);

    // enum 版本同样能把全部消息分发给订阅者。
    let mut enum_sink = Vec::new();
    for msg in &parsed {
        fanout_enum(
            msg,
            &mut [&mut |m: &MarketMessage| {
                enum_sink.push(m.kind().to_string());
            }],
        );
    }
    assert_eq!(
        enum_sink,
        expected
            .iter()
            .map(|(k, _, _)| k.clone())
            .collect::<Vec<_>>()
    );
}

#[test]
fn resume_after_disconnect_has_no_duplicates_and_no_gaps_for_ticks() {
    let all = sample_messages();
    let frames = to_frames(&all);

    // 订阅者 A 消费前 3 帧（Quote + Tick serial 1/2）后断开。
    let consumed_frames: Vec<String> = frames.iter().take(3).cloned().collect();
    let consumed = parse_frames(&consumed_frames);
    let cursor = cursor_of(consumed.last().expect("consumed non-empty"));

    // A 重连后发送 resume，服务器从游标之后续推。
    let resume = resume_json(cursor.ts, cursor.serial);
    assert_eq!(
        resume,
        format!(
            r#"{{"type":"resume","since_ts":{},"since_serial":{}}}"#,
            cursor.ts, cursor.serial
        )
    );

    let resumed = resume_after(&all, cursor);

    // 续推必须只含 Tick serial=3 及之后的消息。
    let kinds: Vec<_> = resumed.iter().map(|m| m.kind()).collect();
    assert_eq!(kinds, vec!["tick", "kline", "quote_snapshot"]);
    if let MarketMessage::Tick { data, .. } = &resumed[0] {
        assert_eq!(data.serial, 3);
    } else {
        panic!("first resumed message should be Tick serial 3");
    }

    // 已消费 + 续推 = 原始全量，证明无重复、无缺口。
    let mut combined = consumed;
    combined.extend(resumed);
    assert_eq!(combined, all);
}

#[test]
fn resume_does_not_skip_same_ts_non_tick_even_with_imperfect_order() {
    // 即使接入方没有严格按 (ts, serial) 全序推送，同 ts 的 Kline 也不能成为缺口。
    let messages = vec![
        MarketMessage::Quote {
            symbol: a_symbol(),
            data: sample_quote(2_000),
        },
        MarketMessage::Tick {
            symbol: tw_symbol("2330"),
            data: sample_tick(2_000, 1),
        },
        MarketMessage::Kline {
            symbol: tw_symbol("1437"),
            data: sample_kline(2_000),
        },
        MarketMessage::Tick {
            symbol: tw_symbol("2330"),
            data: sample_tick(2_000, 2),
        },
    ];

    // 订阅者消费到 Tick(2000,1) 断开。
    let consumed = &messages[..2];
    let cursor = cursor_of(consumed.last().expect("consumed non-empty"));
    let resumed = resume_after(&messages, cursor);

    // Kline(2000) 必须被重推，不能因为 ts == since_ts 且非 Tick 就被跳过。
    assert!(
        resumed
            .iter()
            .any(|m| matches!(m, MarketMessage::Kline { data, .. } if data.ts == 2_000))
    );
    assert_eq!(
        resumed
            .iter()
            .filter(|m| matches!(m, MarketMessage::Tick { .. }))
            .count(),
        1
    );
    if let Some(MarketMessage::Tick { data, .. }) = resumed.last() {
        assert_eq!(data.serial, 2);
    } else {
        panic!("last resumed message should be Tick serial 2");
    }
}

#[test]
fn resume_at_stream_start_returns_full_history() {
    let all = sample_messages();
    let resumed = resume_after(&all, Cursor { ts: 0, serial: 0 });
    assert_eq!(resumed, all);
}

#[test]
fn resume_at_stream_end_has_no_new_messages() {
    let all = sample_messages();
    let last = all.last().expect("sample stream non-empty");
    let cursor = cursor_of(last);
    let resumed = resume_after(&all, cursor);

    // 同 ts 的非 Tick 允许幂等重推；但不得出现任何“新”的 Tick 或更晚消息。
    assert!(resumed.iter().all(|m| m.timestamp() == cursor.ts));
    assert!(
        resumed
            .iter()
            .all(|m| !matches!(m, MarketMessage::Tick { .. }))
    );
}

#[test]
fn snapshot_duplicate_at_resume_boundary_is_idempotent() {
    // Quote 是“最新快照”，即使同一 ts 被重复推送，也只是覆盖，终态一致。
    let quote = sample_messages()[0].clone();
    let quote_frame = serde_json::to_string(&quote).unwrap();
    let frames = vec![quote_frame.clone(), quote_frame];

    let parsed = parse_frames(&frames);
    let mut state: Option<Quote> = None;
    for msg in &parsed {
        match msg {
            MarketMessage::Quote { data, .. } => state = Some(data.clone()),
            _ => panic!("expected only quote frames"),
        }
    }

    assert_eq!(state.as_ref().map(|q| q.price), Some(3.208));
    if let MarketMessage::Quote { data, .. } = &quote {
        assert_eq!(state.as_ref(), Some(data));
    }
}

#[test]
fn control_frames_are_not_market_messages() {
    assert_eq!(
        is_control_frame(r#"{"type":"heartbeat"}"#),
        Some("heartbeat")
    );
    assert_eq!(is_control_frame(&heartbeat_json()), Some("heartbeat"));
    assert_eq!(is_control_frame(&resume_json(123, 456)), Some("resume"));

    // 行情帧不是控制帧。
    for frame in to_frames(&sample_messages()) {
        assert_eq!(is_control_frame(&frame), None);
    }

    // 控制帧不能反序列化为 MarketMessage。
    assert!(serde_json::from_str::<MarketMessage>(&heartbeat_json()).is_err());
    assert!(serde_json::from_str::<MarketMessage>(&resume_json(1, 2)).is_err());
}

#[test]
fn unknown_type_still_fails_loud_as_market_message() {
    assert_eq!(is_control_frame(r#"{"type":"bogus"}"#), None);
    let result: Result<MarketMessage, _> = serde_json::from_str(r#"{"type":"bogus"}"#);
    assert!(result.is_err());
}

#[test]
fn empty_frame_stream_is_valid() {
    let frames: Vec<String> = Vec::new();
    assert!(parse_frames(&frames).is_empty());
}
