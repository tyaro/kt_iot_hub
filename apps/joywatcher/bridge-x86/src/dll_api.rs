use std::ffi::{c_void, CString};
use std::mem::size_of;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};

use crate::connection::{JoyWatcherBridgeApi, JoyWatcherConnectionOptions};
use crate::protocol::{MockValue, ReadValuePayload, ResolvedTag};

type ConnectNetFn = unsafe extern "C" fn() -> i32;
type DisconnectNetFn = unsafe extern "C" fn() -> i32;
type DisconnectNetForceFn = unsafe extern "C" fn();
type JwGetTagIds2Fn = unsafe extern "C" fn(
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
type TagSel2Fn = unsafe extern "C" fn(ed: *mut i8) -> i32;
type JwReadFn = unsafe extern "C" fn(
    uid: i32,
    password: *const i8,
    nid: i32,
    data: *mut JoyWatcherComData1,
) -> i32;

const TAG_NAME_SLOT_SIZE: usize = 256;
const TAGSEL2_BUFFER_SIZE: usize = 64 * 1024;

pub struct JoyWatcherDllApi {
    _library: LibraryHandle,
    dll_path: PathBuf,
    connect_net_fn: ConnectNetFn,
    disconnect_net_fn: DisconnectNetFn,
    disconnect_net_force_fn: DisconnectNetForceFn,
    jw_get_tag_ids2_fn: JwGetTagIds2Fn,
    tag_sel2_fn: TagSel2Fn,
    jw_read_fn: JwReadFn,
    read_user_id: i32,
    read_password: String,
}

impl JoyWatcherDllApi {
    pub fn new(explicit_path: Option<PathBuf>) -> Result<Self> {
        let dll_path = resolve_dll_path(explicit_path)?;
        let library = unsafe { LibraryHandle::load(&dll_path)? };
        let connect_net_fn = unsafe { library.load_symbol::<ConnectNetFn>("ConnectNet")? };
        let disconnect_net_fn =
            unsafe { library.load_symbol::<DisconnectNetFn>("DisconnectNet")? };
        let disconnect_net_force_fn = unsafe {
            library.load_symbol::<DisconnectNetForceFn>("DisconnectNetForce")?
        };
        let jw_get_tag_ids2_fn = unsafe { library.load_symbol::<JwGetTagIds2Fn>("JWGetTagIDS2")? };
        let tag_sel2_fn = unsafe { library.load_symbol::<TagSel2Fn>("TagSel2")? };
        let jw_read_fn = unsafe { library.load_symbol::<JwReadFn>("JWRead")? };

        Ok(Self {
            _library: library,
            dll_path,
            connect_net_fn,
            disconnect_net_fn,
            disconnect_net_force_fn,
            jw_get_tag_ids2_fn,
            tag_sel2_fn,
            jw_read_fn,
            read_user_id: 0,
            read_password: String::new(),
        })
    }

    pub fn dll_path(&self) -> &Path {
        &self.dll_path
    }
}

impl JoyWatcherBridgeApi for JoyWatcherDllApi {
    fn mode(&self) -> &'static str {
        "dll"
    }

    fn connect_net(&mut self, options: &JoyWatcherConnectionOptions) -> Result<()> {
        let password = options.password.clone().unwrap_or_default();
        let _ = CString::new(password.as_str()).context("JoyWatcher password contains NUL")?;

        let result = unsafe { (self.connect_net_fn)() };
        ensure_bool_like_success("ConnectNet", result)?;
        self.read_user_id = options.user_id.unwrap_or_default();
        self.read_password = password;
        Ok(())
    }

    fn disconnect_net(&mut self) -> Result<()> {
        let result = unsafe { (self.disconnect_net_fn)() };
        ensure_bool_like_success("DisconnectNet", result)
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
                size_of::<i32>() as i32,
                0,
                -1,
                -1,
            )
        };
        ensure_bool_like_success("JWGetTagIDS2", result)?;

        Ok(tags
            .iter()
            .zip(ids.into_iter())
            .map(|(tag_path, tag_id)| ResolvedTag {
                tag_path: tag_path.clone(),
                tag_id,
            })
            .collect())
    }

    fn browse_tags(&mut self) -> Result<Vec<String>> {
        let mut buffer = vec![0u8; TAGSEL2_BUFFER_SIZE];
        let result = unsafe { (self.tag_sel2_fn)(buffer.as_mut_ptr().cast::<i8>()) };
        ensure_bool_like_success("TagSel2", result)?;

        parse_tagsel2_buffer(&buffer)
    }

    fn read_tags(&self, tag_ids: &[i32]) -> Result<Vec<ReadValuePayload>> {
        if tag_ids.is_empty() {
            return Ok(Vec::new());
        }

        let password = CString::new(self.read_password.as_str())
            .context("JoyWatcher password contains NUL")?;
        let mut rows = tag_ids
            .iter()
            .map(|tag_id| JoyWatcherComData1 {
                col_id: *tag_id,
                raw_value: [0u8; 16],
                dtype: 0,
            })
            .collect::<Vec<_>>();

        let result = unsafe {
            (self.jw_read_fn)(
                self.read_user_id,
                password.as_ptr(),
                rows.len() as i32,
                rows.as_mut_ptr(),
            )
        };
        ensure_bool_like_success("JWRead", result)?;

        rows.into_iter()
            .map(|row| row.into_payload())
            .collect::<Result<Vec<_>>>()
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct JoyWatcherComData1 {
    col_id: i32,
    raw_value: [u8; 16],
    dtype: i8,
}

impl JoyWatcherComData1 {
    const TYPE_ERROR: i8 = -2;
    const TYPE_BIT: i8 = 4;
    const TYPE_STRING: i8 = 5;
    const TYPE_LSTRING: i8 = 8;

    fn into_payload(self) -> Result<ReadValuePayload> {
        let (quality, value) = match self.decode_value() {
            Ok(value) => ("good".to_string(), value),
            Err(error) => ("bad".to_string(), MockValue::String(error.to_string())),
        };

        Ok(ReadValuePayload {
            tag_id: self.col_id,
            quality,
            value,
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
        String::from_utf8_lossy(&self.raw_value[..end]).to_string()
    }
}

fn build_tag_name_buffer(tags: &[String]) -> Result<Vec<u8>> {
    let mut buffer = vec![0u8; tags.len() * TAG_NAME_SLOT_SIZE];

    for (index, tag) in tags.iter().enumerate() {
        let bytes = tag.as_bytes();
        if bytes.len() >= TAG_NAME_SLOT_SIZE {
            return Err(anyhow!(
                "tag path is too long for JWGetTagIDS2 fixed buffer (max {} bytes): {}",
                TAG_NAME_SLOT_SIZE - 1,
                tag
            ));
        }

        let offset = index * TAG_NAME_SLOT_SIZE;
        buffer[offset..offset + bytes.len()].copy_from_slice(bytes);
    }

    Ok(buffer)
}

fn ensure_bool_like_success(function_name: &str, result: i32) -> Result<()> {
    if result == 0 {
        return Err(anyhow!(
            "{} returned 0 (treated as failure in current bridge implementation)",
            function_name
        ));
    }

    Ok(())
}

fn parse_tagsel2_buffer(buffer: &[u8]) -> Result<Vec<String>> {
    let end = buffer.iter().position(|byte| *byte == 0).unwrap_or(buffer.len());
    let text = String::from_utf8_lossy(&buffer[..end]).to_string();

    Ok(text
        .split(['\r', '\n'])
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn resolve_dll_path(explicit_path: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) = explicit_path {
        if path.exists() {
            return Ok(path);
        }

        return Err(anyhow!(
            "JoyWaApi.dll not found at explicit path: {}",
            path.display()
        ));
    }

    if let Ok(configured_path) = std::env::var("JOYWATCHER_DLL_PATH") {
        let path = PathBuf::from(configured_path);
        if path.exists() {
            return Ok(path);
        }
    }

    let candidates = default_dll_candidates();
    candidates
        .into_iter()
        .find(|path| path.exists())
        .ok_or_else(|| anyhow!("JoyWaApi.dll not found in known search roots"))
}

fn default_dll_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(dir) = std::env::var("JOYWATCHER_DLL_DIR") {
        push_unique(&mut candidates, PathBuf::from(dir).join("JoyWaApi.dll"));
    }

    if let Ok(current_dir) = std::env::current_dir() {
        push_unique(&mut candidates, current_dir.join("JoyWaApi.dll"));
        if let Some(parent) = current_dir.parent() {
            push_unique(&mut candidates, parent.join("JoyWaApi.dll"));
        }
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            push_unique(&mut candidates, exe_dir.join("JoyWaApi.dll"));
            if let Some(parent) = exe_dir.parent() {
                push_unique(&mut candidates, parent.join("JoyWaApi.dll"));
            }
        }
    }

    if let Some(repo_root) = find_repo_root() {
        push_unique(&mut candidates, repo_root.join("参考").join("JoyWaApi.dll"));
        push_unique(
            &mut candidates,
            repo_root.join("参考").join("JoyWaApi").join("JoyWaApi.dll"),
        );
        push_unique(
            &mut candidates,
            repo_root
                .join("参考")
                .join("JoyWaApi")
                .join("BC")
                .join("JoyWaApi.dll"),
        );
    }

    if cfg!(windows) {
        if let Ok(windir) = std::env::var("WINDIR").or_else(|_| std::env::var("SystemRoot")) {
            push_unique(
                &mut candidates,
                PathBuf::from(windir).join("SysWOW64").join("JoyWaApi.dll"),
            );
        }
    }

    candidates
}

fn push_unique(vec: &mut Vec<PathBuf>, path: PathBuf) {
    if !vec.contains(&path) {
        vec.push(path);
    }
}

fn find_repo_root() -> Option<PathBuf> {
    let current_dir = std::env::current_dir().ok()?;
    find_ancestor_with(&current_dir, |dir| dir.join("Cargo.toml").exists())
}

fn find_ancestor_with(start: &Path, predicate: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    let mut cursor = Some(start);
    while let Some(path) = cursor {
        if predicate(path) {
            return Some(path.to_path_buf());
        }
        cursor = path.parent();
    }
    None
}

#[cfg(windows)]
struct LibraryHandle(*mut c_void);

#[cfg(windows)]
impl LibraryHandle {
    unsafe fn load(path: &Path) -> Result<Self> {
        let wide_path = to_wide(path);
        let handle = LoadLibraryW(wide_path.as_ptr());
        if handle.is_null() {
            let error = std::io::Error::last_os_error();
            return Err(anyhow!(
                "LoadLibraryW failed for {}: {}",
                path.display(),
                error
            ));
        }

        Ok(Self(handle))
    }

    unsafe fn load_symbol<T: Copy>(&self, symbol_name: &str) -> Result<T> {
        let symbol = CString::new(symbol_name).context("symbol name contains NUL")?;
        let address = GetProcAddress(self.0, symbol.as_ptr());
        if address.is_null() {
            let error = std::io::Error::last_os_error();
            return Err(anyhow!(
                "GetProcAddress failed for {}: {}",
                symbol_name,
                error
            ));
        }

        Ok(std::mem::transmute_copy(&address))
    }
}

#[cfg(windows)]
impl Drop for LibraryHandle {
    fn drop(&mut self) {
        unsafe {
            if !self.0.is_null() {
                let _ = FreeLibrary(self.0);
            }
        }
    }
}

#[cfg(windows)]
fn to_wide(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn LoadLibraryW(lp_lib_file_name: *const u16) -> *mut c_void;
    fn FreeLibrary(h_lib_module: *mut c_void) -> i32;
    fn GetProcAddress(h_module: *mut c_void, lp_proc_name: *const i8) -> *mut c_void;
}

#[cfg(not(windows))]
struct LibraryHandle;

#[cfg(not(windows))]
impl LibraryHandle {
    unsafe fn load(_path: &Path) -> Result<Self> {
        Err(anyhow!("JoyWatcher DLL loading is only supported on Windows"))
    }

    unsafe fn load_symbol<T: Copy>(&self, _symbol_name: &str) -> Result<T> {
        Err(anyhow!("JoyWatcher DLL loading is only supported on Windows"))
    }
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
            col_id: 77,
            raw_value,
            dtype: 0,
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
            col_id: 90,
            raw_value: [0u8; 16],
            dtype: JoyWatcherComData1::TYPE_ERROR,
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
            vec!["Line1/Tank/Level".to_string(), "Line1/Tank/Temp".to_string()]
        );
    }
}