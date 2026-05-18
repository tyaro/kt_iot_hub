use std::path::{Path, PathBuf};

pub const BRIDGE_EXE_NAME: &str = "joywatcher-bridge-x86.exe";
pub const DLL_FILE_NAME: &str = "JoyWaApi.dll";

pub fn bridge_exe_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::<PathBuf>::new();

    if let Ok(explicit_path) = std::env::var("JOYWATCHER_BRIDGE_EXE") {
        push_unique(&mut candidates, PathBuf::from(explicit_path));
    }

    if let Ok(bridge_dir) = std::env::var("JOYWATCHER_BRIDGE_DIR") {
        push_unique(&mut candidates, PathBuf::from(bridge_dir).join(BRIDGE_EXE_NAME));
    }

    if let Some(repo_root) = find_repo_root() {
        push_unique(
            &mut candidates,
            repo_root.join("driver-ui").join("joywatcher").join(BRIDGE_EXE_NAME),
        );
        push_unique(
            &mut candidates,
            repo_root
                .join("target")
                .join("i686-pc-windows-msvc")
                .join("debug")
                .join(BRIDGE_EXE_NAME),
        );
        push_unique(
            &mut candidates,
            repo_root
                .join("target")
                .join("i686-pc-windows-msvc")
                .join("release")
                .join(BRIDGE_EXE_NAME),
        );
    }

    if let Ok(current_dir) = std::env::current_dir() {
        push_unique(&mut candidates, current_dir.join(BRIDGE_EXE_NAME));
        if let Some(parent) = current_dir.parent() {
            push_unique(&mut candidates, parent.join(BRIDGE_EXE_NAME));
        }
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            push_unique(&mut candidates, exe_dir.join(BRIDGE_EXE_NAME));
            if let Some(parent) = exe_dir.parent() {
                push_unique(&mut candidates, parent.join(BRIDGE_EXE_NAME));
            }
        }
    }

    candidates
}

pub fn dll_file_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::<PathBuf>::new();

    if let Ok(configured_path) = std::env::var("JOYWATCHER_DLL_PATH") {
        push_unique(&mut candidates, PathBuf::from(configured_path));
    }

    if let Ok(configured_dir) = std::env::var("JOYWATCHER_DLL_DIR") {
        push_unique(&mut candidates, PathBuf::from(configured_dir).join(DLL_FILE_NAME));
    }

    if let Ok(windir) = std::env::var("WINDIR").or_else(|_| std::env::var("SystemRoot")) {
        push_unique(
            &mut candidates,
            PathBuf::from(windir).join("SysWOW64").join(DLL_FILE_NAME),
        );
    }

    if let Ok(current_dir) = std::env::current_dir() {
        push_unique(&mut candidates, current_dir.join(DLL_FILE_NAME));
        if let Some(parent) = current_dir.parent() {
            push_unique(&mut candidates, parent.join(DLL_FILE_NAME));
        }
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            push_unique(&mut candidates, exe_dir.join(DLL_FILE_NAME));
            if let Some(parent) = exe_dir.parent() {
                push_unique(&mut candidates, parent.join(DLL_FILE_NAME));
            }
        }
    }

    if let Some(repo_root) = find_repo_root() {
        push_unique(&mut candidates, repo_root.join("参考").join(DLL_FILE_NAME));
        push_unique(
            &mut candidates,
            repo_root.join("参考").join("JoyWaApi").join(DLL_FILE_NAME),
        );
        push_unique(
            &mut candidates,
            repo_root
                .join("参考")
                .join("JoyWaApi")
                .join("BC")
                .join(DLL_FILE_NAME),
        );
    }

    candidates
}

pub fn find_repo_root() -> Option<PathBuf> {
    let current_dir = std::env::current_dir().ok()?;
    find_ancestor_with(&current_dir, |dir| dir.join("Cargo.toml").exists())
}

pub fn push_unique(vec: &mut Vec<PathBuf>, path: PathBuf) {
    if !vec.contains(&path) {
        vec.push(path);
    }
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
