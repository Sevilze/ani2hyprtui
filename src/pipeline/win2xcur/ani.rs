use anyhow::{Result, bail};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Cursor, Read, Seek, SeekFrom};

use super::cur::{CurParser, CursorFrame};

const SIGNATURE: &[u8] = b"RIFF";
const ANI_TYPE: &[u8] = b"ACON";
const HEADER_CHUNK: &[u8] = b"anih";
const LIST_CHUNK: &[u8] = b"LIST";
const SEQ_CHUNK: &[u8] = b"seq ";
const RATE_CHUNK: &[u8] = b"rate";
const FRAME_TYPE: &[u8] = b"fram";
const ICON_CHUNK: &[u8] = b"icon";

const ICON_FLAG: u32 = 0x1;

pub struct AniParser;

#[derive(Debug)]
struct AnihHeader {
    size: u32,
    frame_count: u32,
    step_count: u32,
    _width: u32,
    _height: u32,
    _bit_count: u32,
    _planes: u32,
    display_rate: u32,
    flags: u32,
}

impl AnihHeader {
    fn validate<F>(&self, _log_fn: F) -> Result<()>
    where
        F: FnMut(String),
    {
        if self.size != 36 {
            bail!("Invalid ANI header size: {}", self.size);
        }

        Ok(())
    }
}

impl AniParser {
    pub fn can_parse(data: &[u8]) -> bool {
        data.len() >= 12 && &data[0..4] == SIGNATURE && &data[8..12] == ANI_TYPE
    }

    pub fn parse<F>(data: &[u8], mut log_fn: F) -> Result<Vec<CursorFrame>>
    where
        F: FnMut(String),
    {
        if !Self::can_parse(data) {
            bail!("Not a valid .ANI file");
        }

        let mut cursor = Cursor::new(data);

        cursor.seek(SeekFrom::Start(0))?;
        let mut sig = [0u8; 4];
        cursor.read_exact(&mut sig)?;
        let _file_size = cursor.read_u32::<LittleEndian>()?;
        let mut ani_type = [0u8; 4];
        cursor.read_exact(&mut ani_type)?;

        if ani_type != ANI_TYPE {
            bail!("Not an ACON (animated cursor) RIFF file");
        }

        let header = Self::read_anih_header(&mut cursor, data)?;
        header.validate(&mut log_fn)?;

        if (header.flags & ICON_FLAG) == 0 {
            bail!("Raw BMP images not supported");
        }

        let mut frames = Vec::new();
        let mut order: Option<Vec<u32>> = None;
        let mut delays: Option<Vec<u32>> = None;

        // Continue reading chunks
        while (cursor.position() as usize) < data.len() {
            let chunk_result =
                Self::read_expected_chunk(&mut cursor, data, &[LIST_CHUNK, SEQ_CHUNK, RATE_CHUNK]);
            if chunk_result.is_err() {
                break; // End of file or no more expected chunks
            }

            let (chunk_name, chunk_size, chunk_data_start) = chunk_result?;

            match &chunk_name[..] {
                LIST_CHUNK => {
                    let mut list_type = [0u8; 4];
                    cursor.read_exact(&mut list_type)?;

                    if list_type == FRAME_TYPE {
                        frames = Self::read_frames(
                            &mut cursor,
                            data,
                            header.frame_count as usize,
                            &mut log_fn,
                        )?;
                    }
                }
                SEQ_CHUNK => {
                    order = Some(Self::read_seq_chunk(
                        &mut cursor,
                        header.step_count as usize,
                    )?);
                }
                RATE_CHUNK => {
                    delays = Some(Self::read_rate_chunk(
                        &mut cursor,
                        header.step_count as usize,
                    )?);
                }
                _ => {
                    // Skip unknown chunk
                    cursor.seek(SeekFrom::Start(chunk_data_start + chunk_size as u64))?;
                }
            }

            // Align to word boundary
            if cursor.position() & 1 != 0 {
                cursor.seek(SeekFrom::Current(1))?;
            }
        }

        // Build final sequence
        let order = order.unwrap_or_else(|| (0..header.frame_count).collect());
        let delays =
            delays.unwrap_or_else(|| vec![header.display_rate; header.step_count as usize]);

        if order.len() != header.step_count as usize {
            bail!("Sequence length mismatch");
        }
        if delays.len() != header.step_count as usize {
            bail!("Rate length mismatch");
        }

        // Create sequence of frames
        let mut sequence = Vec::new();
        for (idx, delay) in order.iter().zip(delays.iter()) {
            if *idx >= frames.len() as u32 {
                bail!("Invalid frame index in sequence");
            }
            let mut frame = frames[*idx as usize].clone();
            frame.delay = ((*delay as f64 / 60.0) * 1000.0) as u32;
            sequence.push(frame);
        }

        Ok(sequence)
    }

    fn read_chunk(cursor: &mut Cursor<&[u8]>) -> Result<([u8; 4], u32, u64)> {
        let mut name = [0u8; 4];
        cursor.read_exact(&mut name)?;
        let size = cursor.read_u32::<LittleEndian>()?;
        let data_start = cursor.position();
        Ok((name, size, data_start))
    }

    fn read_expected_chunk(
        cursor: &mut Cursor<&[u8]>,
        data: &[u8],
        expected: &[&[u8]],
    ) -> Result<([u8; 4], u32, u64)> {
        loop {
            let (name, size, data_start) = Self::read_chunk(cursor)?;

            // Check if this is an expected chunk
            if expected.iter().any(|&exp| name == exp) {
                return Ok((name, size, data_start));
            }

            // Skip this chunk and continue
            cursor.seek(SeekFrom::Start(data_start + size as u64))?;

            if cursor.position() & 1 != 0 {
                cursor.seek(SeekFrom::Current(1))?;
            }

            if cursor.position() as usize >= data.len() {
                bail!("Expected chunk not found, reached end of file");
            }
        }
    }

    fn read_anih_header(cursor: &mut Cursor<&[u8]>, data: &[u8]) -> Result<AnihHeader> {
        // Find anih chunk
        let (_, _size, _) = Self::read_expected_chunk(cursor, data, &[HEADER_CHUNK])?;

        Ok(AnihHeader {
            size: cursor.read_u32::<LittleEndian>()?,
            frame_count: cursor.read_u32::<LittleEndian>()?,
            step_count: cursor.read_u32::<LittleEndian>()?,
            _width: cursor.read_u32::<LittleEndian>()?,
            _height: cursor.read_u32::<LittleEndian>()?,
            _bit_count: cursor.read_u32::<LittleEndian>()?,
            _planes: cursor.read_u32::<LittleEndian>()?,
            display_rate: cursor.read_u32::<LittleEndian>()?,
            flags: cursor.read_u32::<LittleEndian>()?,
        })
    }

    fn read_frames<F>(
        cursor: &mut Cursor<&[u8]>,
        full_data: &[u8],
        count: usize,
        mut log_fn: F,
    ) -> Result<Vec<CursorFrame>>
    where
        F: FnMut(String),
    {
        let mut frames = Vec::new();

        for _ in 0..count {
            let (name, size, data_start) = Self::read_chunk(cursor)?;
            if name != ICON_CHUNK {
                bail!("Expected icon chunk in frame list");
            }

            let start = data_start as usize;
            let end = start + size as usize;
            if end > full_data.len() {
                bail!("Icon data extends beyond file");
            }

            let icon_data = &full_data[start..end];
            let cur_frames = CurParser::parse(icon_data, &mut log_fn)?;

            if let Some(frame) = cur_frames.first() {
                frames.push(frame.clone());
            }

            cursor.seek(SeekFrom::Start(data_start + size as u64))?;
            if cursor.position() & 1 != 0 {
                cursor.seek(SeekFrom::Current(1))?;
            }
        }

        Ok(frames)
    }

    fn read_seq_chunk(cursor: &mut Cursor<&[u8]>, count: usize) -> Result<Vec<u32>> {
        let mut seq = Vec::new();
        for _ in 0..count {
            seq.push(cursor.read_u32::<LittleEndian>()?);
        }
        Ok(seq)
    }

    fn read_rate_chunk(cursor: &mut Cursor<&[u8]>, count: usize) -> Result<Vec<u32>> {
        let mut rates = Vec::new();
        for _ in 0..count {
            rates.push(cursor.read_u32::<LittleEndian>()?);
        }
        Ok(rates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ani_detection() {
        let valid = b"RIFF\x00\x00\x00\x00ACON";
        assert!(AniParser::can_parse(valid));

        let invalid = b"RIFF\x00\x00\x00\x00WAVE";
        assert!(!AniParser::can_parse(invalid));
    }
}
