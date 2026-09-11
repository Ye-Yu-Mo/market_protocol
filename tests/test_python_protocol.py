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


def test_rust_golden_fixture_has_the_same_semantics() -> None:
    fixture = (
        Path(__file__).parent / "fixtures" / "quote_transport.hex"
    ).read_text().strip()
    expected = bytes.fromhex(fixture)
    assert market_frame().SerializeToString() == expected
    decoded = pb.TransportFrame.FromString(expected)

    assert decoded.WhichOneof("payload") == "market"
    assert decoded.market.WhichOneof("payload") == "quote"
    assert decoded.market.symbol.code == "560010"
    assert decoded.market.quote.ts == 1_787_904_896_000
    assert decoded.market.quote.HasField("in_vol")
    assert decoded.market.quote.in_vol == 69_090_700.0
