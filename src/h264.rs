// SPDX-License-Identifier: MIT OR Apache-2.0

const NAL_TYPE_SPS: u8 = 7;

/// Scan a raw H.264 byte-stream for an SPS NAL unit and return the display
/// resolution `(width, height)` encoded in it.
///
/// Returns `None` if no SPS NAL unit is found or if the SPS data cannot be
/// parsed.  The resolution accounts for `frame_cropping_flag` so the returned
/// dimensions match the true display size (e.g. 1080, not 1088).
pub fn extract_resolution_from_stream(data: &[u8]) -> Option<(u32, u32)> {
    let nals = find_nal_units(data);

    for nal in nals {
        if nal.is_empty() {
            continue;
        }

        if (nal[0] & 0x1F) == NAL_TYPE_SPS {
            return parse_sps_resolution(nal);
        }
    }

    None
}

fn find_nal_units(data: &[u8]) -> Vec<&[u8]> {
    let mut nals = Vec::new();
    let mut i = 0;

    while i < data.len() {
        if let Some(start) = find_start_code(data, i) {
            if let Some(end) = find_start_code(data, start + 3) {
                nals.push(&data[start..end]);
                i = end;
            } else {
                nals.push(&data[start..]);
                break;
            }
        } else {
            break;
        }
    }

    nals
}

fn find_start_code(data: &[u8], offset: usize) -> Option<usize> {
    for i in offset..data.len().saturating_sub(2) {
        if data[i] == 0 && data[i + 1] == 0 {
            if data[i + 2] == 1 {
                return Some(i + 3);
            }
            if i + 3 < data.len() && data[i + 2] == 0 && data[i + 3] == 1 {
                return Some(i + 4);
            }
        }
    }

    None
}

/// Remove H.264 emulation-prevention bytes from a NALU payload, producing the
/// Raw Byte Sequence Payload (RBSP).
///
/// The H.264 spec inserts `0x03` after every `0x00 0x00` pair in the RBSP to
/// prevent accidental start-code emulation.  Before bit-level parsing the SPS
/// payload must be converted back to RBSP by removing these `0x03` bytes.
fn remove_emulation_prevention_bytes(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(data.len());
    let mut i = 0;
    while i < data.len() {
        // 0x00 0x00 0x03 → 0x00 0x00 (drop the 0x03)
        if i + 2 < data.len() && data[i] == 0x00 && data[i + 1] == 0x00 && data[i + 2] == 0x03 {
            result.push(data[i]);
            result.push(data[i + 1]);
            i += 3;
        } else {
            result.push(data[i]);
            i += 1;
        }
    }
    result
}

fn parse_sps_resolution(sps: &[u8]) -> Option<(u32, u32)> {
    if sps.is_empty() {
        return None;
    }

    // Strip emulation-prevention bytes before bit-level parsing (H.264 §7.4.1).
    // The first byte is the NAL header and is not part of the RBSP payload.
    let rbsp = remove_emulation_prevention_bytes(&sps[1..]);
    let mut reader = BitReader::new(&rbsp);
    reader.skip(8)?;
    reader.skip(8)?;
    reader.skip(8)?;
    reader.read_ue()?;

    let profile_idc = sps[1];
    if [100, 110, 122, 244, 44, 83, 86, 118, 128].contains(&profile_idc) {
        let chroma_format_idc = reader.read_ue()?;

        if chroma_format_idc == 3 {
            reader.skip(1)?;
        }

        reader.read_ue()?;
        reader.read_ue()?;
        reader.skip(1)?;

        let seq_scaling_matrix_present_flag = reader.read_bit()?;
        if seq_scaling_matrix_present_flag == 1 {
            let count = if chroma_format_idc != 3 { 8 } else { 12 };
            for i in 0..count {
                let seq_scaling_list_present_flag = reader.read_bit()?;
                if seq_scaling_list_present_flag == 1 {
                    let size = if i < 6 { 16 } else { 64 };
                    let mut last_scale = 8;
                    let mut next_scale = 8;
                    for _ in 0..size {
                        if next_scale != 0 {
                            let delta_scale = reader.read_se()?;
                            next_scale = (last_scale + delta_scale + 256) % 256;
                        }
                        last_scale = if next_scale == 0 {
                            last_scale
                        } else {
                            next_scale
                        };
                    }
                }
            }
        }
    }

    reader.read_ue()?;
    let pic_order_cnt_type = reader.read_ue()?;
    if pic_order_cnt_type == 0 {
        reader.read_ue()?;
    } else if pic_order_cnt_type == 1 {
        reader.skip(1)?;
        reader.read_se()?;
        reader.read_se()?;
        let num_ref_frames_in_pic_order_cnt_cycle = reader.read_ue()?;
        for _ in 0..num_ref_frames_in_pic_order_cnt_cycle {
            reader.read_se()?;
        }
    }

    reader.read_ue()?;
    reader.skip(1)?;

    let pic_width_in_mbs_minus1 = reader.read_ue()?;
    let pic_height_in_map_units_minus1 = reader.read_ue()?;
    let frame_mbs_only_flag = reader.read_bit()?;

    if frame_mbs_only_flag == 0 {
        reader.skip(1)?;
    }

    let mut width = (pic_width_in_mbs_minus1 + 1) * 16;
    let mut height = (pic_height_in_map_units_minus1 + 1) * 16 * (2 - frame_mbs_only_flag as u32);

    // H.264 §7.4.2.1.1 — frame_cropping_flag trims macrobloc-aligned dimensions
    // to the true display size.  For 4:2:0 content (standard on Android), each
    // crop unit is 2 luma samples.  Without this, a 1080p stream reports 1088
    // because 1080 is not a multiple of 16.
    let frame_cropping_flag = reader.read_bit()?;
    if frame_cropping_flag == 1 {
        let crop_left = reader.read_ue()?;
        let crop_right = reader.read_ue()?;
        let crop_top = reader.read_ue()?;
        let crop_bottom = reader.read_ue()?;
        width = width.saturating_sub(2 * (crop_left + crop_right));
        height =
            height.saturating_sub((2 - frame_mbs_only_flag as u32) * 2 * (crop_top + crop_bottom));
    }

    Some((width, height))
}

struct BitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_pos: u8,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_pos: 0,
            bit_pos: 0,
        }
    }

    fn read_bit(&mut self) -> Option<u8> {
        if self.byte_pos >= self.data.len() {
            return None;
        }

        let bit = (self.data[self.byte_pos] >> (7 - self.bit_pos)) & 1;
        self.bit_pos += 1;

        if self.bit_pos == 8 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }

        Some(bit)
    }

    fn skip(&mut self, n: usize) -> Option<()> {
        for _ in 0..n {
            self.read_bit()?;
        }
        Some(())
    }

    fn read_ue(&mut self) -> Option<u32> {
        const MAX_LEADING_ZEROS: u32 = 32;

        let mut leading_zeros = 0u32;
        while self.read_bit()? == 0 {
            leading_zeros += 1;
            if leading_zeros >= MAX_LEADING_ZEROS {
                return None;
            }
        }

        if leading_zeros == 0 {
            return Some(0);
        }

        let mut value = 1u32;
        for _ in 0..leading_zeros {
            value = (value << 1) | self.read_bit()? as u32;
        }

        Some(value - 1)
    }

    fn read_se(&mut self) -> Option<i32> {
        let code_num = self.read_ue()?;
        let sign = if code_num & 1 == 1 { 1 } else { -1 };
        Some(sign * ((code_num + 1) >> 1) as i32)
    }
}
