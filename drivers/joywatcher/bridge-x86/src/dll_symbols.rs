use std::ffi::{c_void, CString};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};

use crate::dll_ffi::{DisconnectNetForceFn, JwGetTagIds2Fn, JwReadFn, TagSel2Fn};
use crate::path_utils::DLL_FILE_NAME;

pub(crate) struct LoadedJoyWatcherSymbols {
    pub(crate) library: LibraryHandle,
    pub(crate) dll_path: PathBuf,
    pub(crate) connect_net_addr: *mut c_void,
    pub(crate) disconnect_net_addr: *mut c_void,
    pub(crate) disconnect_net_force_fn: DisconnectNetForceFn,
    pub(crate) jw_get_tag_ids2_fn: JwGetTagIds2Fn,
    pub(crate) tag_sel2_fn: TagSel2Fn,
    pub(crate) jw_read_fn: JwReadFn,
}

pub(crate) fn load_symbols(explicit_path: Option<PathBuf>) -> Result<LoadedJoyWatcherSymbols> {
    let dll_path = resolve_dll_path(explicit_path)?;
    let library = unsafe { LibraryHandle::load(&dll_path)? };

    let connect_net_addr = unsafe { library.load_raw_symbol("ConnectNet")? };
    let disconnect_net_addr = unsafe { library.load_raw_symbol("DisconnectNet")? };
    let disconnect_net_force_fn =
        unsafe { library.load_symbol::<DisconnectNetForceFn>("DisconnectNetForce")? };
    let jw_get_tag_ids2_fn = unsafe { library.load_symbol::<JwGetTagIds2Fn>("JWGetTagIDS2")? };
    let tag_sel2_fn = unsafe { library.load_symbol::<TagSel2Fn>("TagSel2")? };
    let jw_read_fn = unsafe { library.load_symbol::<JwReadFn>("JWRead")? };

    Ok(LoadedJoyWatcherSymbols {
        library,
        dll_path,
        connect_net_addr,
        disconnect_net_addr,
        disconnect_net_force_fn,
        jw_get_tag_ids2_fn,
        tag_sel2_fn,
        jw_read_fn,
    })
}

fn resolve_dll_path(explicit_path: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) = explicit_path {
        if path.exists() {
            return Ok(path);
        }

        return Err(anyhow!(
            "{} not found at explicit path: {}",
            DLL_FILE_NAME,
            path.display()
        ));
    }

    crate::path_utils::dll_file_candidates()
        .into_iter()
        .find(|path| path.exists())
        .ok_or_else(|| anyhow!("{} not found in known search roots", DLL_FILE_NAME))
}

#[cfg(windows)]
pub(crate) struct LibraryHandle(*mut c_void);

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
        let address = self.load_raw_symbol(symbol_name)?;
        Ok(std::mem::transmute_copy(&address))
    }

    unsafe fn load_raw_symbol(&self, symbol_name: &str) -> Result<*mut c_void> {
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

        Ok(address)
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
pub(crate) struct LibraryHandle;

#[cfg(not(windows))]
impl LibraryHandle {
    unsafe fn load(_path: &Path) -> Result<Self> {
        Err(anyhow!(
            "JoyWatcher DLL loading is only supported on Windows"
        ))
    }

    unsafe fn load_symbol<T: Copy>(&self, _symbol_name: &str) -> Result<T> {
        Err(anyhow!(
            "JoyWatcher DLL loading is only supported on Windows"
        ))
    }

    unsafe fn load_raw_symbol(&self, _symbol_name: &str) -> Result<*mut c_void> {
        Err(anyhow!(
            "JoyWatcher DLL loading is only supported on Windows"
        ))
    }
}
