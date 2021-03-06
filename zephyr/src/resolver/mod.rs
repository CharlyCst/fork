//! # Resolver
//!
//! A resolver is a piece responsible for resolving the path of packages and fetching the code,
//! it is used by the Ctx to retrieve imported modules.
use std::fmt;

use crate::ctx::KnownPackage;
use crate::error::ErrorHandler;

/// A unique ID for a file.
///
/// Internally the compiler uses a file ID of 0 when a FileId is needed and one can't be obtained.
/// In theory, this should never leak, sometimes bug happens so it is recommended to avoid using a
/// FileId of 0 when implementing a resolver.
#[derive(Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Copy, Clone)]
pub struct FileId(pub u16);

/// A file can contain either Zephyr code or Zephyr assembly.
#[derive(Debug)]
pub enum FileKind {
    Zephyr,
    Asm,
}

/// A module can be either standalone (inside a single file) or standard (occupate the whole
/// directory).
#[derive(Eq, PartialEq, Debug)]
pub enum ModuleKind {
    Standalone,
    Standard,
}

/// A file prepared to be passed to the AST parser.
pub struct PreparedFile {
    pub code: String,
    pub f_id: FileId,
    pub file_name: String,
    pub kind: FileKind,
}

/// A path to a module from the package root.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct ModulePath {
    pub root: String,
    pub path: Vec<String>,
}

/// A module resolver, used to locate and retrieve code.
pub trait Resolver {
    /// Given a module path return a list of files for that module.
    fn resolve_module(
        &self,
        module: &ModulePath,
        err: &mut impl ErrorHandler,
    ) -> Result<(Vec<PreparedFile>, ModuleKind), ()>;
}

impl ModulePath {
    pub fn from_root(root: String) -> Self {
        Self {
            root,
            path: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn from_known_package(pkg: KnownPackage) -> Self {
        let root = pkg.as_str().to_owned();
        Self {
            root,
            path: Vec::new(),
        }
    }

    pub fn alias(&self) -> &str {
        if let Some(module) = self.path.last() {
            module.as_str()
        } else {
            self.root.as_str()
        }
    }
}

impl fmt::Display for ModulePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut path = Vec::with_capacity(1 + self.path.len());
        path.push(self.root.clone());
        path.extend(self.path.clone());
        write!(f, "{}", path.join("."))
    }
}

impl fmt::Display for FileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
