//! Programmer-mode values as text: each byte as a Latin-1 character (ASCII) or each
//! 16-bit unit as UTF-16 (Unicode), most significant first, as the macOS calculator
//! shows them.

use crate::calculator::WordSize;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CharMode {
    #[default]
    Ascii,
    Unicode,
}

/// The value's characters. Characters that can't be shown (NUL, control codes, unpaired
/// surrogates) are left out.
pub fn to_chars(bits: u64, word: WordSize, mode: CharMode) -> String {
    let bits = bits & word.mask();
    let printable = |c: &char| !c.is_control();
    match mode {
        CharMode::Ascii => {
            let bytes = bits.to_be_bytes();
            bytes[8 - word.bits() as usize / 8..]
                .iter()
                .map(|&b| char::from(b))
                .filter(printable)
                .collect()
        }
        CharMode::Unicode => {
            let n = (word.bits() as usize / 16).max(1);
            let units = (0..n).rev().map(|i| (bits >> (16 * i)) as u16);
            char::decode_utf16(units)
                .filter_map(Result::ok)
                .filter(printable)
                .collect()
        }
    }
}

/// The value whose characters are `text`, packed most significant first. `None` when
/// the text is empty, a character has no code in `mode` (ASCII: above U+00FF) or the
/// codes need more than 64 bits.
pub fn from_chars(text: &str, mode: CharMode) -> Option<u64> {
    let (width, codes): (u32, Vec<u64>) = match mode {
        CharMode::Ascii => (
            8,
            text.chars()
                .map(|c| u8::try_from(c).ok().map(u64::from))
                .collect::<Option<_>>()?,
        ),
        CharMode::Unicode => (16, text.encode_utf16().map(u64::from).collect()),
    };
    if codes.is_empty() || codes.len() as u32 * width > 64 {
        return None;
    }
    Some(codes.into_iter().fold(0, |acc, c| acc << width | c))
}

#[cfg(test)]
mod tests {
    use super::*;
    use CharMode::*;
    use WordSize::*;

    #[test]
    fn ascii() {
        assert_eq!(to_chars(0x8000_0000_0444_CCAB, Qword, Ascii), "DÌ«");
        assert_eq!(to_chars(0x4869, Qword, Ascii), "Hi");
        assert_eq!(to_chars(0x4869, Byte, Ascii), "i");
        assert_eq!(to_chars(0, Qword, Ascii), "");
        assert_eq!(from_chars("Hi", Ascii), Some(0x4869));
        assert_eq!(from_chars("é", Ascii), Some(0xE9));
        assert_eq!(from_chars("12345678", Ascii), Some(0x3132_3334_3536_3738));
        assert_eq!(from_chars("123456789", Ascii), None);
        assert_eq!(from_chars("€", Ascii), None);
        assert_eq!(from_chars("", Ascii), None);
    }

    #[test]
    fn unicode() {
        assert_eq!(to_chars(0x20AC, Qword, Unicode), "€");
        assert_eq!(from_chars("€", Unicode), Some(0x20AC));
        let smile = from_chars("😀", Unicode).unwrap();
        assert_eq!(smile, 0xD83D_DE00);
        assert_eq!(to_chars(smile, Dword, Unicode), "😀");
        // Only the low unit fits a Word: an unpaired surrogate shows nothing.
        assert_eq!(to_chars(smile, Word, Unicode), "");
        assert_eq!(to_chars(0x41, Byte, Unicode), "A");
        assert_eq!(from_chars("abcde", Unicode), None);
        for s in ["Hi", "€uro", "😀😀"] {
            assert_eq!(to_chars(from_chars(s, Unicode).unwrap(), Qword, Unicode), s);
        }
    }
}
