use thiserror::Error;

#[derive(Debug, Error)]
pub enum TranspileError<'a> {
    #[error("shader has no entry point")]
    NoEntryPoint,
    #[error("requested transpilation target not supported")]
    TargetNotSupported,

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

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[cfg(not(feature = "wgsl-in"))]
    #[error("")]
    _Phantom(&'a ())
}
