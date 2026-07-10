use std::io;


/// Extensions for reading binary values from byte streams.
pub trait ReadExt {
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
