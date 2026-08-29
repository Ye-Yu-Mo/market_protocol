//! 统一行情消息的元数据访问 trait。
//!
//! # M3 裁决：保留
//!
//! `MarketMessage` enum 自带 `symbol()`/`timestamp()`，但 webserver 的 fan-out
//! 订阅者集合是异构处理器：不同订阅者可以只依赖 `&dyn Message` 的统一视图，
//! 而不必在每处都 match 具体 enum 变体。`&dyn Message` 为“一个处理入口接任意
//! 消息”提供了编译期接口，后续若有新的消息实现类型（例如直接来自 spider 的
//! 原始包装类型）也可以接入同一 fan-out，不需要改 webserver 的分发代码。
//!
//! 因此本模块保留，并在 `tests/pipeline.rs` 中同时保留 `&dyn Message` 与
//! `&MarketMessage` 两种 fan-out 写法作对照。

use crate::Symbol;
use crate::message::MarketMessage;

/// 统一行情消息的元数据访问。
///
/// 让消费方以 `&dyn Message` 处理任意消息，而无需逐个 match 具体类型。
pub trait Message {
    /// 消息类型标签（与 envelope `type` 一致）：`"quote"`/`"tick"`/`"kline"`/`"quote_snapshot"`。
    fn kind(&self) -> &'static str;
    /// 消息所属标的。
    fn symbol(&self) -> &Symbol;
    /// 消息数据内时间戳（epoch 毫秒）。
    fn timestamp(&self) -> i64;
}

impl Message for MarketMessage {
    fn kind(&self) -> &'static str {
        match self {
            Self::Quote { .. } => "quote",
            Self::Tick { .. } => "tick",
            Self::Kline { .. } => "kline",
            Self::QuoteSnapshot { .. } => "quote_snapshot",
        }
    }

    fn symbol(&self) -> &Symbol {
        self.symbol()
    }

    fn timestamp(&self) -> i64 {
        self.timestamp()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Market;
    use crate::message::MarketMessage;
    use crate::types::{Kline, Level, Period, Quote, QuoteSnapshot, Tick};

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

    fn messages() -> Vec<MarketMessage> {
        vec![
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
        ]
    }

    #[test]
    fn kind_returns_envelope_type_label_for_all_variants() {
        let expected = ["quote", "tick", "kline", "quote_snapshot"];
        for (msg, expected_kind) in messages().iter().zip(expected) {
            assert_eq!(msg.kind(), expected_kind);
        }
    }

    #[test]
    fn symbol_and_timestamp_match_enum_accessors() {
        for msg in messages() {
            let dyn_msg: &dyn Message = &msg;
            assert_eq!(dyn_msg.symbol(), msg.symbol());
            assert_eq!(dyn_msg.timestamp(), msg.timestamp());
        }
    }

    #[test]
    fn fanout_can_consume_via_dyn_message() {
        let mut kinds = Vec::new();
        let mut fanout = |msg: &dyn Message| kinds.push(msg.kind());
        for msg in &messages() {
            fanout(msg);
        }
        assert_eq!(kinds, vec!["quote", "tick", "kline", "quote_snapshot"]);
    }
}
