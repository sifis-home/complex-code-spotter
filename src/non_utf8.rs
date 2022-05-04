use encoding_rs::{CoderResult, SHIFT_JIS};

use crate::{Error, Result};

const BUFFER_SIZE: usize = 4096;

#[inline]
pub(crate) fn encode_to_utf8(buf: &[u8]) -> Result<String> {
    let mut buffer_bytes = [0u8; BUFFER_SIZE];
    let buffer_str = std::str::from_utf8_mut(&mut buffer_bytes[..])?;

    let (result, _, _, _) = SHIFT_JIS.new_decoder().decode_to_str(buf, buffer_str, true);

    if matches!(result, CoderResult::InputEmpty) {
        Ok(buffer_str.to_owned())
    } else {
        Err(Error::NonUtf8Conversion)
    }
}
