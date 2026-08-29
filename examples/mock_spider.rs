//! mock spider：演示 spider 侧用协议产出 WS 帧流。
//!
//! 运行：`cargo run --example mock_spider`
//!
//! 输出为 JSON 行（每行一个 WS text frame），可直接管道给
//! `cargo run --example mock_webserver`：
//!
//! ```bash
//! cargo run --example mock_spider | cargo run --example mock_webserver
//! ```

use market_protocol::message::MarketMessage;
use market_protocol::types::{Kline, Level, Period, Quote, QuoteSnapshot, Tick};
use market_protocol::{Market, Symbol};

fn main() {
    let messages = sample_messages();
    for msg in messages {
        let frame = serde_json::to_string(&msg).expect("serialize MarketMessage");
        println!("{frame}");
    }
}

/// 真实样本消息流：A 股 560010 Quote、台股 2330 Tick（同 ts 连续多笔 serial 递增）、
/// 台股 1437 Kline、A 股 560010 QuoteSnapshot。
fn sample_messages() -> Vec<MarketMessage> {
    let a = Symbol::new(Market::A, "560010");
    let tw_2330 = Symbol::new(Market::Tw, "2330");
    let tw_1437 = Symbol::new(Market::Tw, "1437");

    vec![
        MarketMessage::Quote {
            symbol: a.clone(),
            data: Quote {
                ts: 1_787_904_896_000,
                price: 3.208,
                open: 3.211,
                high: 3.245,
                low: 3.204,
                pre_close: 3.220,
                // 源如果给“手”，spider 必须 ×100 换算成“股”再进协议。
                volume: 132_886_000.0,
                amount: 428_272_326.0,
                in_vol: Some(69_090_700.0),
                out_vol: Some(63_795_300.0),
            },
        },
        MarketMessage::Tick {
            symbol: tw_2330.clone(),
            data: Tick {
                ts: 1_787_887_504_611,
                price: 1420.0,
                volume: 2_000.0,
                buy_price: Some(1420.0),
                sell_price: Some(1425.0),
                inout_flag: Some(1),
                serial: 41,
            },
        },
        MarketMessage::Tick {
            symbol: tw_2330.clone(),
            data: Tick {
                ts: 1_787_887_504_611,
                price: 1420.0,
                volume: 3_000.0,
                buy_price: Some(1420.0),
                sell_price: Some(1425.0),
                inout_flag: Some(1),
                serial: 42,
            },
        },
        MarketMessage::Tick {
            symbol: tw_2330,
            data: Tick {
                ts: 1_787_887_504_611,
                price: 1421.0,
                volume: 1_000.0,
                buy_price: Some(1421.0),
                sell_price: Some(1425.0),
                inout_flag: Some(1),
                serial: 43,
            },
        },
        MarketMessage::Kline {
            symbol: tw_1437,
            data: Kline {
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
            },
        },
        MarketMessage::QuoteSnapshot {
            symbol: a,
            data: QuoteSnapshot {
                ts: 1_787_904_896_000,
                bids: vec![
                    Level {
                        price: 3.207,
                        volume: 10_000.0,
                    },
                    Level {
                        price: 3.206,
                        volume: 20_000.0,
                    },
                ],
                asks: vec![
                    Level {
                        price: 3.208,
                        volume: 5_000.0,
                    },
                    Level {
                        price: 3.209,
                        volume: 8_000.0,
                    },
                ],
            },
        },
    ]
}
