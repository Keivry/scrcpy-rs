// SPDX-License-Identifier: MIT OR Apache-2.0

use {
    crate::{MAX_PACKET_SIZE, Result, ScrcpyError},
    byteorder::{BigEndian, ReadBytesExt},
    std::io::Read,
};

pub const PACKET_FLAG_CONFIG: u64 = 1 << 63;
pub const PACKET_FLAG_KEY_FRAME: u64 = 1 << 62;

/// A decoded video packet received from the scrcpy server.
///
/// Config packets (`is_config = true`) carry codec-specific parameter data
/// (e.g. H.264 SPS/PPS) and have no display timestamp.  Key-frame packets
/// (`is_keyframe = true`) can be decoded without prior frames.
#[derive(Debug, Clone)]
pub struct VideoPacket {
    /// Presentation timestamp in microseconds.  Meaningless for config packets.
    pub pts_us: u64,
    /// `true` if this packet carries codec configuration data (SPS/PPS for H.264).
    pub is_config: bool,
    /// `true` if this packet is an independently decodable key frame.
    pub is_keyframe: bool,
    /// Raw compressed video payload.
    pub data: Vec<u8>,
}

impl VideoPacket {
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let pts_and_flags = reader.read_u64::<BigEndian>()?;
        let packet_size = reader.read_u32::<BigEndian>()? as usize;

        if packet_size > MAX_PACKET_SIZE {
            return Err(ScrcpyError::ProtocolError(format!(
                "Video packet size {} exceeds maximum allowed {} (10MB)",
                packet_size, MAX_PACKET_SIZE
            )));
        }

        let mut data = vec![0u8; packet_size];
        reader.read_exact(&mut data)?;

        let is_config = (pts_and_flags & PACKET_FLAG_CONFIG) != 0;
        let is_keyframe = (pts_and_flags & PACKET_FLAG_KEY_FRAME) != 0;
        let pts_mask = !(PACKET_FLAG_CONFIG | PACKET_FLAG_KEY_FRAME);

        Ok(Self {
            pts_us: pts_and_flags & pts_mask,
            is_config,
            is_keyframe,
            data,
        })
    }

    pub fn is_empty(&self) -> bool { self.data.is_empty() }

    pub fn total_size(&self) -> usize { 12 + self.data.len() }
}
