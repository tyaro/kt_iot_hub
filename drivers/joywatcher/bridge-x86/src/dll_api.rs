use std::ffi::{c_void, CString};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tracing::info;

use crate::connection::{JoyWatcherBridgeApi, JoyWatcherConnectionOptions};
use crate::dll_ffi::{
    ConnectNetCdeclFn, ConnectNetStdcallFn, DEFAULT_READ_USER_ID, DisconnectNetCdeclFn,
    DisconnectNetForceFn, DisconnectNetStdcallFn, JoyWatcherComData1, JwGetTagIds2Fn, JwReadFn,
    TAGSEL2_BUFFER_SIZE, TAG_NAME_SLOT_SIZE, TagSel2Fn, build_tag_name_buffer,
    ensure_bool_like_success, ensure_jwread_success, ensure_pointer_like_success,
    parse_tagsel2_buffer,
};
use crate::dll_symbols::{LibraryHandle, load_symbols};
use crate::protocol::{ReadValuePayload, ResolvedTag};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoyWatcherConnectConvention {
    Cdecl,
    Stdcall,
}

impl JoyWatcherConnectConvention {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cdecl => "cdecl",
            Self::Stdcall => "stdcall",
        }
    }
}

pub struct JoyWatcherDllApi {
    _library: LibraryHandle,
    dll_path: PathBuf,
    connect_net_addr: *mut c_void,
    disconnect_net_addr: *mut c_void,
    disconnect_net_force_fn: DisconnectNetForceFn,
    jw_get_tag_ids2_fn: JwGetTagIds2Fn,
    tag_sel2_fn: TagSel2Fn,
    jw_read_fn: JwReadFn,
    read_user_id: i32,
    read_password: String,
    connect_convention: JoyWatcherConnectConvention,
}

impl JoyWatcherDllApi {
    pub fn new(
        explicit_path: Option<PathBuf>,
        connect_convention: JoyWatcherConnectConvention,
    ) -> Result<Self> {
        let symbols = load_symbols(explicit_path)?;

        info!(
            dll = %symbols.dll_path.display(),
            connect_convention = connect_convention.as_str(),
            connect_symbol = ?symbols.connect_net_addr,
            disconnect_symbol = ?symbols.disconnect_net_addr,
            "JoyWatcher DLL symbols loaded"
        );

        Ok(Self {
            _library: symbols.library,
            dll_path: symbols.dll_path,
            connect_net_addr: symbols.connect_net_addr,
            disconnect_net_addr: symbols.disconnect_net_addr,
            disconnect_net_force_fn: symbols.disconnect_net_force_fn,
            jw_get_tag_ids2_fn: symbols.jw_get_tag_ids2_fn,
            tag_sel2_fn: symbols.tag_sel2_fn,
            jw_read_fn: symbols.jw_read_fn,
            read_user_id: DEFAULT_READ_USER_ID,
            read_password: String::new(),
            connect_convention,
        })
    }

    pub fn dll_path(&self) -> &Path {
        &self.dll_path
    }

    fn call_connect_net(&self) -> isize {
        unsafe {
            match self.connect_convention {
                JoyWatcherConnectConvention::Cdecl => {
                    let func: ConnectNetCdeclFn = std::mem::transmute_copy(&self.connect_net_addr);
                    func()
                }
                JoyWatcherConnectConvention::Stdcall => {
                    let func: ConnectNetStdcallFn = std::mem::transmute_copy(&self.connect_net_addr);
                    func()
                }
            }
        }
    }

    fn call_disconnect_net(&self) -> isize {
        unsafe {
            match self.connect_convention {
                JoyWatcherConnectConvention::Cdecl => {
                    let func: DisconnectNetCdeclFn =
                        std::mem::transmute_copy(&self.disconnect_net_addr);
                    func()
                }
                JoyWatcherConnectConvention::Stdcall => {
                    let func: DisconnectNetStdcallFn =
                        std::mem::transmute_copy(&self.disconnect_net_addr);
                    func()
                }
            }
        }
    }
}

impl JoyWatcherBridgeApi for JoyWatcherDllApi {
    fn mode(&self) -> &'static str {
        "dll"
    }

    fn connect_net(&mut self, options: &JoyWatcherConnectionOptions) -> Result<()> {
        let password = options.password.clone().unwrap_or_default();
        let _ = CString::new(password.as_str()).context("JoyWatcher password contains NUL")?;

        info!(
            connect_convention = self.connect_convention.as_str(),
            endpoint = ?options.endpoint,
            user_id = ?options.user_id,
            password_len = password.len(),
            "Calling JoyWatcher ConnectNet"
        );
        let result = self.call_connect_net();
        info!(
            connect_convention = self.connect_convention.as_str(),
            raw_result = result,
            raw_result_hex = format!("0x{result:08X}"),
            "JoyWatcher ConnectNet returned"
        );
        ensure_pointer_like_success("ConnectNet", result)?;
        self.read_user_id = options.user_id.unwrap_or(DEFAULT_READ_USER_ID);
        self.read_password = password;
        Ok(())
    }

    fn disconnect_net(&mut self) -> Result<()> {
        let result = self.call_disconnect_net();
        info!(
            connect_convention = self.connect_convention.as_str(),
            raw_result = result,
            raw_result_hex = format!("0x{result:08X}"),
            "JoyWatcher DisconnectNet returned"
        );
        ensure_pointer_like_success("DisconnectNet", result)
    }

    fn disconnect_net_force(&mut self) -> Result<()> {
        unsafe { (self.disconnect_net_force_fn)() };
        Ok(())
    }

    fn resolve_tags(&mut self, tags: &[String]) -> Result<Vec<ResolvedTag>> {
        if tags.is_empty() {
            return Ok(Vec::new());
        }

        let names_buffer = build_tag_name_buffer(tags)?;
        let mut ids = vec![0i32; tags.len()];
        let result = unsafe {
            (self.jw_get_tag_ids2_fn)(
                tags.len() as i32,
                names_buffer.as_ptr().cast::<c_void>(),
                TAG_NAME_SLOT_SIZE as i32,
                0,
                ids.as_mut_ptr().cast::<c_void>(),
                std::mem::size_of::<i32>() as i32,
                0,
                -1,
                -1,
            )
        };
        ensure_bool_like_success("JWGetTagIDS2", result)?;

        Ok(tags
            .iter()
            .zip(ids)
            .map(|(tag_path, tag_id)| ResolvedTag {
                tag_path: tag_path.clone(),
                tag_id,
            })
            .collect())
    }

    fn browse_tags(&mut self) -> Result<Vec<String>> {
        let mut buffer = vec![0u8; TAGSEL2_BUFFER_SIZE];
        info!(
            buffer_size = TAGSEL2_BUFFER_SIZE,
            "Calling JoyWatcher TagSel2 (dialog will block until user closes it)"
        );
        let result = unsafe { (self.tag_sel2_fn)(buffer.as_mut_ptr().cast::<i8>()) };
        info!(
            raw_result = result,
            raw_result_hex = format!("0x{result:08X}"),
            "JoyWatcher TagSel2 returned"
        );
        ensure_bool_like_success("TagSel2", result)?;

        parse_tagsel2_buffer(&buffer)
    }

    fn read_tags(&self, tag_ids: &[i32]) -> Result<Vec<ReadValuePayload>> {
        if tag_ids.is_empty() {
            return Ok(Vec::new());
        }

        info!(
            user_id = self.read_user_id,
            password_len = self.read_password.len(),
            tag_count = tag_ids.len(),
            tag_ids = ?tag_ids,
            "Calling JoyWatcher JWRead"
        );

        let password = CString::new(self.read_password.as_str())
            .context("JoyWatcher password contains NUL")?;
        let mut rows = tag_ids
            .iter()
            .map(|tag_id| JoyWatcherComData1::new(*tag_id))
            .collect::<Vec<_>>();

        let result = unsafe {
            (self.jw_read_fn)(
                self.read_user_id,
                password.as_ptr(),
                rows.len() as i32,
                rows.as_mut_ptr(),
            )
        };
        info!(
            raw_result = result,
            raw_result_hex = format!("0x{result:08X}"),
            "JoyWatcher JWRead returned"
        );
        ensure_jwread_success(result)?;

        rows.into_iter()
            .map(|row| row.into_payload())
            .collect::<Result<Vec<_>>>()
    }
}
