//! JoyWatcher DLL の FFI 契約を Rust 側へ写した最小定義。
//!
//! この段階では DLL のリンクや動的ロードはまだ行わない。
//! 目的は以下の 2 点:
//! - ヘッダ由来の型・シンボル名・呼出規約メモを Rust 側へ集約する
//! - 後続の FFI 実装で `TCOM_DATA1` / `JWRead` の値解釈を再利用できるようにする

use anyhow::{anyhow, Result};

pub const REQUIRED_FFI_SYMBOLS: &[&str] = &[
    "ConnectNet",
    "DisconnectNet",
    "DisconnectNetForce",
    "JWGetTagIDS2",
    "JWRead",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoyWatcherCallingConvention {
    Cdecl,
    StdcallVariantInBorlandHeader,
}

impl JoyWatcherCallingConvention {
    pub fn summary(self) -> &'static str {
        match self {
            Self::Cdecl => "JoyWaApiHelp / JoyWApi.h は _cdecl を示している",
            Self::StdcallVariantInBorlandHeader => {
                "BC/JoyWApi.h では ConnectNet / DisconnectNet のみ _stdcall 表記がある"
            }
        }
    }
}

pub fn observed_calling_conventions() -> &'static [JoyWatcherCallingConvention] {
    const CONVENTIONS: &[JoyWatcherCallingConvention] = &[
        JoyWatcherCallingConvention::Cdecl,
        JoyWatcherCallingConvention::StdcallVariantInBorlandHeader,
    ];
    CONVENTIONS
}

#[allow(dead_code)]
pub type ConnectNetFn = unsafe extern "C" fn() -> i32;
#[allow(dead_code)]
pub type DisconnectNetFn = unsafe extern "C" fn() -> i32;
#[allow(dead_code)]
pub type DisconnectNetForceFn = unsafe extern "C" fn();
#[allow(dead_code)]
pub type JwGetTagIds2Fn = unsafe extern "C" fn(
    n_tag: i32,
    src: *const core::ffi::c_void,
    src_size: i32,
    name_offs: i32,
    dest: *mut core::ffi::c_void,
    dest_size: i32,
    id_offs: i32,
    kata_offs: i32,
    len_offs: i32,
) -> i32;
#[allow(dead_code)]
pub type JwReadFn = unsafe extern "C" fn(
    uid: i32,
    password: *const i8,
    nid: i32,
    data: *mut JoyWatcherComData1,
) -> i32;

#[cfg_attr(not(test), allow(dead_code))]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JoyWatcherComData1 {
    pub col_id: i32,
    pub raw_value: [u8; 16],
    pub dtype: i8,
}

#[cfg_attr(not(test), allow(dead_code))]
impl JoyWatcherComData1 {
    pub const TYPE_ERROR: i8 = -2;
    pub const TYPE_BIT: i8 = 4;
    pub const TYPE_STRING: i8 = 5;
    pub const TYPE_LSTRING: i8 = 8;

    pub fn double_value(&self) -> f64 {
        let bytes: [u8; 8] = self.raw_value[..8]
            .try_into()
            .expect("slice with exact length");
        f64::from_le_bytes(bytes)
    }

    pub fn bool_value(&self) -> bool {
        self.raw_value.first().copied().unwrap_or_default() != 0
    }

    pub fn string_value(&self) -> String {
        let end = self
            .raw_value
            .iter()
            .position(|b| *b == 0)
            .unwrap_or(self.raw_value.len());
        String::from_utf8_lossy(&self.raw_value[..end]).to_string()
    }

    pub fn decode_value(&self) -> Result<JoyWatcherReadValue> {
        match self.dtype {
            Self::TYPE_BIT => Ok(JoyWatcherReadValue::Bool(self.bool_value())),
            Self::TYPE_STRING | Self::TYPE_LSTRING => {
                Ok(JoyWatcherReadValue::String(self.string_value()))
            }
            Self::TYPE_ERROR => Err(anyhow!("JoyWatcher returned error dtype for col_id={}.", self.col_id)),
            _ => Ok(JoyWatcherReadValue::Number(self.double_value())),
        }
    }
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq)]
pub enum JoyWatcherReadValue {
    Bool(bool),
    Number(f64),
    String(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_double_value() {
        let mut raw_value = [0u8; 16];
        raw_value[..8].copy_from_slice(&42.5f64.to_le_bytes());

        let data = JoyWatcherComData1 {
            col_id: 1,
            raw_value,
            dtype: 0,
        };

        assert_eq!(data.decode_value().unwrap(), JoyWatcherReadValue::Number(42.5));
    }

    #[test]
    fn decode_bool_value() {
        let mut raw_value = [0u8; 16];
        raw_value[0] = 1;

        let data = JoyWatcherComData1 {
            col_id: 2,
            raw_value,
            dtype: JoyWatcherComData1::TYPE_BIT,
        };

        assert_eq!(data.decode_value().unwrap(), JoyWatcherReadValue::Bool(true));
    }

    #[test]
    fn decode_string_value() {
        let mut raw_value = [0u8; 16];
        raw_value[..5].copy_from_slice(b"TEST\0");

        let data = JoyWatcherComData1 {
            col_id: 3,
            raw_value,
            dtype: JoyWatcherComData1::TYPE_STRING,
        };

        assert_eq!(
            data.decode_value().unwrap(),
            JoyWatcherReadValue::String("TEST".to_string())
        );
    }

    #[test]
    fn decode_error_dtype() {
        let data = JoyWatcherComData1 {
            col_id: 9,
            raw_value: [0u8; 16],
            dtype: JoyWatcherComData1::TYPE_ERROR,
        };

        let error = data.decode_value().expect_err("error dtype should fail");
        assert!(error.to_string().contains("error dtype"));
    }
}