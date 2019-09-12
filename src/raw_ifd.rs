/// Raw IFDs are the disk-stored versions of their counterparts - 
/// they usually only contain the data necessary to point to other sources of data.

use byteorder::{ByteOrder, ReadBytesExt, WriteBytesExt};
use failure::Error;
use std::io::{Seek, SeekFrom};

/// A struct representing a disk-stored IFD value.
#[derive(Debug, Clone, Copy)]
pub struct RawIFDEntry {
    pub tag: u16,
    pub tag_type: u16,
    pub count: u32,
    pub value_or_offset: [u8; 4],
}

impl RawIFDEntry {
    /// Read the entry value from `reader`.
    pub fn from_reader<E: ByteOrder, R: ReadBytesExt>(
        reader: &mut R,
    ) -> Result<Self, std::io::Error> {
        Ok(Self {
            tag: reader.read_u16::<E>()?,
            tag_type: reader.read_u16::<E>()?,
            count: reader.read_u32::<E>()?,
            value_or_offset: {
                let mut buffer = [0; 4];
                reader.read_exact(&mut buffer)?;
                buffer
            },
        })
    }

    /// Write the entry value to `writer`. 
    pub fn to_writer<E: ByteOrder, W: WriteBytesExt>(
        &self,
        writer: &mut W,
    ) -> Result<(), std::io::Error> {
        writer.write_u16::<E>(self.tag)?;
        writer.write_u16::<E>(self.tag_type)?;
        writer.write_u32::<E>(self.count)?;
        writer.write_all(&self.value_or_offset)?;
        Ok(())
    }
}

/// A struct representing a disk-stored IFD.
#[derive(Debug, Clone)]
pub struct RawIFD(pub Vec<RawIFDEntry>);

impl RawIFD {
    /// Read an entire IFD from `reader`
    pub fn from_reader<E: ByteOrder, R: ReadBytesExt>(reader: &mut R) -> Result<Self, Error> {
        let entry_count = reader.read_u16::<E>()? as usize;
        let mut entries = Vec::with_capacity(entry_count);
        for _ in 0..entry_count {
            entries.push(RawIFDEntry::from_reader::<E, R>(reader)?);
        }
        Ok(Self(entries))
    }

    /// Write an entire IFD to `writer`
    pub fn to_writer<E: ByteOrder, W: WriteBytesExt>(&self, writer: &mut W) -> Result<(), Error> {
        assert!(self.0.len() < std::u16::MAX as usize);
        writer.write_u16::<E>(self.0.len() as u16)?;
        for entry in &self.0 {
            entry.to_writer::<E, W>(writer)?;
        }
        Ok(())
    }
}

pub fn read_raw_ifds<E: ByteOrder, R: ReadBytesExt + Seek>(
    reader: &mut R,
) -> Result<Box<[RawIFD]>, Error> {
    let mut ifds = Vec::new();
    'ifd_load: loop {
        let next_ifd_offset = reader.read_u32::<E>()?;
        if next_ifd_offset == 0 {
            break 'ifd_load;
        }
        reader.seek(SeekFrom::Start(next_ifd_offset.into()))?;
        ifds.push(RawIFD::from_reader::<E, R>(reader)?);
    }
    Ok(ifds.into_boxed_slice())
}

pub fn write_raw_ifds<E: ByteOrder, W: WriteBytesExt + Seek>(
    writer: &mut W,
    ifds: &[RawIFD],
) -> Result<(), Error> {
    let mut ifd_iter = ifds.iter().peekable();
    loop {
        if let Some(ifd) = ifd_iter.next() {
            ifd.to_writer::<E, W>(writer)?;
            if ifd_iter.peek().is_some() {
                let current_position = writer.seek(SeekFrom::Current(0))?;
                writer.write_u32::<E>(current_position as u32 + 4)?;
            }
        } else {
            writer.write_u32::<E>(0)?;
            break;
        }
    }
    Ok(())
}
