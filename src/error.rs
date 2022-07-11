use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::path::PathBuf;
use thiserror::Error;

pub struct VecErr<T: Error> {
    pub inner: Vec<T>,
}

impl<T: Error> Debug for VecErr<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(&self.inner).finish()
    }
}

impl<T: Error + Display> Display for VecErr<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(&self.inner).finish()
    }
}

impl<T: Error> std::error::Error for VecErr<T> {}

impl<T: Error> From<Vec<T>> for VecErr<T> {
    fn from(inner: Vec<T>) -> Self {
        VecErr { inner }
    }
}

#[derive(Debug, Error)]
pub enum SourceError {
    #[error("unhandled shader stage")]
    UnhandledShaderStage,
    #[cfg(feature = "wgsl-in")]
    #[error(transparent)]
    WGSLParse(#[from] naga::front::wgsl::ParseError),
    #[cfg(feature = "glsl-in")]
    #[error("unable to parse GLSL: {0:#?}")]
    GLSLParse(#[from] VecErr<naga::front::glsl::Error>),
    #[cfg(feature = "spv-in")]
    #[error(transparent)]
    SPVParse(#[from] naga::front::spv::Error),
    #[error("unable to validate shader: {0}")]
    Validation(PathBuf),
}

#[derive(Debug, Error)]
pub enum TranspileError<'a> {
    #[error("shader has no entry point")]
    NoEntryPoint,
    #[error("source file transpilation not supported")]
    SourceNotSupported,
    #[error("requested transpilation target not supported")]
    TargetNotSupported,
    #[error("unhandled shader stage")]
    UnhandledShaderStage,

    #[cfg(feature = "wgsl-in")]
    #[error("{0:?}")]
    WGSLFront(naga::front::wgsl::Error<'a>),
    #[cfg(feature = "glsl-in")]
    #[error(transparent)]
    GLSLFront(#[from] naga::front::glsl::Error),
    #[cfg(feature = "spv-in")]
    #[error(transparent)]
    SPVFront(#[from] naga::front::spv::Error),

    #[cfg(feature = "glsl-out")]
    #[error(transparent)]
    GLSLBack(#[from] naga::back::glsl::Error),
    #[cfg(feature = "wgsl-out")]
    #[error(transparent)]
    WGSLBack(#[from] naga::back::wgsl::Error),
    #[cfg(feature = "spv-out")]
    #[error(transparent)]
    SPVBack(#[from] naga::back::spv::Error),
    #[cfg(feature = "hlsl-out")]
    #[error(transparent)]
    HLSLBack(#[from] naga::back::hlsl::Error),
    #[cfg(feature = "msl-out")]
    #[error(transparent)]
    MSLBack(#[from] naga::back::msl::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Parse(#[from] SourceError),

    #[cfg(not(feature = "wgsl-in"))]
    #[error("")]
    _Phantom(&'a ()),
}
