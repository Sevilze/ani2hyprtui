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

const SEQUENCE_FLAG: u32 = 0x2;
const ICON_FLAG: u32 = 0x1;

pub struct AniParser;

#[derive(Debug)]
struct AnihHeader {
    size: u32,
    frame_count: u32,
    step_count: u32,
    width: u32,
    height: u32,
    bit_count: u32,
    planes: u32,
    display_rate: u32,
    flags: u32,
}

impl AniParser {
    pub fn can_parse(data: &[u8]) -> bool {
        data.len() >= 12 
            && &data[0..4] == SIGNATURE 
            && &data[8..12] == ANI_TYPE
    }

    pub fn parse(data: &[u8]) -> Result<Vec<CursorFrame>> {
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

        if &ani_type != ANI_TYPE {
            bail!("Not an ACON (animated cursor) RIFF file");
        }

        let header = Self::read_anih_header(&mut cursor)?;
        
        if (header.flags & ICON_FLAG) == 0 {
            bail!("Raw BMP images not supported");
        }

        let mut frames = Vec::new();
        let mut order: Option<Vec<u32>> = None;
        let mut delays: Option<Vec<u32>> = None;

        // Continue reading chunks
        while (cursor.position() as usize) < data.len() {
            let chunk_result = Self::read_chunk(&mut cursor);
            if chunk_result.is_err() {
                break; // End of file
            }
            
            let (chunk_name, chunk_size, chunk_data_start) = chunk_result?;
            
            match &chunk_name[..] {
                LIST_CHUNK => {
                    let mut list_type = [0u8; 4];
                    cursor.read_exact(&mut list_type)?;
                    
                    if &list_type == FRAME_TYPE {
                        frames = Self::read_frames(&mut cursor, data, header.frame_count as usize)?;
                    }
                }
                SEQ_CHUNK => {
                    order = Some(Self::read_seq_chunk(&mut cursor, header.step_count as usize)?);
                }
                RATE_CHUNK => {
                    delays = Some(Self::read_rate_chunk(&mut cursor, header.step_count as usize)?);
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
        let delays = delays.unwrap_or_else(|| vec![header.display_rate; header.step_count as usize]);

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

    fn read_anih_header(cursor: &mut Cursor<&[u8]>) -> Result<AnihHeader> {
        // Find anih chunk
        loop {
            let (name, size, _) = Self::read_chunk(cursor)?;
            if &name == HEADER_CHUNK {
                if size != 36 {
                    bail!("Invalid anih header size");
                }
                break;
            }
            cursor.seek(SeekFrom::Current(size as i64))?;
            if cursor.position() & 1 != 0 {
                cursor.seek(SeekFrom::Current(1))?;
            }
        }

        Ok(AnihHeader {
            size: cursor.read_u32::<LittleEndian>()?,
            frame_count: cursor.read_u32::<LittleEndian>()?,
            step_count: cursor.read_u32::<LittleEndian>()?,
            width: cursor.read_u32::<LittleEndian>()?,
            height: cursor.read_u32::<LittleEndian>()?,
            bit_count: cursor.read_u32::<LittleEndian>()?,
            planes: cursor.read_u32::<LittleEndian>()?,
            display_rate: cursor.read_u32::<LittleEndian>()?,
            flags: cursor.read_u32::<LittleEndian>()?,
        })
    }

    fn read_frames(cursor: &mut Cursor<&[u8]>, full_data: &[u8], count: usize) -> Result<Vec<CursorFrame>> {
        let mut frames = Vec::new();
        
        for _ in 0..count {
            let (name, size, data_start) = Self::read_chunk(cursor)?;
            if &name != ICON_CHUNK {
                bail!("Expected icon chunk in frame list");
            }
            
            let start = data_start as usize;
            let end = start + size as usize;
            if end > full_data.len() {
                bail!("Icon data extends beyond file");
            }
            
            let icon_data = &full_data[start..end];
            let cur_frames = CurParser::parse(icon_data)?;
            
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
