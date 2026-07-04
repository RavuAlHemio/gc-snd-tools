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
