//! WS 传输帧约定。
//!
//! # 三类帧
//!
//! - **消息帧**：`MarketMessage` 的 JSON（`{"type":"quote",...}`），
//!   spider → webserver 的主数据流。
//! - **heartbeat 帧**：`{"type":"heartbeat"}`，服务器周期发送，客户端收到后
//!   重置连接超时计时。间隔可配置，默认值见 [`DEFAULT_HEARTBEAT_INTERVAL_SECS`]。
//! - **resume 帧**（客户端 → 服务器）：`{"type":"resume","since_ts":123,"since_serial":456}`，
//!   客户端断线重连后请求从游标续推。
//!
//! heartbeat/resume 是传输层控制帧，不是行情消息，因此不能塞进
//! `MarketMessage` enum。frame 层用顶层 `type` 字段区分“控制帧 vs 消息帧”，
//! 由 [`is_control_frame`] 分拣。
//!
//! # 断线重连续推语义（`(ts, serial)` 双键游标）
//!
//! - 客户端持续记录**最后收到的 `(ts, serial)`**。
//! - 断线重连后先发 `resume {since_ts, since_serial}`。
//! - 服务器从游标之后续推：**`ts > since_ts` 的全部消息**，以及
//!   **`ts == since_ts && serial > since_serial` 的 Tick**。
//! - **流全序约束（服务器责任）**：服务器必须按 `(ts, serial)` 全序推送；
//!   非 Tick 消息视为位于 `(ts, 0)`。即同一 `ts` 内，Quote/Kline/QuoteSnapshot
//!   必须先于该秒的 Tick 发送。没有这条约束，游标后的同 `ts` 非 Tick 可能被
//!   续推过滤跳过，造成 K 线缺口。
//! - 续推容错：若接入方无法保证上述全序，宁可把同 `ts` 的非 Tick 也重推
//!   （幂等无害，Kline 按 `ts` 去重），也不要让它们成为缺口。
//! - 语义分界：`Tick` 用 `(ts, serial)` 精确定位（同一秒多笔成交 ts 重复，
//!   serial 保证不丢不重）；`Quote`/`QuoteSnapshot` 是“最新快照”，重复推送
//!   **幂等无害**（覆盖即可）；`Kline` 靠 `ts` 去重。因此续推的“无重复无缺口”
//!   保证主要针对 `Tick`，快照类消息允许以 ts 粒度重推。

/// 默认 heartbeat 间隔（秒）。线上可通过配置覆盖，不做硬编码契约。
pub const DEFAULT_HEARTBEAT_INTERVAL_SECS: u64 = 15;

/// 识别 JSON 文本是否为控制帧。
///
/// 返回 `Some("heartbeat")` / `Some("resume")`；行情消息或未知 `type` 返回
/// `None`。未知 `type` 留给 `MarketMessage` 反序列化 fail loud。
pub fn is_control_frame(json: &str) -> Option<&'static str> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    match value.get("type")?.as_str()? {
        "heartbeat" => Some("heartbeat"),
        "resume" => Some("resume"),
        _ => None,
    }
}

/// 生成 heartbeat 帧。
pub fn heartbeat_json() -> String {
    r#"{"type":"heartbeat"}"#.to_owned()
}

/// 生成 resume 帧。
pub fn resume_json(since_ts: i64, since_serial: i64) -> String {
    format!(r#"{{"type":"resume","since_ts":{since_ts},"since_serial":{since_serial}}}"#)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::MarketMessage;
    use crate::types::{Kline, Level, Period, Quote, QuoteSnapshot, Tick};
    use crate::{Market, Symbol};

    fn sample_market_message() -> MarketMessage {
        MarketMessage::Quote {
            symbol: Symbol::new(Market::A, "560010"),
            data: Quote {
                ts: 1_787_904_896_000,
                price: 3.208,
                open: 3.211,
                high: 3.245,
                low: 3.204,
                pre_close: 3.220,
                volume: 132_886_000.0,
                amount: 428_272_326.0,
                in_vol: None,
                out_vol: None,
            },
        }
    }

    #[test]
    fn heartbeat_json_is_exact_and_recognized() {
        let json = heartbeat_json();
        assert_eq!(json, r#"{"type":"heartbeat"}"#);
        assert_eq!(is_control_frame(&json), Some("heartbeat"));
    }

    #[test]
    fn resume_json_is_exact_and_recognized() {
        let json = resume_json(123, 456);
        assert_eq!(
            json,
            r#"{"type":"resume","since_ts":123,"since_serial":456}"#
        );
        assert_eq!(is_control_frame(&json), Some("resume"));
    }

    #[test]
    fn market_message_is_not_control_frame() {
        let json = serde_json::to_string(&sample_market_message()).unwrap();
        assert_eq!(is_control_frame(&json), None);
    }

    #[test]
    fn unknown_type_is_not_control_frame_and_market_parse_fails_loud() {
        let json = r#"{"type":"bogus"}"#;
        assert_eq!(is_control_frame(json), None);
        assert!(serde_json::from_str::<MarketMessage>(json).is_err());
    }

    #[test]
    fn malformed_json_is_not_control_frame() {
        assert_eq!(is_control_frame("not json"), None);
        assert_eq!(is_control_frame(""), None);
    }

    #[test]
    fn control_frames_do_not_parse_as_market_message() {
        assert!(serde_json::from_str::<MarketMessage>(&heartbeat_json()).is_err());
        assert!(serde_json::from_str::<MarketMessage>(&resume_json(1, 2)).is_err());
    }

    #[test]
    fn other_market_variants_are_not_control_frames() {
        let messages = vec![
            MarketMessage::Tick {
                symbol: Symbol::new(Market::Tw, "2330"),
                data: Tick {
                    ts: 0,
                    price: 1.0,
                    volume: 1.0,
                    buy_price: None,
                    sell_price: None,
                    inout_flag: None,
                    serial: 1,
                },
            },
            MarketMessage::Kline {
                symbol: Symbol::new(Market::Tw, "1437"),
                data: Kline {
                    ts: 0,
                    period: Period::D1,
                    open: 1.0,
                    high: 1.0,
                    low: 1.0,
                    close: 1.0,
                    volume: 1.0,
                    amount: 1.0,
                    turnover: None,
                    pre_close: None,
                },
            },
            MarketMessage::QuoteSnapshot {
                symbol: Symbol::new(Market::A, "560010"),
                data: QuoteSnapshot {
                    ts: 0,
                    bids: vec![Level {
                        price: 1.0,
                        volume: 1.0,
                    }],
                    asks: vec![],
                },
            },
        ];
        for msg in messages {
            let json = serde_json::to_string(&msg).unwrap();
            assert_eq!(is_control_frame(&json), None);
        }
    }
}
