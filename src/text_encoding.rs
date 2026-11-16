use std::borrow::Cow;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextEncoding {
    #[default]
    Utf8,
    Utf16Le,
    Utf16Be,
    Windows949,
    EucKr,
    Windows31J,
    ShiftJis,
    Gbk,
    Gb18030,
    Big5,
    Windows1252,
}
impl gpui::Global for TextEncoding {}
impl TextEncoding {
    pub const ALL: [Self; 11] = [
        Self::Utf8,
        Self::Utf16Le,
        Self::Utf16Be,
        Self::Windows949,
        Self::EucKr,
        Self::Windows31J,
        Self::ShiftJis,
        Self::Gbk,
        Self::Gb18030,
        Self::Big5,
        Self::Windows1252,
    ];
    pub fn current(cx: &gpui::App) -> Self {
        cx.try_global::<Self>().copied().unwrap_or_default()
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Utf8 => "UTF-8",
            Self::Utf16Le => "UTF-16LE",
            Self::Utf16Be => "UTF-16BE",
            Self::Windows949 => "windows-949",
            Self::EucKr => "EUC-KR",
            Self::Windows31J => "Windows-31J",
            Self::ShiftJis => "Shift_JIS",
            Self::Gbk => "GBK",
            Self::Gb18030 => "GB18030",
            Self::Big5 => "Big5",
            Self::Windows1252 => "windows-1252",
        }
    }
    pub fn is_utf16(self) -> bool {
        matches!(self, Self::Utf16Le | Self::Utf16Be)
    }
    pub fn decode(self, bytes: &[u8]) -> Option<Cow<'_, str>> {
        if self == Self::Utf8 {
            // 부분 쓰기의 원문 바이트를 보존하도록 UTF-8 BOM도 유지한다.
            return std::str::from_utf8(bytes).ok().map(Cow::Borrowed);
        }
        let decoder = match self {
            Self::Utf8 => unreachable!(),
            Self::Utf16Le => encoding_rs::UTF_16LE,
            Self::Utf16Be => encoding_rs::UTF_16BE,
            // WHATWG 호환 디코더는 한국어/일본어 Windows 확장도 포함한다.
            Self::Windows949 | Self::EucKr => encoding_rs::EUC_KR,
            Self::Windows31J | Self::ShiftJis => encoding_rs::SHIFT_JIS,
            Self::Gbk => encoding_rs::GBK,
            Self::Gb18030 => encoding_rs::GB18030,
            Self::Big5 => encoding_rs::BIG5,
            Self::Windows1252 => encoding_rs::WINDOWS_1252,
        };
        let bytes = if self.is_utf16() {
            if !bytes.len().is_multiple_of(2) {
                return None;
            }
            let (bom, other): (&[u8], &[u8]) = if self == Self::Utf16Le {
                (&[0xff, 0xfe], &[0xfe, 0xff])
            } else {
                (&[0xfe, 0xff], &[0xff, 0xfe])
            };
            if bytes.starts_with(other) {
                return None;
            }
            bytes.strip_prefix(bom).unwrap_or(bytes)
        } else {
            bytes
        };
        decoder.decode_without_bom_handling_and_without_replacement(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::TextEncoding::*;
    #[test]
    fn decodes_fixed_language_bytes_for_every_menu_choice() {
        for (encoding, bytes, expected) in [
            (Utf8, "한글".as_bytes(), "한글"),
            (Utf16Le, &[0xff, 0xfe, 0x5c, 0xd5, 0x00, 0xae], "한글"),
            (Utf16Be, &[0xfe, 0xff, 0xd5, 0x5c, 0xae, 0x00], "한글"),
            (Windows949, &[0xc7, 0xd1, 0xb1, 0xdb], "한글"),
            (EucKr, &[0xc7, 0xd1, 0xb1, 0xdb], "한글"),
            (Windows31J, &[0x93, 0xfa, 0x96, 0x7b], "日本"),
            (ShiftJis, &[0x93, 0xfa, 0x96, 0x7b], "日本"),
            (Gbk, &[0xd6, 0xd0, 0xce, 0xc4], "中文"),
            (Gb18030, &[0x90, 0x30, 0x81, 0x30], "𐀀"),
            (Big5, &[0xa4, 0xa4, 0xa4, 0xe5], "中文"),
            (Windows1252, &[0x80, 0xe9], "€é"),
        ] {
            assert_eq!(
                encoding.decode(bytes).as_deref(),
                Some(expected),
                "{}",
                encoding.name()
            );
        }
    }
    #[test]
    fn rejects_invalid_sequences_and_keeps_utf8_bom_and_whitespace() {
        assert!(Utf8.decode(&[0xff]).is_none());
        assert!(Windows949.decode(&[0xc7]).is_none());
        assert!(ShiftJis.decode(&[0x93]).is_none());
        assert!(Utf16Le.decode(b"a").is_none());
        assert!(Utf16Le.decode(&[0x00, 0xd8]).is_none());
        assert!(Utf16Be.decode(&[0xff, 0xfe, 0, 0]).is_none());
        assert!(Utf16Le.decode(&[0xfe, 0xff, 0, 0]).is_none());
        assert_eq!(
            Utf8.decode(b"\xef\xbb\xbf \t\r\n").as_deref(),
            Some("\u{feff} \t\r\n")
        );
        for encoding in super::TextEncoding::ALL {
            assert_eq!(encoding.decode(b"").as_deref(), Some(""));
        }
    }
}
