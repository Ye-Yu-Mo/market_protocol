//! 统一行情消息 envelope。
//!
//! # envelope 契约
//!
//! - 序列化格式为 JSON，envelope 形如：
//!   `{"type":"quote","symbol":{...},"data":{...}}`。
//! - **顶层无 `ts`**：`ts` 由各消息类型自带（M1 已定，见 [`types`](crate::types)）。
//! - `type` 标签、`symbol`/`data` 字段名自 M2 起为不可改契约。
//!
//! # 前向兼容约定
//!
//! - 所有消息类型都保持 serde 默认行为，**不开启 `deny_unknown_fields`**：
//!   旧消费者解析含新字段的 JSON 时忽略未知字段。
//! - 未来给任何消息类型新增字段，字段必须是 `Option<T>` 或带 `#[serde(default)]`：
//!   新消费者解析旧消息时不失败。
//! - 遇到未知 `type` 标签时 `from_str::<MarketMessage>` 返回 `Err`（fail loud）。
//!   消费方收到未知类型应捕获 `Err` 并跳过，不得当作已知类型处理。

use crate::Symbol;
use crate::types::{Kline, Quote, QuoteSnapshot, Tick};
use serde::{Deserialize, Serialize};

/// 统一行情消息。
///
/// envelope 结构定案：`{"type": "...", "symbol": {...}, "data": {...}}`。
/// **顶层无 `ts`**——`ts` 由各消息类型自带（M1 已定，见 [`types`](crate::types)）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MarketMessage {
    Quote { symbol: Symbol, data: Quote },
    Tick { symbol: Symbol, data: Tick },
    Kline { symbol: Symbol, data: Kline },
    QuoteSnapshot { symbol: Symbol, data: QuoteSnapshot },
}

impl MarketMessage {
    /// 返回消息所属标的。
    pub fn symbol(&self) -> &Symbol {
        match self {
            Self::Quote { symbol, .. }
            | Self::Tick { symbol, .. }
            | Self::Kline { symbol, .. }
            | Self::QuoteSnapshot { symbol, .. } => symbol,
        }
    }

    /// 返回消息数据内的时间戳（epoch 毫秒）。
    pub fn timestamp(&self) -> i64 {
        match self {
            Self::Quote { data, .. } => data.ts,
            Self::Tick { data, .. } => data.ts,
            Self::Kline { data, .. } => data.ts,
            Self::QuoteSnapshot { data, .. } => data.ts,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Market;
    use crate::types::Period;

    fn a_symbol() -> Symbol {
        Symbol::new(Market::A, "560010")
    }

    fn tw_symbol(code: &str) -> Symbol {
        Symbol::new(Market::Tw, code)
    }

    fn sample_quote() -> Quote {
        Quote {
            ts: 1_787_904_896_000,
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

    fn sample_tick() -> Tick {
        Tick {
            ts: 1_787_887_504_611,
            price: 1420.0,
            volume: 2_000.0,
            buy_price: Some(1420.0),
            sell_price: Some(1425.0),
            inout_flag: Some(1),
            serial: 42,
        }
    }

    fn sample_kline() -> Kline {
        Kline {
            ts: 1_787_500_800_000,
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

    fn sample_snapshot() -> QuoteSnapshot {
        QuoteSnapshot {
            ts: 1_787_904_896_000,
            bids: vec![
                crate::types::Level {
                    price: 3.207,
                    volume: 10_000.0,
                },
                crate::types::Level {
                    price: 3.206,
                    volume: 20_000.0,
                },
            ],
            asks: vec![crate::types::Level {
                price: 3.208,
                volume: 5_000.0,
            }],
        }
    }

    #[test]
    fn quote_round_trip_uses_type_tag_quote() {
        let msg = MarketMessage::Quote {
            symbol: a_symbol(),
            data: sample_quote(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"quote""#));
        assert!(json.contains(r#""symbol":{"market":"A","code":"560010"}"#));
        let back: MarketMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, back);
    }

    #[test]
    fn tick_round_trip_uses_type_tag_tick() {
        let msg = MarketMessage::Tick {
            symbol: tw_symbol("2330"),
            data: sample_tick(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"tick""#));
        let back: MarketMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, back);
    }

    #[test]
    fn envelope_has_no_top_level_ts() {
        let msg = MarketMessage::Quote {
            symbol: a_symbol(),
            data: sample_quote(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(value.get("ts").is_none());
        assert!(value.get("data").and_then(|d| d.get("ts")).is_some());
    }

    #[test]
    fn kline_round_trip_uses_type_tag_kline() {
        let msg = MarketMessage::Kline {
            symbol: tw_symbol("1437"),
            data: sample_kline(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"kline""#));
        let back: MarketMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, back);
    }

    #[test]
    fn quote_snapshot_round_trip_uses_type_tag_quote_snapshot() {
        let msg = MarketMessage::QuoteSnapshot {
            symbol: a_symbol(),
            data: sample_snapshot(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"quote_snapshot""#));
        let back: MarketMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, back);
    }

    #[test]
    fn real_a_share_560010_quote_json_parses() {
        let json = r#"{
            "type": "quote",
            "symbol": {"market": "A", "code": "560010"},
            "data": {
                "ts": 1787904896000,
                "price": 3.208,
                "open": 3.211,
                "high": 3.245,
                "low": 3.204,
                "pre_close": 3.220,
                "volume": 132886000.0,
                "amount": 428272326.0,
                "in_vol": 69090700.0,
                "out_vol": 63795300.0
            }
        }"#;
        let msg: MarketMessage = serde_json::from_str(json).unwrap();
        match msg {
            MarketMessage::Quote { symbol, data } => {
                assert_eq!(symbol, a_symbol());
                assert_eq!(data.price, 3.208);
                assert_eq!(data.volume, 132_886_000.0);
                assert_eq!(data.in_vol, Some(69_090_700.0));
                assert_eq!(data.out_vol, Some(63_795_300.0));
            }
            _ => panic!("expected Quote"),
        }
    }

    #[test]
    fn real_tw_2330_quote_json_parses() {
        let json = r#"{
            "type": "quote",
            "symbol": {"market": "Tw", "code": "2330"},
            "data": {
                "ts": 1787887504611,
                "price": 1420.0,
                "open": 1415.0,
                "high": 1430.0,
                "low": 1410.0,
                "pre_close": 1410.0,
                "volume": 14992.0,
                "amount": 21289000.0,
                "in_vol": 7997.0,
                "out_vol": 6995.0
            }
        }"#;
        let msg: MarketMessage = serde_json::from_str(json).unwrap();
        match msg {
            MarketMessage::Quote { symbol, data } => {
                assert_eq!(symbol, tw_symbol("2330"));
                assert_eq!(data.price, 1420.0);
                assert_eq!(data.in_vol, Some(7_997.0));
                assert_eq!(data.out_vol, Some(6_995.0));
            }
            _ => panic!("expected Quote"),
        }
    }

    #[test]
    fn real_tw_1437_kline_json_parses() {
        let json = r#"{
            "type": "kline",
            "symbol": {"market": "Tw", "code": "1437"},
            "data": {
                "ts": 1787500800000,
                "period": "1d",
                "open": 32.3,
                "high": 32.30,
                "low": 31.70,
                "close": 31.8,
                "volume": 70951.0,
                "amount": 2260790.0,
                "turnover": 74.0,
                "pre_close": 32.0
            }
        }"#;
        let msg: MarketMessage = serde_json::from_str(json).unwrap();
        match msg {
            MarketMessage::Kline { symbol, data } => {
                assert_eq!(symbol, tw_symbol("1437"));
                assert_eq!(data.period, Period::D1);
                assert_eq!(data.turnover, Some(74.0));
                assert_eq!(data.pre_close, Some(32.0));
            }
            _ => panic!("expected Kline"),
        }
    }

    #[test]
    fn unknown_type_tag_fails_loud() {
        let result: Result<MarketMessage, _> = serde_json::from_str(
            r#"{"type":"kline_minute","symbol":{"market":"A","code":"560010"},"data":{}}"#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn unknown_fields_are_tolerated() {
        let json = r#"{
            "type": "quote",
            "symbol": {"market": "A", "code": "560010"},
            "data": {
                "ts": 1787904896000,
                "price": 3.208,
                "open": 3.211,
                "high": 3.245,
                "low": 3.204,
                "pre_close": 3.220,
                "volume": 132886000.0,
                "amount": 428272326.0,
                "in_vol": null,
                "out_vol": null,
                "extra_data": 456
            },
            "extra": 123
        }"#;
        let msg: MarketMessage = serde_json::from_str(json).unwrap();
        match msg {
            MarketMessage::Quote { data, .. } => assert_eq!(data.price, 3.208),
            _ => panic!("expected Quote"),
        }
    }

    #[test]
    fn missing_symbol_or_data_fails() {
        let missing_symbol: Result<MarketMessage, _> = serde_json::from_str(
            r#"{"type":"quote","data":{"ts":0,"price":1.0,"open":1.0,"high":1.0,"low":1.0,"pre_close":1.0,"volume":0.0,"amount":0.0}}"#,
        );
        assert!(missing_symbol.is_err());

        let missing_data: Result<MarketMessage, _> =
            serde_json::from_str(r#"{"type":"quote","symbol":{"market":"A","code":"560010"}}"#);
        assert!(missing_data.is_err());
    }

    #[test]
    fn optional_fields_missing_are_none() {
        let json = r#"{
            "type": "quote",
            "symbol": {"market": "A", "code": "560010"},
            "data": {
                "ts": 0,
                "price": 1.0,
                "open": 1.0,
                "high": 1.0,
                "low": 1.0,
                "pre_close": 1.0,
                "volume": 0.0,
                "amount": 0.0
            }
        }"#;
        let msg: MarketMessage = serde_json::from_str(json).unwrap();
        match msg {
            MarketMessage::Quote { data, .. } => {
                assert_eq!(data.in_vol, None);
                assert_eq!(data.out_vol, None);
            }
            _ => panic!("expected Quote"),
        }
    }

    #[test]
    fn accessors_return_symbol_and_data_ts() {
        let quote = MarketMessage::Quote {
            symbol: a_symbol(),
            data: sample_quote(),
        };
        assert_eq!(quote.symbol(), &a_symbol());
        assert_eq!(quote.timestamp(), 1_787_904_896_000);

        let tick = MarketMessage::Tick {
            symbol: tw_symbol("2330"),
            data: sample_tick(),
        };
        assert_eq!(tick.symbol(), &tw_symbol("2330"));
        assert_eq!(tick.timestamp(), 1_787_887_504_611);

        let kline = MarketMessage::Kline {
            symbol: tw_symbol("1437"),
            data: sample_kline(),
        };
        assert_eq!(kline.symbol(), &tw_symbol("1437"));
        assert_eq!(kline.timestamp(), 1_787_500_800_000);

        let snapshot = MarketMessage::QuoteSnapshot {
            symbol: a_symbol(),
            data: sample_snapshot(),
        };
        assert_eq!(snapshot.symbol(), &a_symbol());
        assert_eq!(snapshot.timestamp(), 1_787_904_896_000);
    }

    #[test]
    fn old_consumer_known_variants_still_work() {
        // 模拟“老消费者”：只处理已知四种变体，未来新增变体由 `_` 兜底。
        // 未知 type 标签由调用方先反序列化失败拦住，不会进入本函数。
        // 当前 enum 还没有第五个变体，`_` 暂时不可达；允许该 lint 以表达“新增变体不破坏旧 match”。
        #[allow(unreachable_patterns)]
        fn consume(msg: &MarketMessage) -> i64 {
            match msg {
                MarketMessage::Quote { data, .. } => data.ts,
                MarketMessage::Tick { data, .. } => data.ts,
                MarketMessage::Kline { data, .. } => data.ts,
                MarketMessage::QuoteSnapshot { data, .. } => data.ts,
                _ => 0,
            }
        }

        let msgs = [
            MarketMessage::Quote {
                symbol: a_symbol(),
                data: sample_quote(),
            },
            MarketMessage::Tick {
                symbol: tw_symbol("2330"),
                data: sample_tick(),
            },
            MarketMessage::Kline {
                symbol: tw_symbol("1437"),
                data: sample_kline(),
            },
            MarketMessage::QuoteSnapshot {
                symbol: a_symbol(),
                data: sample_snapshot(),
            },
        ];

        for msg in &msgs {
            assert_eq!(consume(msg), msg.timestamp());
        }
    }
}
