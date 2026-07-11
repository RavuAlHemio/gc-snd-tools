use std::fmt;
use std::io;


/// Extensions for reading binary values from byte streams.
pub trait ReadExt {
    /// Reads an unsigned 8-bit integer from the stream.
    ///
    /// Differences in byte order do not affect single-byte values.
    ///
    /// If reading fails, the function returns the error it encountered. Once reading succeeds, this
    /// function is guaranteed to succeed; decoding the integer itself cannot fail.
    fn read_u8(&mut self) -> Result<u8, io::Error>;

    /// Reads a big-endian unsigned 16-bit integer from the stream.
    ///
    /// Big endian is the byte order where the most significant byte appears first.
    ///
    /// If reading fails, the function returns the error it encountered. Once reading succeeds, this
    /// function is guaranteed to succeed; decoding the integer itself cannot fail.
    fn read_u16_be(&mut self) -> Result<u16, io::Error>;

    /// Reads a big-endian signed two's-complement 16-bit integer from the stream.
    ///
    /// Big endian is the byte order where the most significant byte appears first.
    ///
    /// If reading fails, the function returns the error it encountered. Once reading succeeds, this
    /// function is guaranteed to succeed; decoding the integer itself cannot fail.
    fn read_i16_be(&mut self) -> Result<i16, io::Error>;

    /// Reads a big-endian unsigned 32-bit integer from the stream.
    ///
    /// Big endian is the byte order where the most significant byte appears first.
    ///
    /// If reading fails, the function returns the error it encountered. Once reading succeeds, this
    /// function is guaranteed to succeed; decoding the integer itself cannot fail.
    fn read_u32_be(&mut self) -> Result<u32, io::Error>;

    /// Reads a big-endian 32-bit floating-point number (IEEE binary32) from the stream.
    ///
    /// Big endian is the byte order where the most significant byte appears first.
    ///
    /// If reading fails, the function returns the error it encountered. Once reading succeeds, this
    /// function is guaranteed to succeed; decoding the integer itself cannot fail.
    fn read_f32_be(&mut self) -> Result<f32, io::Error>;
}
impl<R: io::Read> ReadExt for R {
    fn read_u8(&mut self) -> Result<u8, io::Error> {
        let mut ret = 0u8;
        self.read_exact(std::slice::from_mut(&mut ret))?;
        Ok(ret)
    }

    fn read_u16_be(&mut self) -> Result<u16, io::Error> {
        let mut buf = [0u8; 2];
        self.read_exact(&mut buf)?;
        Ok(u16::from_be_bytes(buf))
    }

    fn read_i16_be(&mut self) -> Result<i16, io::Error> {
        let mut buf = [0u8; 2];
        self.read_exact(&mut buf)?;
        Ok(i16::from_be_bytes(buf))
    }

    fn read_u32_be(&mut self) -> Result<u32, io::Error> {
        let mut buf = [0u8; 4];
        self.read_exact(&mut buf)?;
        Ok(u32::from_be_bytes(buf))
    }

    fn read_f32_be(&mut self) -> Result<f32, io::Error> {
        let mut buf = [0u8; 4];
        self.read_exact(&mut buf)?;
        Ok(f32::from_be_bytes(buf))
    }
}


/// Returns the subslice of the given slice that ends at the first zero byte encountered.
///
/// A zero byte is a byte with value 0x00 (not 0x30). The slice is returned without the zero byte.
/// If the slice does not contain a zero byte, the whole slice is returned.
pub fn end_at_first_zero(buf: &[u8]) -> &[u8] {
    let zero_pos = buf.iter().position(|b| *b == 0x00);
    match zero_pos {
        Some(pos) => &buf[0..pos],
        None => buf,
    }
}


/// A wrapper around a byte slice with a [`std::fmt::Display`] implementation outputting strings
/// reminiscent of byte strings in Rust.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ByteStr<'a>(pub &'a [u8]);
impl<'a> fmt::Display for ByteStr<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "b\"")?;
        for b in self.0 {
            match *b {
                b'"' => {
                    write!(f, "\\\"")?;
                },
                b' '..=b'~' => {
                    write!(f, "{}", char::from_u32((*b).into()).unwrap())?;
                },
                b'\n' => {
                    write!(f, "\\n")?;
                },
                b'\r' => {
                    write!(f, "\\r")?;
                },
                b'\t' => {
                    write!(f, "\\t")?;
                },
                other => {
                    write!(f, "\\x{:02X}", other)?;
                },
            }
        }
        write!(f, "\"")
    }
}
