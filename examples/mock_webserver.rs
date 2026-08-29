//! mock webserver：演示 webserver 侧收帧、分拣、fan-out。
//!
//! 运行：
//! ```bash
//! # 方式一：直接跑，使用内置样本
//! cargo run --example mock_webserver
//!
//! # 方式二：从 mock_spider 管道读帧流
//! cargo run --example mock_spider | cargo run --example mock_webserver
//! ```
//!
//! 每行 JSON 先经 `is_control_frame` 分拣；控制帧直接打印，行情帧反序列化为
//! `MarketMessage` 后 fan-out。订阅者 A 用 `&dyn Message` 统一消费，
//! 订阅者 B 用 `&MarketMessage` 消费，两种方式同时保留作对照。

use std::io::{IsTerminal, Read};

use market_protocol::frame::is_control_frame;
use market_protocol::message::MarketMessage;
use market_protocol::types::{Kline, Level, Period, Quote, QuoteSnapshot, Tick};
use market_protocol::{Market, Message, Symbol};

fn main() {
    let frames = read_frames();
    let mut subscriber_a = Subscriber::new("A(dyn Message)");
    let mut subscriber_b = Subscriber::new("B(MarketMessage)");

    for frame in frames {
        if let Some(kind) = is_control_frame(&frame) {
            println!("[control] kind={kind} frame={frame}");
            continue;
        }

        let msg: MarketMessage = match serde_json::from_str(&frame) {
            Ok(msg) => msg,
            Err(err) => {
                println!("[drop] cannot parse as MarketMessage: {err}");
                continue;
            }
        };

        // ① fan-out 用 &dyn Message 统一消费。
        fanout_dyn(&msg, &mut |m: &dyn Message| {
            subscriber_a.on_message(m);
        });

        // ② fan-out 用 &MarketMessage（enum 自带方法）消费。
        fanout_enum(&msg, &mut |m: &MarketMessage| {
            subscriber_b.on_message(m);
        });
    }
}

fn read_frames() -> Vec<String> {
    // 管道输入时逐行读；空输入或终端运行则用内置样本，方便看到输出。
    let mut input = String::new();
    if !std::io::stdin().is_terminal() {
        std::io::stdin()
            .read_to_string(&mut input)
            .expect("read stdin");
    }
    let frames: Vec<String> = input.lines().map(str::to_owned).collect();
    if frames.is_empty() {
        default_frames()
    } else {
        frames
    }
}

fn fanout_dyn(msg: &dyn Message, subscriber: &mut dyn FnMut(&dyn Message)) {
    subscriber(msg);
}

fn fanout_enum(msg: &MarketMessage, subscriber: &mut dyn FnMut(&MarketMessage)) {
    subscriber(msg);
}

struct Subscriber {
    name: &'static str,
}

impl Subscriber {
    fn new(name: &'static str) -> Self {
        Self { name }
    }

    fn on_message(&mut self, msg: &dyn Message) {
        println!(
            "[{}] kind={} symbol={:?} ts={}",
            self.name,
            msg.kind(),
            msg.symbol(),
            msg.timestamp()
        );
    }
}

fn default_frames() -> Vec<String> {
    let a = Symbol::new(Market::A, "560010");
    let tw_2330 = Symbol::new(Market::Tw, "2330");
    let tw_1437 = Symbol::new(Market::Tw, "1437");

    let messages = vec![
        MarketMessage::Quote {
            symbol: a.clone(),
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
    ];

    messages
        .into_iter()
        .map(|m| serde_json::to_string(&m).expect("serialize MarketMessage"))
        .collect()
}
