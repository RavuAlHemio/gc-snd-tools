use std::io::{self, Read, Seek, SeekFrom, Write};

use arrayvec::ArrayVec;

use crate::array_vec_slice::ArrayVecSlice;


const MAX_TEXT_SECTIONS: usize = 7;
const MAX_DATA_SECTIONS: usize = 11;


#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct DolHeader {
    pub text_offsets: ArrayVec<u32, MAX_TEXT_SECTIONS>,
    pub data_offsets: ArrayVec<u32, MAX_DATA_SECTIONS>,

    pub text_addresses: ArrayVec<u32, MAX_TEXT_SECTIONS>,
    pub data_addresses: ArrayVec<u32, MAX_DATA_SECTIONS>,

    pub text_sizes: ArrayVec<u32, MAX_TEXT_SECTIONS>,
    pub data_sizes: ArrayVec<u32, MAX_DATA_SECTIONS>,

    pub bss_address: u32,
    pub bss_size: u32,
    pub entry_point: u32,
    pub padding: [u32; 7],
}
impl DolHeader {
    pub fn to_bytes(&self) -> [u8; 256] {
        let mut u32_buf: ArrayVec<u32, 64> = ArrayVec::new();

        let slices = [
            ArrayVecSlice::new(&self.text_offsets),
            ArrayVecSlice::new(&self.data_offsets),
            ArrayVecSlice::new(&self.text_addresses),
            ArrayVecSlice::new(&self.data_addresses),
            ArrayVecSlice::new(&self.text_sizes),
            ArrayVecSlice::new(&self.data_sizes),
        ];
        for slice in slices {
            for &value in slice.as_ref() {
                u32_buf.push(value);
            }
            for _ in slice.as_ref().len()..slice.capacity() {
                u32_buf.push(0);
            }
        }

        let values = [
            self.bss_address,
            self.bss_size,
            self.entry_point,
        ];
        for value in values {
            u32_buf.push(value);
        }

        for value in self.padding {
            u32_buf.push(value);
        }

        assert_eq!(u32_buf.len(), u32_buf.capacity());

        let mut buf: ArrayVec<u8, 256> = ArrayVec::new();
        for value in u32_buf {
            buf.extend(value.to_be_bytes());
        }

        buf.into_inner().unwrap()
    }

    pub fn try_read<R: Read>(reader: &mut R) -> Result<Self, io::Error> {
        let mut buf = [0u8; 256];
        reader.read_exact(&mut buf)?;
        Ok(Self::from_bytes(&buf))
    }

    pub fn from_bytes(buf: &[u8]) -> Self {
        assert_eq!(buf.len(), 256);

        let mut u32_buf = [0u32; 64];
        for (chunk, u32_val) in buf.chunks(4).zip(u32_buf.as_mut_slice()) {
            *u32_val = u32::from_be_bytes(chunk.try_into().unwrap());
        }

        let mut i = 0;

        let mut text_offsets = ArrayVec::new();
        for &value in &u32_buf[i..i+MAX_TEXT_SECTIONS] {
            text_offsets.push(value);
        }
        i += MAX_TEXT_SECTIONS;

        let mut data_offsets = ArrayVec::new();
        for &value in &u32_buf[i..i+MAX_DATA_SECTIONS] {
            data_offsets.push(value);
        }
        i += MAX_DATA_SECTIONS;

        let mut text_addresses = ArrayVec::new();
        for &value in &u32_buf[i..i+MAX_TEXT_SECTIONS] {
            text_addresses.push(value);
        }
        i += MAX_TEXT_SECTIONS;

        let mut data_addresses = ArrayVec::new();
        for &value in &u32_buf[i..i+MAX_DATA_SECTIONS] {
            data_addresses.push(value);
        }
        i += MAX_DATA_SECTIONS;

        let mut text_sizes = ArrayVec::new();
        for &value in &u32_buf[i..i+MAX_TEXT_SECTIONS] {
            text_sizes.push(value);
        }
        i += MAX_TEXT_SECTIONS;

        let mut data_sizes = ArrayVec::new();
        for &value in &u32_buf[i..i+MAX_DATA_SECTIONS] {
            data_sizes.push(value);
        }
        i += MAX_DATA_SECTIONS;

        let bss_address = u32_buf[i];
        let bss_size = u32_buf[i+1];
        let entry_point = u32_buf[i+2];
        i += 3;

        let mut padding = [0u32; 7];
        padding.copy_from_slice(&u32_buf[i..i+7]);

        while text_offsets.last() == Some(&0) && text_addresses.last() == Some(&0) && text_sizes.last() == Some(&0) {
            text_offsets.pop();
            text_addresses.pop();
            text_sizes.pop();
        }

        while data_offsets.last() == Some(&0) && data_addresses.last() == Some(&0) && data_sizes.last() == Some(&0) {
            data_offsets.pop();
            data_addresses.pop();
            data_sizes.pop();
        }

        Self {
            text_offsets,
            data_offsets,
            text_addresses,
            data_addresses,
            text_sizes,
            data_sizes,
            bss_address,
            bss_size,
            entry_point,
            padding,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct DolFile {
    pub text_sections: ArrayVec<SectionWithData, MAX_TEXT_SECTIONS>,
    pub data_sections: ArrayVec<SectionWithData, MAX_DATA_SECTIONS>,
    pub bss: SectionLocation,
    pub entry_point: u32,
}
impl DolFile {
    pub fn try_write<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        // assemble header
        let mut header = DolHeader {
            text_offsets: ArrayVec::new(),
            data_offsets: ArrayVec::new(),
            text_addresses: ArrayVec::new(),
            data_addresses: ArrayVec::new(),
            text_sizes: ArrayVec::new(),
            data_sizes: ArrayVec::new(),
            bss_address: self.bss.address,
            bss_size: self.bss.size,
            entry_point: self.entry_point,
            padding: [0u32; 7],
        };

        let mut cur_offset = 256;

        for text_section in &self.text_sections {
            let data_len = text_section.data.len();
            let data_padded_len: u32 = (((data_len + (4 - 1)) / 4) * 4).try_into().unwrap();

            header.text_offsets.push(cur_offset);
            header.text_addresses.push(text_section.address);
            header.text_sizes.push(data_padded_len);

            cur_offset += data_padded_len;
        }
        for data_section in &self.data_sections {
            let data_len = data_section.data.len();
            let data_padded_len: u32 = (((data_len + (4 - 1)) / 4) * 4).try_into().unwrap();

            header.data_offsets.push(cur_offset);
            header.data_addresses.push(data_section.address);
            header.data_sizes.push(data_padded_len);

            cur_offset += data_padded_len;
        }

        // output header
        let header_bytes = header.to_bytes();
        writer.write_all(&header_bytes)?;

        // output sections (with padding to 4 bytes)
        for text_section in &self.text_sections {
            let padding_array = [0u8; 4];
            let padding_len = (4 - (text_section.data.len() % 4)) % 4;
            writer.write_all(&text_section.data)?;
            writer.write_all(&padding_array[..padding_len])?;
        }
        for data_section in &self.data_sections {
            let padding_array = [0u8; 4];
            let padding_len = (4 - (data_section.data.len() % 4)) % 4;
            writer.write_all(&data_section.data)?;
            writer.write_all(&padding_array[..padding_len])?;
        }

        Ok(())
    }

    pub fn try_read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let header = DolHeader::try_read(reader)?;

        let text_section_values = header.text_addresses.iter()
            .zip(header.text_offsets.iter())
            .zip(header.text_sizes.iter())
            .map(|((a, o), s)| (*a, *o, *s));
        let mut text_sections = ArrayVec::new();
        for (address, offset, size_u32s) in text_section_values {
            reader.seek(SeekFrom::Start(offset.into()))?;
            let size_u8s = usize::try_from(size_u32s).unwrap() * 4;
            let mut data = vec![0u8; size_u8s];
            reader.read_exact(&mut data)?;

            text_sections.push(SectionWithData {
                address,
                data,
            })
        }

        let data_section_values = header.data_addresses.iter()
            .zip(header.data_offsets.iter())
            .zip(header.data_sizes.iter())
            .map(|((a, o), s)| (*a, *o, *s));
        let mut data_sections = ArrayVec::new();
        for (address, offset, size_u32s) in data_section_values {
            reader.seek(SeekFrom::Start(offset.into()))?;
            let size_u8s = usize::try_from(size_u32s).unwrap() * 4;
            let mut data = vec![0u8; size_u8s];
            reader.read_exact(&mut data)?;

            data_sections.push(SectionWithData {
                address,
                data,
            })
        }

        let ret = DolFile {
            text_sections,
            data_sections,
            bss: SectionLocation {
                address: header.bss_address,
                size: header.bss_size,
            },
            entry_point: header.entry_point,
        };
        Ok(ret)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct SectionLocation {
    pub address: u32,
    pub size: u32,
}

#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct SectionWithData {
    pub address: u32,
    pub data: Vec<u8>,
}
