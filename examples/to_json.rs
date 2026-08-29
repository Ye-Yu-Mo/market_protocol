//! 演示：把真实样本 `MarketMessage` 序列化为 envelope JSON。
//!
//! 运行：`cargo run --example to_json`

use market_protocol::message::MarketMessage;
use market_protocol::types::Quote;
use market_protocol::{Market, Symbol};

fn main() {
    // A 股 560010 腾讯实测报价（envelope 顶层无 ts，ts 在 data 内）。
    let msg = MarketMessage::Quote {
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
            in_vol: Some(69_090_700.0),
            out_vol: Some(63_795_300.0),
        },
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&msg).expect("serialize MarketMessage")
    );
}
