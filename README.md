# scrcpy-rs

[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

Scrcpy wire-protocol types and control primitives for Rust.

This crate owns the **protocol layer** only: packet types, H.264 stream parsing, and the control-message sender.

## Add to your project

```toml
[dependencies]
scrcpy = { path = "3rd-party/scrcpy-rs" }
```

## Public API

### Packet types

| Type            | Description                                       |
| --------------- | ------------------------------------------------- |
| [`VideoPacket`] | One compressed video frame from the server stream |
| [`AudioPacket`] | One compressed audio frame from the server stream |

`VideoPacket::read_from(reader)` reads a packet from any `std::io::Read`
source. `AudioPacket::from_bytes(data)` parses a packet from a byte slice;
`codec_id` is always `0` on construction and must be filled in by the caller
after reading the preceding codec-metadata packet.

### H.264 helpers

`scrcpy::h264::extract_resolution_from_stream(data)` — scan a raw H.264
byte-stream for an SPS NAL unit and return `(width, height)`. Accounts for
`frame_cropping_flag` so the result matches the true display size (e.g. 1080,
not 1088).

### Control channel

[`ControlSender`] serialises and sends scrcpy control messages over a
`TcpStream`. All clones share the same underlying socket (guarded by a
`Mutex`).

```rust
use scrcpy::ControlSender;

let sender = ControlSender::new(tcp_stream, screen_width, screen_height);
sender.send_touch_down(x, y)?;
sender.send_touch_up(x, y)?;
sender.send_key_press(keycode, metastate)?;
sender.send_text("hello")?;
sender.send_screen_off()?;
sender.send_scroll(x, y, 0.0, -1.0)?;
```

### Error type

[`ScrcpyError`] / [`Result`] — crate-level error and result aliases.

### Constants

| Constant                   | Value                                 |
| -------------------------- | ------------------------------------- |
| `SCRCPY_SERVER_VERSION`    | `"3.3.3"`                             |
| `SCRCPY_SERVER_CLASS_NAME` | `"com.genymobile.scrcpy.Server"`      |
| `SCRCPY_SERVER_PATH`       | `"/data/local/tmp/scrcpy-server.jar"` |
| `MAX_PACKET_SIZE`          | 10 MB                                 |
| `GRACEFUL_WAIT_MS`         | 250 ms                                |

## Protocol notes

See [`PROTOCOL.md`](PROTOCOL.md) for a detailed description of the scrcpy
wire protocol (handshake, stream framing, control message encoding).

## Supported scrcpy server version

`3.3.3` — the matching server JAR must be pushed to the device before connecting.

## License

Licensed under either of

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.
