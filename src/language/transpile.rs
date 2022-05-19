use crate::config::Config;
use crate::error::{SourceError, TranspileError};
use crate::language::codegen::CodegenData;
use crate::shader::{Shader, ShaderCode};
#[allow(unused_imports)]
use crate::util::LogResult;
use crate::util::{file_prefix, Name};
use naga::{EntryPoint, Module, ShaderStage};
#[cfg(feature = "config-file")]
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
#[non_exhaustive]
#[cfg_attr(feature = "config-file", derive(Serialize, Deserialize))]
pub enum ShaderLanguage {
    WGSL,
    GLSL,
    SPV,
    HLSL,
    MSL,
}

impl ShaderLanguage {
    // I swear there's a macro that does most of this...
    pub const COUNT: usize = 5;

    pub const ALL: [ShaderLanguage; ShaderLanguage::COUNT] = [
        ShaderLanguage::WGSL,
        ShaderLanguage::GLSL,
        ShaderLanguage::SPV,
        ShaderLanguage::HLSL,
        ShaderLanguage::MSL,
    ];

    #[warn(unreachable_code)]
    pub fn from_file_name(path: impl AsRef<Path>) -> Option<ShaderLanguage> {
        let ext = path
            .as_ref()
            .extension()
            .and_then(|os_str| os_str.to_str())?;

        Some(match ext.to_ascii_lowercase().as_str() {
            #[cfg(feature = "wgsl-in")]
            "wgsl" => ShaderLanguage::WGSL,
            #[cfg(feature = "glsl-in")]
            "glsl" | "vs" | "fs" | "cs" | "vert" | "frag" | "comp" => {
                ShaderLanguage::GLSL
            }
            #[cfg(feature = "spv-in")]
            "spv" => ShaderLanguage::SPV,
            _ => return None,
        })
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            ShaderLanguage::WGSL => "wgsl",
            ShaderLanguage::GLSL => "glsl",
            ShaderLanguage::SPV => "spv",
            ShaderLanguage::HLSL => "hlsl",
            ShaderLanguage::MSL => "msl",
        }
    }

    pub fn to_uppercase_str(&self) -> &'static str {
        match self {
            ShaderLanguage::WGSL => "WGSL",
            ShaderLanguage::GLSL => "GLSL",
            ShaderLanguage::SPV => "SPV",
            ShaderLanguage::HLSL => "HLSL",
            ShaderLanguage::MSL => "MSL",
        }
    }

    pub fn is_binary(&self) -> bool {
        *self == ShaderLanguage::SPV
    }

    pub(crate) fn get_ext(&self, stage: Option<ShaderStage>) -> &'static str {
        match self {
            ShaderLanguage::WGSL => match stage {
                Some(ShaderStage::Vertex) => "vert.wgsl",
                Some(ShaderStage::Fragment) => "frag.wgsl",
                Some(ShaderStage::Compute) => "comp.wgsl",
                None => "wgsl",
            },
            ShaderLanguage::GLSL => match stage {
                Some(ShaderStage::Vertex) => "vert.glsl",
                Some(ShaderStage::Fragment) => "frag.glsl",
                Some(ShaderStage::Compute) => "comp.glsl",
                None => "glsl",
            },
            ShaderLanguage::SPV => match stage {
                Some(ShaderStage::Vertex) => "v.spv",
                Some(ShaderStage::Fragment) => "f.spv",
                Some(ShaderStage::Compute) => "c.spv",
                None => "spv",
            },
            ShaderLanguage::HLSL => match stage {
                Some(ShaderStage::Vertex) => "vert.hlsl",
                Some(ShaderStage::Fragment) => "frag.hlsl",
                Some(ShaderStage::Compute) => "comp.hlsl",
                None => "hlsl",
            },
            ShaderLanguage::MSL => match stage {
                Some(ShaderStage::Vertex) => "vert.msl",
                Some(ShaderStage::Fragment) => "frag.msl",
                Some(ShaderStage::Compute) => "comp.msl",
                None => "msl",
            },
        }
    }

    pub fn parse(self, shader: &mut Shader) -> Result<&Module, SourceError> {
        if shader.module.is_some() {
            return Ok(shader.module.as_ref().unwrap());
        }

        shader.module = Some({
            let source = shader.source.as_ref().expect("no shader source");

            match self {
                #[cfg(feature = "spv-in")]
                ShaderLanguage::SPV => {
                    use naga::front::spv;

                    let options = spv::Options::default();
                    spv::parse_u8_slice(source.unwrap_binary(), &options)?
                }
                #[cfg(feature = "wgsl-in")]
                ShaderLanguage::WGSL => {
                    naga::front::wgsl::parse_str(source.unwrap_text())?
                }
                #[cfg(feature = "glsl-in")]
                ShaderLanguage::GLSL => {
                    use naga::front::glsl;

                    let stage = shader
                        .source_stage
                        .ok_or(TranspileError::UnhandledShaderStage)?;
                    let options = glsl::Options {
                        stage,
                        defines: Default::default(),
                    };

                    let mut parser = glsl::Parser::default();

                    parser.parse(&options, source.unwrap_text())?
                }
                _ => unimplemented!("parse target not implemented"),
            }
        });

        Ok(shader.module.as_ref().expect("no module after parsing"))
    }

    #[allow(unreachable_patterns)]
    pub fn generate<'a>(
        self,
        shader: &Shader,
        result: &mut ShaderCode,
        target: Option<&EntryPoint>,
    ) -> Result<(), TranspileError<'a>> {
        match self {
            #[cfg(feature = "spv-out")]
            ShaderLanguage::SPV => {
                use byteorder::{WriteBytesExt, LE};
                use naga::back::spv;

                let target = target.ok_or(TranspileError::NoEntryPoint)?;

                let options = spv::Options::default();
                let mut writer = spv::Writer::new(&options)?;

                let pipeline_options = spv::PipelineOptions {
                    shader_stage: target.stage,
                    entry_point: target
                        .function
                        .name
                        .clone()
                        .ok_or(TranspileError::NoEntryPoint)?,
                };

                let mut words: Vec<u32> = vec![];
                writer.write(
                    shader.module.as_ref().expect("no module"),
                    shader.module_info.as_ref().expect("no module info"),
                    Some(&pipeline_options),
                    &mut words,
                )?;
                for w in words {
                    result.write_u32::<LE>(w)?;
                }
            }
            #[cfg(feature = "glsl-out")]
            ShaderLanguage::GLSL => {
                use naga::back::glsl;

                let target = target.ok_or(TranspileError::NoEntryPoint)?;

                let options = glsl::Options::default();
                let pipeline_options = glsl::PipelineOptions {
                    shader_stage: target.stage,
                    entry_point: target
                        .function
                        .name
                        .clone()
                        .ok_or(TranspileError::NoEntryPoint)?,
                };

                let mut writer = glsl::Writer::new(
                    result,
                    shader.module.as_ref().expect("no module"),
                    shader.module_info.as_ref().expect("no module info"),
                    &options,
                    &pipeline_options,
                )?;
                writer.write()?;
            }
            #[cfg(feature = "wgsl-out")]
            ShaderLanguage::WGSL => {
                use naga::back::wgsl;

                let mut writer = wgsl::Writer::new(result, wgsl::WriterFlags::empty());
                writer.write(
                    shader.module.as_ref().expect("no module"),
                    shader.module_info.as_ref().expect("no module info"),
                )?;
            }
            #[cfg(feature = "hlsl-out")]
            ShaderLanguage::HLSL => {
                use naga::back::hlsl;

                let mut writer = hlsl::Writer::new(result, &hlsl::Options::default());
                writer.write(
                    shader.module.as_ref().expect("no module"),
                    shader.module_info.as_ref().expect("no module info"),
                )?;
            }
            #[cfg(feature = "msl-out")]
            ShaderLanguage::MSL => {
                use naga::back::msl;

                let mut writer = msl::Writer::new(result);
                writer.write(
                    shader.module.as_ref().expect("no module"),
                    shader.module_info.as_ref().expect("no module info"),
                    &msl::Options::default(),
                    &msl::PipelineOptions::default(),
                )?;
            }
            _ => return Err(TranspileError::TargetNotSupported),
        }
        Ok(())
    }
}

impl Display for ShaderLanguage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_uppercase_str())
    }
}

impl FromStr for ShaderLanguage {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value.to_ascii_lowercase().as_str() {
            "wgsl" => ShaderLanguage::WGSL,
            "glsl" => ShaderLanguage::GLSL,
            "spv" => ShaderLanguage::SPV,
            "hlsl" => ShaderLanguage::HLSL,
            "msl" => ShaderLanguage::MSL,
            _ => return Err(()),
        })
    }
}

#[derive(Debug)]
pub struct ShaderFile {
    pub language: ShaderLanguage,
    pub path: PathBuf,
    pub stage: Option<ShaderStage>,
}

impl ShaderFile {
    pub fn name(&self) -> String {
        let mut result = file_prefix(&self.path)
            .and_then(|os_str| os_str.to_str())
            .expect("invalid shader file name")
            .to_ascii_uppercase()
            .replace('.', "_");

        if let Some(stage) = self.stage {
            result.push_str(match stage {
                ShaderStage::Vertex => "_VERT",
                ShaderStage::Fragment => "_FRAG",
                ShaderStage::Compute => "_COMP",
            });
        }

        result
    }
}

impl PartialEq for ShaderFile {
    fn eq(&self, other: &Self) -> bool {
        self.path.eq(&other.path)
    }
}
impl Eq for ShaderFile {}
impl PartialOrd for ShaderFile {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.path.partial_cmp(&other.path)
    }
}
impl Ord for ShaderFile {
    fn cmp(&self, other: &Self) -> Ordering {
        self.path.cmp(&other.path)
    }
}

pub trait Transpile {
    fn transpile_and_write<'a>(
        &self,
        config: &'a Config,
    ) -> Result<CodegenData, TranspileError<'a>>;
}

impl Transpile for Shader {
    fn transpile_and_write<'a>(
        &self,
        config: &'a Config,
    ) -> Result<CodegenData, TranspileError<'a>> {
        let module = self.module.as_ref().expect("shader module must exist");

        let mut result = CodegenData::default();

        log::info!("Transpiling: {:?}", &self.path);
        let source_lang = ShaderLanguage::from_file_name(&self.path)
            .ok_or(TranspileError::SourceNotSupported)?;
        log::info!("Detected language: {}", source_lang);

        result.register_source(
            source_lang,
            ShaderFile {
                language: ShaderLanguage::from_file_name(&self.path).unwrap(),
                path: self.path.to_path_buf(),
                stage: None,
            },
        );

        for &target in &config.targets {
            let target_dir = &config.out.join(target.to_str());

            if !target_dir.exists() {
                std::fs::create_dir(&target_dir)?;
            }

            if module.entry_points.len() > 1 {
                match target {
                    ShaderLanguage::WGSL | ShaderLanguage::SPV => {
                        log::info!("Generating {} module...", target.to_uppercase_str());
                        let entry_point = &module.entry_points[0];

                        let transpiled =
                            transpile_entry(self, Some(entry_point), target)?;
                        std::fs::write(
                            &config
                                .out
                                .join(target.to_str())
                                .join(self.path.with_extension(target.get_ext(None))),
                            transpiled,
                        )?;
                        result.register_result(
                            target,
                            ShaderFile {
                                language: target,
                                stage: None,
                                path: config
                                    .out_relative()
                                    .join(target.to_str())
                                    .join(self.path.with_extension(target.get_ext(None))),
                            },
                        );
                    }
                    ShaderLanguage::GLSL | ShaderLanguage::HLSL | ShaderLanguage::MSL => {
                        log::info!("Generating {} files...", target.to_uppercase_str());
                        for entry_point in &module.entry_points {
                            log::info!(
                                "- {} {} shader entry point: {}",
                                target.to_uppercase_str(),
                                entry_point.stage.name(),
                                match &entry_point.function.name {
                                    Some(s) => s.as_str(),
                                    None => "<no_function>",
                                }
                            );
                            let transpiled =
                                transpile_entry(self, Some(entry_point), target)?;

                            std::fs::write(
                                &target_dir.join(self.path.with_extension(
                                    target.get_ext(Some(entry_point.stage)),
                                )),
                                transpiled,
                            )?;

                            result.register_result(
                                target,
                                ShaderFile {
                                    language: target,
                                    stage: Some(entry_point.stage),
                                    path: config
                                        .out_relative()
                                        .join(target.to_str())
                                        .join(self.path.with_extension(
                                            target.get_ext(Some(entry_point.stage)),
                                        )),
                                },
                            );
                        }
                    }
                }
            } else if !module.entry_points.is_empty() {
                let entry_point = &module.entry_points[0];
                let transpiled = transpile_entry(self, Some(entry_point), target)?;
                std::fs::write(
                    &target_dir.join(
                        self.path
                            .with_extension(target.get_ext(Some(entry_point.stage))),
                    ),
                    transpiled,
                )?;
                result.register_result(
                    target,
                    ShaderFile {
                        language: target,
                        stage: Some(entry_point.stage),
                        path: config.out_relative().join(target.to_str()).join(
                            self.path
                                .with_extension(target.get_ext(Some(entry_point.stage))),
                        ),
                    },
                );
            } else {
                log::info!(
                    "Skipping shader source with no entry points: {}",
                    self.path.display()
                );
                continue;
            }
        }

        Ok(result)
    }
}

fn transpile_entry<'a>(
    shader: &Shader,
    entry_point: Option<&EntryPoint>,
    target: ShaderLanguage,
) -> Result<ShaderCode, TranspileError<'a>> {
    let mut transpiled = if target.is_binary() {
        ShaderCode::Binary(Vec::with_capacity(512))
    } else {
        ShaderCode::Text(String::with_capacity(1024))
    };

    target.generate(shader, &mut transpiled, entry_point)?;

    Ok(transpiled)
}

impl Transpile for Vec<Shader> {
    fn transpile_and_write<'a>(
        &self,
        config: &'a Config,
    ) -> Result<CodegenData, TranspileError<'a>> {
        let mut result = CodegenData::default();

        // Remove previously generated files
        if config.out.exists() {
            log::info!("Removing old generated files...");
            std::fs::remove_dir_all(&config.out)?;
            std::fs::create_dir_all(&config.out)?;
        }
        for shader in self {
            match shader.transpile_and_write(config) {
                Ok(data) => result += data,
                Err(err) => {
                    log::error!(
                        "Encountered errors while transpiling: {}\n{}",
                        shader.path.display(),
                        err
                    );
                    return Err(err);
                }
            };
        }

        Ok(result)
    }
}
