use std::ffi::c_void;

use anyhow::{anyhow, Result};

use crate::protocol::{MockValue, ReadValuePayload};

pub(crate) type DisconnectNetForceFn = unsafe extern "C" fn();
pub(crate) type JwGetTagIds2Fn = unsafe extern "C" fn(
    n_tag: i32,
    src: *const c_void,
    src_size: i32,
    name_offs: i32,
    dest: *mut c_void,
    dest_size: i32,
    id_offs: i32,
    kata_offs: i32,
    len_offs: i32,
) -> i32;
pub(crate) type TagSel2Fn = unsafe extern "C" fn(ed: *mut i8) -> i32;
pub(crate) type JwReadFn = unsafe extern "C" fn(
    uid: i32,
    password: *const i8,
    nid: i32,
    data: *mut JoyWatcherComData1,
) -> i32;

pub(crate) type ConnectNetCdeclFn = unsafe extern "C" fn() -> isize;
pub(crate) type DisconnectNetCdeclFn = unsafe extern "C" fn() -> isize;
pub(crate) type ConnectNetStdcallFn = unsafe extern "system" fn() -> isize;
pub(crate) type DisconnectNetStdcallFn = unsafe extern "system" fn() -> isize;

pub(crate) const TAG_NAME_SLOT_SIZE: usize = 256;
pub(crate) const TAGSEL2_BUFFER_SIZE: usize = 1024 * 1024;
pub(crate) const DEFAULT_READ_USER_ID: i32 = 1;
const SHIFT_JIS_CODE_PAGE: u32 = 932;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct JoyWatcherComData1 {
    col_id: i32,
    _padding0: i32,
    raw_value: [u8; 16],
    dtype: i8,
    _padding1: [u8; 7],
}

impl JoyWatcherComData1 {
    const TYPE_ERROR: i8 = -2;
    const TYPE_BIT: i8 = 4;
    const TYPE_STRING: i8 = 5;
    const TYPE_LSTRING: i8 = 8;

    pub(crate) fn new(col_id: i32) -> Self {
        Self {
            col_id,
            _padding0: 0,
            raw_value: [0u8; 16],
            dtype: 0,
            _padding1: [0u8; 7],
        }
    }

    pub(crate) fn into_payload(self) -> Result<ReadValuePayload> {
        let (quality, value) = match self.decode_value() {
            Ok(value) => ("good".to_string(), value),
            Err(error) => ("bad".to_string(), MockValue::String(error.to_string())),
        };

        Ok(ReadValuePayload {
            tag_id: self.col_id,
            quality,
            value,
            dtype: Some(self.dtype),
        })
    }

    fn decode_value(&self) -> Result<MockValue> {
        match self.dtype {
            Self::TYPE_BIT => Ok(MockValue::Bool(self.bool_value())),
            Self::TYPE_STRING | Self::TYPE_LSTRING => Ok(MockValue::String(self.string_value())),
            Self::TYPE_ERROR => Err(anyhow!(
                "JoyWatcher returned error dtype for col_id={}",
                self.col_id
            )),
            _ => Ok(MockValue::Number(self.double_value())),
        }
    }

    fn double_value(&self) -> f64 {
        let bytes: [u8; 8] = self.raw_value[..8]
            .try_into()
            .expect("slice with exact length");
        f64::from_le_bytes(bytes)
    }

    fn bool_value(&self) -> bool {
        self.raw_value.first().copied().unwrap_or_default() != 0
    }

    fn string_value(&self) -> String {
        let end = self
            .raw_value
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(self.raw_value.len());
        decode_shift_jis_lossy(&self.raw_value[..end])
    }
}

pub(crate) fn build_tag_name_buffer(tags: &[String]) -> Result<Vec<u8>> {
    let mut buffer = vec![0u8; tags.len() * TAG_NAME_SLOT_SIZE];

    for (index, tag) in tags.iter().enumerate() {
        let bytes = encode_shift_jis(tag)?;
        if bytes.len() >= TAG_NAME_SLOT_SIZE {
            return Err(anyhow!(
                "tag path is too long for JWGetTagIDS2 fixed buffer (max {} bytes): {}",
                TAG_NAME_SLOT_SIZE - 1,
                tag
            ));
        }

        let offset = index * TAG_NAME_SLOT_SIZE;
        buffer[offset..offset + bytes.len()].copy_from_slice(&bytes);
    }

    Ok(buffer)
}

pub(crate) fn ensure_bool_like_success(function_name: &str, result: i32) -> Result<()> {
    if result == 0 {
        return Err(anyhow!(
            "{} returned 0 (treated as failure in current bridge implementation)",
            function_name
        ));
    }

    Ok(())
}

pub(crate) fn ensure_pointer_like_success(function_name: &str, result: isize) -> Result<()> {
    if result == 0 {
        return Err(anyhow!(
            "{} returned NULL/0 (treated as failure in current bridge implementation)",
            function_name
        ));
    }

    Ok(())
}

pub(crate) fn ensure_jwread_success(result: i32) -> Result<()> {
    if result < 0 {
        return Err(anyhow!(
            "JWRead returned negative status {} (treated as failure in current bridge implementation)",
            result
        ));
    }

    Ok(())
}

pub(crate) fn parse_tagsel2_buffer(buffer: &[u8]) -> Result<Vec<String>> {
    let end = buffer
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(buffer.len());
    let text = decode_shift_jis_lossy(&buffer[..end]);

    Ok(text
        .split(['\r', '\n'])
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn encode_shift_jis(text: &str) -> Result<Vec<u8>> {
    #[cfg(windows)]
    {
        if text.is_empty() {
            return Ok(Vec::new());
        }

        let source = text.encode_utf16().collect::<Vec<u16>>();
        let required = unsafe {
            WideCharToMultiByte(
                SHIFT_JIS_CODE_PAGE,
                0,
                source.as_ptr(),
                source.len() as i32,
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
                std::ptr::null_mut(),
            )
        };
        if required <= 0 {
            return Err(anyhow!(
                "WideCharToMultiByte(cp932) failed while encoding tag path"
            ));
        }

        let mut encoded = vec![0u8; required as usize];
        let written = unsafe {
            WideCharToMultiByte(
                SHIFT_JIS_CODE_PAGE,
                0,
                source.as_ptr(),
                source.len() as i32,
                encoded.as_mut_ptr().cast::<i8>(),
                encoded.len() as i32,
                std::ptr::null(),
                std::ptr::null_mut(),
            )
        };
        if written <= 0 {
            return Err(anyhow!(
                "WideCharToMultiByte(cp932) failed while writing encoded bytes"
            ));
        }

        Ok(encoded)
    }

    #[cfg(not(windows))]
    {
        Ok(text.as_bytes().to_vec())
    }
}

fn decode_shift_jis_lossy(bytes: &[u8]) -> String {
    #[cfg(windows)]
    {
        if bytes.is_empty() {
            return String::new();
        }

        let required = unsafe {
            MultiByteToWideChar(
                SHIFT_JIS_CODE_PAGE,
                0,
                bytes.as_ptr().cast::<i8>(),
                bytes.len() as i32,
                std::ptr::null_mut(),
                0,
            )
        };
        if required <= 0 {
            return String::from_utf8_lossy(bytes).to_string();
        }

        let mut wide = vec![0u16; required as usize];
        let written = unsafe {
            MultiByteToWideChar(
                SHIFT_JIS_CODE_PAGE,
                0,
                bytes.as_ptr().cast::<i8>(),
                bytes.len() as i32,
                wide.as_mut_ptr(),
                wide.len() as i32,
            )
        };
        if written <= 0 {
            return String::from_utf8_lossy(bytes).to_string();
        }

        String::from_utf16_lossy(&wide)
    }

    #[cfg(not(windows))]
    {
        String::from_utf8_lossy(bytes).to_string()
    }
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn MultiByteToWideChar(
        CodePage: u32,
        dwFlags: u32,
        lpMultiByteStr: *const i8,
        cbMultiByte: i32,
        lpWideCharStr: *mut u16,
        cchWideChar: i32,
    ) -> i32;

    fn WideCharToMultiByte(
        CodePage: u32,
        dwFlags: u32,
        lpWideCharStr: *const u16,
        cchWideChar: i32,
        lpMultiByteStr: *mut i8,
        cbMultiByte: i32,
        lpDefaultChar: *const i8,
        lpUsedDefaultChar: *mut i32,
    ) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_tag_name_buffer_writes_zero_terminated_slots() {
        let buffer = build_tag_name_buffer(&[
            "Line1/Tank/Level".to_string(),
            "Line1/Tank/Temp".to_string(),
        ])
        .unwrap();

        assert_eq!(buffer.len(), TAG_NAME_SLOT_SIZE * 2);
        assert_eq!(&buffer[..16], b"Line1/Tank/Level");
        assert_eq!(buffer[16], 0);
        assert_eq!(
            &buffer[TAG_NAME_SLOT_SIZE..TAG_NAME_SLOT_SIZE + 15],
            b"Line1/Tank/Temp"
        );
        assert_eq!(buffer[TAG_NAME_SLOT_SIZE + 15], 0);
    }

    #[test]
    fn com_data_decodes_numeric_value() {
        let mut raw_value = [0u8; 16];
        raw_value[..8].copy_from_slice(&42.25f64.to_le_bytes());

        let payload = JoyWatcherComData1 {
            raw_value,
            ..JoyWatcherComData1::new(77)
        }
        .into_payload()
        .unwrap();

        assert_eq!(payload.tag_id, 77);
        assert_eq!(payload.quality, "good");
        assert_eq!(payload.value, MockValue::Number(42.25));
    }

    #[test]
    fn com_data_converts_error_dtype_to_bad_quality() {
        let payload = JoyWatcherComData1 {
            dtype: JoyWatcherComData1::TYPE_ERROR,
            ..JoyWatcherComData1::new(90)
        }
        .into_payload()
        .unwrap();

        assert_eq!(payload.tag_id, 90);
        assert_eq!(payload.quality, "bad");
        match payload.value {
            MockValue::String(message) => assert!(message.contains("error dtype")),
            other => panic!("expected error string payload, got {other:?}"),
        }
    }

    #[test]
    fn parse_tagsel2_buffer_splits_crlf_lines() {
        let buffer = b"Line1/Tank/Level\r\nLine1/Tank/Temp\r\n\0extra";
        let items = parse_tagsel2_buffer(buffer).unwrap();

        assert_eq!(
            items,
            vec![
                "Line1/Tank/Level".to_string(),
                "Line1/Tank/Temp".to_string()
            ]
        );
    }

    #[cfg(windows)]
    #[test]
    fn parse_tagsel2_buffer_decodes_shift_jis_japanese() {
        let buffer = [
            0x93, 0xFA, 0x96, 0x7B, 0x8C, 0xEA, b'/', b'T', b'a', b'g', 0x00,
        ];
        let items = parse_tagsel2_buffer(&buffer).unwrap();

        assert_eq!(items, vec!["日本語/Tag".to_string()]);
    }

    #[test]
    fn com_data_layout_matches_vendor_x86_definition() {
        use std::mem::{size_of, MaybeUninit};

        let value = MaybeUninit::<JoyWatcherComData1>::uninit();
        let base = value.as_ptr();
        let col_id_offset = unsafe { std::ptr::addr_of!((*base).col_id) as usize - base as usize };
        let raw_value_offset =
            unsafe { std::ptr::addr_of!((*base).raw_value) as usize - base as usize };
        let dtype_offset = unsafe { std::ptr::addr_of!((*base).dtype) as usize - base as usize };

        assert_eq!(col_id_offset, 0);
        assert_eq!(raw_value_offset, 8);
        assert_eq!(dtype_offset, 24);
        assert_eq!(size_of::<JoyWatcherComData1>(), 32);
    }
}
