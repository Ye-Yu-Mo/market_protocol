from pathlib import Path

from google.protobuf.message import DecodeError

from market_protocol.v1 import market_protocol_pb2 as pb


def market_frame() -> pb.TransportFrame:
    quote = pb.Quote(
        ts=1_787_904_896_000,
        price=3.208,
        open=3.211,
        high=3.245,
        low=3.204,
        pre_close=3.220,
        volume=132_886_000.0,
        amount=428_272_326.0,
        in_vol=69_090_700.0,
        out_vol=63_795_300.0,
    )
    envelope = pb.MarketEnvelope(
        symbol=pb.Symbol(market=pb.MARKET_A, code="560010"),
        quote=quote,
    )
    return pb.TransportFrame(market=envelope)


def test_market_round_trip_and_oneof_selection() -> None:
    encoded = market_frame().SerializeToString()

    decoded = pb.TransportFrame()
    decoded.ParseFromString(encoded)

    assert decoded.WhichOneof("payload") == "market"
    assert decoded.market.WhichOneof("payload") == "quote"
    assert decoded.market.symbol.code == "560010"
    assert decoded.market.symbol.market == pb.MARKET_A
    assert decoded.market.quote.price == 3.208
    assert decoded.market.quote.HasField("price")


def test_optional_presence_distinguishes_zero_from_missing() -> None:
    missing = pb.Quote()
    assert not missing.HasField("price")

    explicit_zero = pb.Quote(price=0.0)
    assert explicit_zero.HasField("price")
    assert explicit_zero.price == 0.0


def test_control_frames_are_binary_messages() -> None:
    heartbeat = pb.TransportFrame(heartbeat=pb.Heartbeat())
    assert heartbeat.WhichOneof("payload") == "heartbeat"

    resume = pb.TransportFrame(resume=pb.Resume(since_ts=123, since_serial=456))
    assert resume.WhichOneof("payload") == "resume"
    assert resume.resume.HasField("since_ts")
    assert resume.resume.HasField("since_serial")


def test_malformed_binary_is_rejected() -> None:
    decoded = pb.TransportFrame()
    try:
        decoded.ParseFromString(b"\x80")
    except DecodeError:
        pass
    else:
        raise AssertionError("malformed protobuf should raise DecodeError")


def test_history_request_response_and_pagination() -> None:
    request = pb.HistoryRequest(
        request_id="req-1",
        symbols=[pb.Symbol(market=pb.MARKET_A, code="000001")],
        period=pb.PERIOD_D1,
        start_date="2026-01-01",
        end_date="2026-01-31",
        adjustment=pb.ADJUSTMENT_RAW,
        page_size=500,
    )
    request_frame = pb.TransportFrame(history_request=request)
    decoded_request = pb.TransportFrame.FromString(request_frame.SerializeToString())
    assert decoded_request.WhichOneof("payload") == "history_request"
    assert decoded_request.history_request.symbols[0].code == "000001"
    assert decoded_request.history_request.adjustment == pb.ADJUSTMENT_RAW

    response = pb.HistoryResponse(
        chunk=pb.HistoryChunk(
            request_id="req-1",
            records=[
                pb.HistoryRecord(
                    symbol=pb.Symbol(market=pb.MARKET_A, code="000001"),
                    trade_date="2026-01-02",
                    kline=pb.Kline(
                        ts=1_767_283_200_000,
                        period=pb.PERIOD_D1,
                        high=10.5,
                        low=9.5,
                        close=10.0,
                        volume=100.0,
                    ),
                )
            ],
            next_page_token="page-2",
            end=False,
            snapshot_id="snapshot-1",
            adjustment=pb.ADJUSTMENT_RAW,
        )
    )
    response_frame = pb.TransportFrame(history_response=response)
    decoded_response = pb.TransportFrame.FromString(response_frame.SerializeToString())
    assert decoded_response.WhichOneof("payload") == "history_response"
    assert decoded_response.history_response.WhichOneof("payload") == "chunk"
    chunk = decoded_response.history_response.chunk
    assert chunk.records[0].trade_date == "2026-01-02"
    assert chunk.records[0].kline.HasField("open") is False
    assert chunk.next_page_token == "page-2"
    assert chunk.HasField("end")
    assert chunk.end is False


def test_history_error_is_a_response_variant() -> None:
    response = pb.HistoryResponse(
        error=pb.HistoryError(
            request_id="req-2",
            code="invalid_range",
            message="end date precedes start date",
            retryable=False,
        )
    )
    frame = pb.TransportFrame(history_response=response)
    decoded = pb.TransportFrame.FromString(frame.SerializeToString())
    assert decoded.history_response.WhichOneof("payload") == "error"
    assert decoded.history_response.error.HasField("retryable")
    assert decoded.history_response.error.retryable is False
