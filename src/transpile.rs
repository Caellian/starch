use crate::config::Config;
use crate::error::TranspileError;
use crate::shader::{Shader, ShaderCode};
#[allow(unused_imports)]
use crate::util::LogResult;
use naga::{EntryPoint, Module, ShaderStage};
#[cfg(feature = "config-file")]
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::AddAssign;
use std::path::{Path, PathBuf};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
#[non_exhaustive]
#[cfg_attr(feature = "config-file", derive(Serialize, Deserialize))]
pub enum Language {
    WGSL,
    GLSL,
    SPV,
    HLSL,
}

impl Language {
    #[warn(unreachable_code)]
    pub fn from_file_name(path: impl AsRef<Path>) -> Option<Language> {
        let ext = path
            .as_ref()
            .extension()
            .map(|os_str| os_str.to_str())
            .flatten()?;

        Some(match ext.to_ascii_lowercase().as_str() {
            #[cfg(feature = "wgsl-in")]
            "wgsl" => Language::WGSL,
            #[cfg(feature = "glsl-in")]
            "glsl" | "vs" | "fs" | "cs" | "vert" | "frag" | "comp" => Language::GLSL,
            #[cfg(feature = "spv-in")]
            "spv" => Language::SPV,
            _ => return None,
        })
    }

    pub fn from_str(value: &str) -> Option<Language> {
        Some(match value.to_ascii_lowercase().as_str() {
            "wgsl" => Language::WGSL,
            "glsl" => Language::GLSL,
            "spv" => Language::SPV,
            "hlsl" => Language::HLSL,
            _ => return None,
        })
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            Language::WGSL => "wgsl",
            Language::GLSL => "glsl",
            Language::SPV => "spv",
            Language::HLSL => "hlsl",
        }
    }

    pub fn to_uppercase_str(&self) -> &'static str {
        match self {
            Language::WGSL => "WGSL",
            Language::GLSL => "GLSL",
            Language::SPV => "SPV",
            Language::HLSL => "HLSL",
        }
    }

    pub fn is_binary(&self) -> bool {
        match self {
            Language::SPV => true,
            _ => false,
        }
    }

    pub fn parse<'a>(&self, shader: &'a mut Shader) -> Option<&'a Module> {
        if shader.module.is_some() {
            return shader.module.as_ref();
        }

        let module = {
            let source = shader.source.as_ref()?;

            match self {
                #[cfg(feature = "spv-in")]
                Language::SPV => {
                    use naga::front::spv;

                    let options = spv::Options::default();
                    spv::parse_u8_slice(source.unwrap_binary(), &options).ok_or_log()
                }
                #[cfg(feature = "wgsl-in")]
                Language::WGSL => naga::front::wgsl::parse_str(source.unwrap_text()).ok_or_log(),
                #[cfg(feature = "glsl-in")]
                Language::GLSL => {
                    use naga::front::glsl;

                    let options = if let Some(stage) = shader.source_stage {
                        glsl::Options {
                            stage,
                            defines: Default::default(),
                        }
                    } else {
                        log::error!("unknown GLSL shader stage");
                        return None;
                    };

                    let mut parser = glsl::Parser::default();
                    match parser.parse(&options, source.unwrap_text()) {
                        Ok(value) => Some(value),
                        Err(errors) => {
                            for error in errors {
                                log::error!("{}", error);
                            }
                            None
                        }
                    }
                }
                _ => unimplemented!("transpilation target not implemented"),
            }
        };

        shader.module = module;
        shader.module.as_ref()
    }

    pub fn generate(
        &self,
        shader: &Shader,
        result: &mut ShaderCode,
        target: &EntryPoint,
    ) -> Result<(), TranspileError> {
        Ok(match self {
            #[cfg(feature = "spv-out")]
            Language::SPV => {
                use byteorder::{WriteBytesExt, LE};
                use naga::back::spv;

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
            Language::GLSL => {
                use naga::back::glsl;

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
            Language::WGSL => {
                use naga::back::wgsl;
                let mut writer = wgsl::Writer::new(result, wgsl::WriterFlags::empty());
                writer.write(
                    shader.module.as_ref().expect("no module"),
                    shader.module_info.as_ref().expect("no module info"),
                )?;
            }
            #[cfg(feature = "hlsl-out")]
            Language::HLSL => {
                use naga::back::hlsl;

                let mut writer = hlsl::Writer::new(result, &hlsl::Options::default());
                writer.write(
                    shader.module.as_ref().expect("no module"),
                    shader.module_info.as_ref().expect("no module info"),
                )?;
            }
            _ => Err(TranspileError::TargetNotSupported)?,
        })
    }
}

#[derive(Debug)]
pub struct ResultInFile {
    pub language: Language,
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct ResultOutFile {
    pub language: Language,
    pub stage: ShaderStage,
    pub path: PathBuf,
}

#[derive(Debug, Default)]
pub struct TranspileStatus {
    pub inputs: Vec<ResultInFile>,
    pub generated: HashMap<Language, Vec<ResultOutFile>>,
}

impl AddAssign for TranspileStatus {
    fn add_assign(&mut self, rhs: Self) {
        let mut rhs = rhs;
        self.inputs.append(&mut rhs.inputs);
        for (k, mut v) in rhs.generated {
            if let Some(it) = self.generated.get_mut(&k) {
                it.append(&mut v);
            } else {
                self.generated.insert(k, v);
            }
        }
    }
}

pub trait Transpile {
    fn transpile<'a>(&self, config: &'a Config) -> Result<TranspileStatus, TranspileError<'a>>;
}

impl Transpile for Shader {
    fn transpile<'a>(&self, config: &'a Config) -> Result<TranspileStatus, TranspileError<'a>> {
        let mut result = TranspileStatus::default();

        log::info!("Transpiling: {:?}", &self.path);
        result.inputs.push(ResultInFile {
            language: Language::from_file_name(&self.path).unwrap(),
            path: self.path.to_path_buf(),
        });

        for target in &config.targets {
            let out_dir = config.out.join(target.to_str());
            if !out_dir.exists() {
                std::fs::create_dir_all(&out_dir)?;
            }

            let module = self.module.as_ref().expect("shader module must exist");
            for entry_point in &module.entry_points {
                let mut transpiled = if target.is_binary() {
                    ShaderCode::Binary(Vec::with_capacity(512))
                } else {
                    ShaderCode::Text(String::with_capacity(1024))
                };

                let out_relative = {
                    let mut relative = self.path.clone();
                    relative.set_extension(stage_ext(entry_point.stage));
                    relative
                };

                let out_file = out_dir.join(&out_relative);
                if out_relative.parent().is_some() && !out_file.parent().unwrap().exists() {
                    std::fs::create_dir_all(out_file.parent().unwrap())?;
                }

                target.generate(self, &mut transpiled, entry_point)?;
                std::fs::write(&out_file, transpiled)?;

                let relative_path = config
                    .out_relative()
                    .join(target.to_str())
                    .join(out_relative);
                match result.generated.get_mut(&target) {
                    Some(gen) => (*gen).push(ResultOutFile {
                        language: *target,
                        stage: entry_point.stage,
                        path: relative_path,
                    }),
                    None => {
                        result.generated.insert(
                            target.clone(),
                            vec![ResultOutFile {
                                language: *target,
                                stage: entry_point.stage,
                                path: relative_path,
                            }],
                        );
                    }
                };
            }
        }

        Ok(result)
    }
}

impl Transpile for Vec<Shader> {
    fn transpile<'a>(&self, config: &'a Config) -> Result<TranspileStatus, TranspileError<'a>> {
        let mut result = TranspileStatus::default();

        for shader in self {
            result += shader.transpile(config)?;
        }

        Ok(result)
    }
}

fn stage_ext(stage: ShaderStage) -> &'static str {
    match stage {
        ShaderStage::Vertex => "vert",
        ShaderStage::Fragment => "frag",
        ShaderStage::Compute => "comp",
    }
}
