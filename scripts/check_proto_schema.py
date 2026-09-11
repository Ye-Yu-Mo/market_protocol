#!/usr/bin/env python3
"""Check the frozen market_protocol.v1 protobuf contract."""

from __future__ import annotations

from pathlib import Path
import re
import sys

PROTO = Path(__file__).parents[1] / "proto/market_protocol/v1/market_protocol.proto"

EXPECTED_ENUMS = {
    "Market": {
        "MARKET_UNSPECIFIED": 0,
        "MARKET_A": 1,
        "MARKET_TW": 2,
        "MARKET_CRYPTO": 3,
    },
    "Period": {
        "PERIOD_UNSPECIFIED": 0,
        "PERIOD_M1": 1,
        "PERIOD_M5": 2,
        "PERIOD_M15": 3,
        "PERIOD_M30": 4,
        "PERIOD_M60": 5,
        "PERIOD_D1": 6,
        "PERIOD_W1": 7,
        "PERIOD_MONTH_1": 8,
    },
    "Adjustment": {
        "ADJUSTMENT_UNSPECIFIED": 0,
        "ADJUSTMENT_RAW": 1,
        "ADJUSTMENT_QFQ": 2,
        "ADJUSTMENT_HFQ": 3,
    },
}

EXPECTED_FIELDS = {
    "Symbol": {"market": 1, "code": 2},
    "Quote": {
        "ts": 1,
        "price": 2,
        "open": 3,
        "high": 4,
        "low": 5,
        "pre_close": 6,
        "volume": 7,
        "amount": 8,
        "in_vol": 9,
        "out_vol": 10,
    },
    "Tick": {
        "ts": 1,
        "price": 2,
        "volume": 3,
        "buy_price": 4,
        "sell_price": 5,
        "inout_flag": 6,
        "serial": 7,
    },
    "Kline": {
        "ts": 1,
        "period": 2,
        "open": 3,
        "high": 4,
        "low": 5,
        "close": 6,
        "volume": 7,
        "amount": 8,
        "turnover": 9,
        "pre_close": 10,
    },
    "Level": {"price": 1, "volume": 2},
    "QuoteSnapshot": {"ts": 1, "bids": 2, "asks": 3},
    "MarketEnvelope": {"symbol": 1, "quote": 2, "tick": 3, "kline": 4, "quote_snapshot": 5},
    "Heartbeat": {},
    "Resume": {"since_ts": 1, "since_serial": 2},
    "HistoryRequest": {
        "request_id": 1,
        "symbols": 2,
        "period": 3,
        "start_date": 4,
        "end_date": 5,
        "adjustment": 6,
        "page_size": 7,
        "page_token": 8,
    },
    "HistoryRecord": {"symbol": 1, "trade_date": 2, "kline": 3},
    "HistoryChunk": {
        "request_id": 1,
        "records": 2,
        "next_page_token": 3,
        "end": 4,
        "snapshot_id": 5,
        "adjustment": 6,
    },
    "HistoryError": {"request_id": 1, "code": 2, "message": 3, "retryable": 4},
    "HistoryResponse": {"chunk": 1, "error": 2},
    "TransportFrame": {
        "market": 1,
        "heartbeat": 2,
        "resume": 3,
        "history_request": 4,
        "history_response": 5,
    },
}

EXPECTED_OPTIONAL = {
    "Symbol": {"market", "code"},
    "Quote": {
        "ts", "price", "open", "high", "low", "pre_close", "volume", "amount", "in_vol", "out_vol"
    },
    "Tick": {"ts", "price", "volume", "buy_price", "sell_price", "inout_flag", "serial"},
    "Kline": {
        "ts", "period", "open", "high", "low", "close", "volume", "amount", "turnover", "pre_close"
    },
    "Level": {"price", "volume"},
    "QuoteSnapshot": {"ts"},
    "MarketEnvelope": {"symbol"},
    "Resume": {"since_ts", "since_serial"},
    "HistoryRequest": {
        "request_id", "period", "start_date", "end_date", "adjustment", "page_size", "page_token"
    },
    "HistoryRecord": {"symbol", "trade_date", "kline"},
    "HistoryChunk": {"request_id", "next_page_token", "end", "snapshot_id", "adjustment"},
    "HistoryError": {"request_id", "code", "message", "retryable"},
}


def strip_comments(source: str) -> str:
    return re.sub(r"//[^\n]*", "", source)


def blocks(source: str, keyword: str) -> dict[str, str]:
    result = {}
    pattern = re.compile(rf"\b{keyword}\s+(\w+)\s*\{{")
    for match in pattern.finditer(source):
        depth = 1
        index = match.end()
        while depth and index < len(source):
            if source[index] == "{":
                depth += 1
            elif source[index] == "}":
                depth -= 1
            index += 1
        result[match.group(1)] = source[match.end() : index - 1]
    return result


def fields(block: str) -> dict[str, int]:
    block = strip_comments(block)
    return {
        name: int(number)
        for name, number in re.findall(
            r"\b(?:optional\s+|repeated\s+)?[.\w]+\s+(\w+)\s*=\s*(\d+)\s*;",
            block,
        )
    }


def optional_fields(block: str) -> set[str]:
    block = strip_comments(block)
    return {
        name
        for name in re.findall(
            r"\boptional\s+[.\w]+\s+(\w+)\s*=\s*\d+\s*;", block
        )
    }


def enum_values(block: str) -> dict[str, int]:
    block = strip_comments(block)
    return {
        name: int(number)
        for name, number in re.findall(r"\b(\w+)\s*=\s*(-?\d+)\s*;", block)
    }


def oneof_fields(block: str) -> dict[str, int] | None:
    payload = re.search(r"\boneof\s+payload\s*\{(.*?)\}", block, re.S)
    return fields(payload.group(1)) if payload else None


def main() -> int:
    source = PROTO.read_text()
    errors = []

    if not re.search(r"\bsyntax\s*=\s*\"proto3\"\s*;", source):
        errors.append("syntax must be proto3")
    if not re.search(r"\bpackage\s+market_protocol\.v1\s*;", source):
        errors.append("package must be market_protocol.v1")

    actual_enums = blocks(source, "enum")
    if set(actual_enums) != set(EXPECTED_ENUMS):
        errors.append(f"enum declarations changed: expected {set(EXPECTED_ENUMS)}, got {set(actual_enums)}")
    for name, expected in EXPECTED_ENUMS.items():
        actual = enum_values(actual_enums.get(name, ""))
        if actual != expected:
            errors.append(f"enum {name} changed: expected {expected}, got {actual}")

    actual_messages = blocks(source, "message")
    if set(actual_messages) != set(EXPECTED_FIELDS):
        errors.append(
            f"message declarations changed: expected {set(EXPECTED_FIELDS)}, got {set(actual_messages)}"
        )
    for name, expected in EXPECTED_FIELDS.items():
        actual = fields(actual_messages.get(name, ""))
        if actual != expected:
            errors.append(f"message {name} fields changed: expected {expected}, got {actual}")
        expected_optional = EXPECTED_OPTIONAL.get(name, set())
        actual_optional = optional_fields(actual_messages.get(name, ""))
        if actual_optional != expected_optional:
            errors.append(
                f"message {name} optional fields changed: expected {expected_optional}, got {actual_optional}"
            )

    for name, expected in {
        "MarketEnvelope": {
            "quote": 2,
            "tick": 3,
            "kline": 4,
            "quote_snapshot": 5,
        },
        "HistoryResponse": {"chunk": 1, "error": 2},
        "TransportFrame": {
            "market": 1,
            "heartbeat": 2,
            "resume": 3,
            "history_request": 4,
            "history_response": 5,
        },
    }.items():
        actual = oneof_fields(actual_messages.get(name, ""))
        if actual != expected:
            errors.append(f"{name}.payload oneof changed: expected {expected}, got {actual}")

    if errors:
        for error in errors:
            print(f"schema contract error: {error}", file=sys.stderr)
        return 1

    print("protobuf schema contract: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
