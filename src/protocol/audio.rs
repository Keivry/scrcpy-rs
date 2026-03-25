// SPDX-License-Identifier: MIT OR Apache-2.0

use {
    crate::{MAX_PACKET_SIZE, Result, ScrcpyError},
    byteorder::{BigEndian, ReadBytesExt},
    std::io::Cursor,
};

/// A decoded audio packet received from the scrcpy server.
///
/// `codec_id` is populated by the connection layer after reading the codec
/// metadata packet that precedes the audio stream.
#[derive(Debug, Clone)]
pub struct AudioPacket {
    /// Presentation timestamp extracted from the high 63 bits of the wire header.
    pub pts: i64,
    /// Codec identifier for this packet.
    ///
    /// Always `0` when constructed via [`AudioPacket::from_bytes`] because the
    /// scrcpy protocol sends codec metadata in a separate preceding packet.
    /// The connection layer is responsible for reading that metadata packet and
    /// filling in this field afterwards.
    pub codec_id: u32,
    /// Packet flags from the wire header (currently only the MSB is used).
    pub flags: u64,
    /// Raw compressed audio payload.
    pub payload: Vec<u8>,
}

impl AudioPacket {
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 12 {
            return Err(ScrcpyError::ProtocolError(format!(
                "Audio packet too short: expected at least 12 bytes, got {}",
                data.len()
            )));
        }

        let mut cursor = Cursor::new(data);
        let pts_and_flags = cursor.read_u64::<BigEndian>().map_err(|e| {
            ScrcpyError::ProtocolError(format!("Failed to read pts_and_flags: {e}"))
        })?;
        let packet_size = cursor
            .read_u32::<BigEndian>()
            .map_err(|e| ScrcpyError::ProtocolError(format!("Failed to read packet_size: {e}")))?;

        if packet_size as usize > MAX_PACKET_SIZE {
            return Err(ScrcpyError::ProtocolError(format!(
                "Audio packet size {} exceeds maximum allowed {} (10MB)",
                packet_size, MAX_PACKET_SIZE
            )));
        }

        let pts = (pts_and_flags & 0x7FFF_FFFF_FFFF_FFFF) as i64;
        let flags = pts_and_flags >> 63;
        let payload_start = 12;
        let expected_total = payload_start + packet_size as usize;

        if data.len() != expected_total {
            return Err(ScrcpyError::ProtocolError(format!(
                "Audio packet size mismatch: expected {}, got {}",
                expected_total,
                data.len()
            )));
        }

        Ok(Self {
            pts,
            codec_id: 0,
            flags,
            payload: data[payload_start..].to_vec(),
        })
    }

    pub fn size(&self) -> usize { 12 + self.payload.len() }
}
