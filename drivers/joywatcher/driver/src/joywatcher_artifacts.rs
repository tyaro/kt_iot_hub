use std::path::PathBuf;

use crate::path_utils::{syswow64_dll_path, DLL_FILE_NAME};

const LIB_FILE_NAME: &str = "JoyWaApi.lib";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoyWatcherArtifacts {
    pub dll_candidates: Vec<PathBuf>,
    pub dll_path: Option<PathBuf>,
    pub lib_path: Option<PathBuf>,
    pub sample_wrapper_dll_path: Option<PathBuf>,
}

impl JoyWatcherArtifacts {
    pub fn inspect() -> Self {
        Self::inspect_with_roots(&default_search_roots())
    }

    pub fn inspect_with_roots(roots: &[PathBuf]) -> Self {
        let dll_candidates = roots
            .iter()
            .map(|root| root.join(DLL_FILE_NAME))
            .collect::<Vec<_>>();

        let dll_path = dll_candidates.iter().find(|path| path.exists()).cloned();

        let lib_candidates = roots
            .iter()
            .map(|root| root.join(LIB_FILE_NAME))
            .collect::<Vec<_>>();
        let lib_path = lib_candidates.iter().find(|path| path.exists()).cloned();

        let sample_wrapper_dll_path = roots
            .iter()
            .map(|root| root.join("Project2.dll"))
            .find(|path| path.exists());

        Self {
            dll_candidates,
            dll_path,
            lib_path,
            sample_wrapper_dll_path,
        }
    }

    pub fn status_message(&self) -> String {
        match (&self.dll_path, &self.lib_path, &self.sample_wrapper_dll_path) {
            (Some(dll), Some(lib), _) => format!(
                "JoyWatcher artifacts ready: dll={} lib={}",
                dll.display(),
                lib.display()
            ),
            (Some(dll), None, _) => format!(
                "JoyWatcher DLL found without import library: dll={}",
                dll.display()
            ),
            (None, Some(lib), Some(sample)) => format!(
                "JoyWatcher import library found but runtime DLL is missing: lib={} sample_wrapper={}",
                lib.display(),
                sample.display()
            ),
            (None, Some(lib), None) => format!(
                "JoyWatcher import library found but runtime DLL is missing: lib={}",
                lib.display()
            ),
            (None, None, Some(sample)) => format!(
                "JoyWatcher sample wrapper DLL found, but JoyWaApi.dll is missing: sample_wrapper={}",
                sample.display()
            ),
            (None, None, None) => {
                "JoyWatcher DLL / import library are not present in known search roots".to_string()
            }
        }
    }

    pub fn missing_dll_warning(&self) -> Option<String> {
        if self.dll_path.is_some() {
            return None;
        }

        Some(format!(
            "JoyWaApi.dll not found. Looked in: {}",
            self.dll_candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

fn default_search_roots() -> Vec<PathBuf> {
    syswow64_dll_path()
        .and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_missing_dll_even_when_lib_exists() {
        let roots = vec![PathBuf::from(r"C:\tmp\joywatcher")];
        let artifacts = JoyWatcherArtifacts::inspect_with_roots(&roots);

        assert!(artifacts.dll_path.is_none());
        assert!(artifacts
            .missing_dll_warning()
            .unwrap()
            .contains("JoyWaApi.dll not found"));
    }

    #[test]
    fn status_mentions_missing_assets() {
        let artifacts = JoyWatcherArtifacts {
            dll_candidates: vec![PathBuf::from(r"C:\tmp\joywatcher\JoyWaApi.dll")],
            dll_path: None,
            lib_path: Some(PathBuf::from(r"C:\tmp\joywatcher\JoyWaApi.lib")),
            sample_wrapper_dll_path: Some(PathBuf::from(r"C:\tmp\joywatcher\Project2.dll")),
        };

        let status = artifacts.status_message();
        assert!(status.contains("import library found"));
        assert!(status.contains("Project2.dll"));
    }
}
