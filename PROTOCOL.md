# scrcpy-rs Protocol Notes

This document describes the scrcpy protocol behavior that this crate implements.
It is not a full upstream scrcpy specification — only the wire-format details and
design choices visible in this repository are covered here.

Protocol facts below are aligned with:

- `src/protocol/control.rs`
- `src/protocol/video.rs`
- `src/protocol/audio.rs`
- scrcpy server version `3.3.3`

## Connection model

The scrcpy protocol uses a three-socket model:

1. video stream
2. audio stream (optional)
3. control stream

The consumer must accept them in exactly that order.

### Handshake sequence

1. Resolve or push `scrcpy-server-v3.3.3`.
2. Reserve a local TCP port from `DEFAULT_PORT_RANGE`.
3. Install an ADB reverse tunnel.
4. Start the Android server process.
5. Accept the video socket.
6. Accept the audio socket if audio is enabled.
7. Accept the control socket.
8. If requested, read device metadata from the video socket.
9. If requested, read video codec metadata from the video socket.
10. If requested and audio is enabled, read audio codec metadata from the audio socket.

### Audio gating

The Android API level is checked before enabling audio. Devices below Android 11 / API 30
are downgraded to video + control only; the consumer is responsible for handling this
downgrade and surfacing the reason to the user.

## Device metadata

When `send_device_meta` is enabled, scrcpy sends a fixed 64-byte device name field.

- encoding: UTF-8
- size: 64 bytes
- padding: trailing `\0`
- sender: the first available media socket (video socket in the normal three-socket flow)

## Video codec metadata

When `send_codec_meta` is enabled, 12 bytes are read before normal video packets:

```text
0..4   codec_id   u32 big-endian
4..8   width      u32 big-endian
8..12  height     u32 big-endian
```

Common upstream codec ids:

- `h264 = 0x68323634`
- `h265 = 0x68323635`
- `av1  = 0x00617631`

The consumer reads this metadata after the handshake and should store the resolution
and codec id for use when decoding subsequent packets.

## Video packet format

`src/protocol/video.rs` parses the current wire format.

Each packet is:

```text
0..8   pts_and_flags  u64 big-endian
8..12  packet_size    u32 big-endian
12..   payload
```

### Flags layout

- bit 63: config packet
- bit 62: key frame
- low 62 bits: PTS in microseconds

This matches the packet parser constants:

- `PACKET_FLAG_CONFIG = 1 << 63`
- `PACKET_FLAG_KEY_FRAME = 1 << 62`

Packets larger than `MAX_PACKET_SIZE` (10 MiB) are rejected to guard against
maliciously large `packet_size` values.

## Audio codec metadata

When audio codec metadata is requested, the upstream format is 4 bytes:

```text
0..4  codec_id  u32 big-endian
```

Common upstream values:

- `opus = 0x6f707573`
- `aac  = 0x00616163`
- `flac = 0x666c6163`
- `raw  = 0x00726177`

Two special values are treated as explicit device-side failures:

- `0`: audio stream disabled by device
- `1`: audio configuration error on the device side

The codec id received during the handshake must be stamped onto every `AudioPacket`
by the consumer after calling `AudioPacket::from_bytes`.

## Audio packet format

`src/protocol/audio.rs` parses a 12-byte packet header followed by payload:

```text
0..8   pts_and_flags  u64 big-endian
8..12  packet_size    u32 big-endian
12..   payload
```

- `pts` is derived from the low 63 bits of `pts_and_flags`
- the high bit is exposed as a generic flag value
- `codec_id` is populated from the codec metadata received during the handshake

## Frame metadata mode

When scrcpy frame metadata is enabled, media packets use a 12-byte header where:

- 8 bytes are `pts_and_flags`
- 4 bytes are `packet_size`

For video, the known config/keyframe bits are fully modeled. For audio, the
current implementation keeps the highest-bit flag only.

## Control message framing

Control messages use the standard scrcpy layout:

```text
[type:1][payload...]
```

All integer payloads are serialized big-endian.

### Message type enum

`src/protocol/control.rs` keeps the upstream type numbers 0 through 17:

| Type | Name |
| --- | --- |
| 0 | InjectKeycode |
| 1 | InjectText |
| 2 | InjectTouchEvent |
| 3 | InjectScrollEvent |
| 4 | BackOrScreenOn |
| 5 | ExpandNotificationPanel |
| 6 | ExpandSettingsPanel |
| 7 | CollapsePanels |
| 8 | GetClipboard |
| 9 | SetClipboard |
| 10 | SetDisplayPower |
| 11 | RotateDevice |
| 12 | UhidCreate |
| 13 | UhidInput |
| 14 | UhidDestroy |
| 15 | OpenHardKeyboardSettings |
| 16 | StartApp |
| 17 | ResetVideo |

### Messages currently serialized

The `ControlMessage` implementation actively serializes these commands:

- `InjectKeycode`
- `InjectText`
- `InjectTouchEvent`
- `InjectScrollEvent`
- `BackOrScreenOn`
- `ExpandNotificationPanel`
- `ExpandSettingsPanel`
- `CollapsePanels`
- `SetDisplayPower`
- `RotateDevice`

The enum keeps the other upstream type numbers for protocol alignment; they are
not yet serialized.

### Control-message details

#### InjectText

- UTF-8 only
- text is truncated to at most 300 bytes at a valid UTF-8 char boundary

#### InjectTouchEvent

- serialized size: 32 bytes
- pressure is clamped to `0.0..=1.0` and converted to `u16`
- special pointer ids defined in code:
  - `POINTER_ID_MOUSE = u64::MAX`
  - `POINTER_ID_GENERIC_FINGER = u64::MAX - 1`
  - `POINTER_ID_VIRTUAL_FINGER = u64::MAX - 2`

Position serialization is 12 bytes total:

```text
x            u32
y            u32
screen_width u16
screen_height u16
```

#### InjectScrollEvent

- scroll values are first divided by 16
- normalized to `-1.0..=1.0`
- then converted to signed 16-bit fixed-point values

## What is not yet implemented

This crate does not mirror every upstream scrcpy feature.

Examples of upstream capabilities not yet implemented:

- clipboard sync messages
- UHID device lifecycle messages
- start-app control message serialization
- reset-video control message serialization
- a general device-message (server → client) parser module
- full upstream audio codec special-case parsing for Opus / FLAC config packets

## Contributor pointers

- Input / gesture behavior → `src/protocol/control.rs`
- Media packet stutter or malformed data → `src/protocol/video.rs` / `src/protocol/audio.rs`
- Extending protocol coverage → add the missing message model, then update this document
