use crate::config::Config;
use crate::preprocess;
use crate::transpile::Language;
use crate::util::{collect_files, PathExt};
use naga::valid::{ModuleInfo, Validator};
use naga::{Module, ShaderStage};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum ShaderCode {
    Text(String),
    Binary(Vec<u8>),
}

impl Default for ShaderCode {
    fn default() -> Self {
        ShaderCode::Binary(vec![])
    }
}

impl AsRef<[u8]> for ShaderCode {
    fn as_ref(&self) -> &[u8] {
        match self {
            ShaderCode::Text(text) => text.as_bytes(),
            ShaderCode::Binary(bin) => bin,
        }
    }
}

impl AsMut<[u8]> for ShaderCode {
    fn as_mut(&mut self) -> &mut [u8] {
        match self {
            ShaderCode::Text(text) => unsafe { text.as_bytes_mut() },
            ShaderCode::Binary(bin) => bin,
        }
    }
}

impl Write for ShaderCode {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.as_mut().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.as_mut().flush()
    }
}

impl std::fmt::Write for ShaderCode {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        Ok(match self {
            ShaderCode::Text(text) => {
                text.push_str(s);
            }
            ShaderCode::Binary(bin) => {
                bin.write_all(s.as_bytes()).map_err(|_| std::fmt::Error)?;
            }
        })
    }
}

impl ShaderCode {
    pub fn read(
        path: impl AsRef<Path>,
        binary: bool,
    ) -> Result<ShaderCode, std::io::Error> {
        Ok(if binary {
            ShaderCode::Binary(std::fs::read(path.as_ref())?)
        } else {
            ShaderCode::Text(std::fs::read_to_string(path.as_ref())?)
        })
    }

    pub fn get_text(&self) -> Option<&String> {
        match self {
            ShaderCode::Text(text) => Some(text),
            _ => None,
        }
    }

    pub fn get_text_mut(&mut self) -> Option<&mut String> {
        match self {
            ShaderCode::Text(text) => Some(text),
            _ => None,
        }
    }

    pub fn get_binary(&self) -> Option<&Vec<u8>> {
        match self {
            ShaderCode::Binary(value) => Some(value),
            _ => None,
        }
    }

    pub fn get_binary_mut(&mut self) -> Option<&mut Vec<u8>> {
        match self {
            ShaderCode::Binary(value) => Some(value),
            _ => None,
        }
    }

    pub fn unwrap_text(&self) -> &str {
        self.get_text().unwrap()
    }

    pub fn unwrap_text_mut(&mut self) -> &mut String {
        self.get_text_mut().unwrap()
    }

    pub fn unwrap_binary(&self) -> &Vec<u8> {
        self.get_binary().unwrap()
    }

    pub fn unwrap_binary_mut(&mut self) -> &mut Vec<u8> {
        self.get_binary_mut().unwrap()
    }
}

#[derive(Debug)]
pub struct Shader {
    pub path: PathBuf,
    pub lang: Language,
    pub source_stage: Option<ShaderStage>,
    pub source: Option<ShaderCode>,

    pub module: Option<Module>,
    pub module_info: Option<ModuleInfo>,
}

impl Shader {
    pub fn new(path: impl AsRef<Path>) -> Option<Shader> {
        Some(Shader {
            path: path.as_ref().to_path_buf(),
            lang: Language::from_file_name(path.as_ref())?,
            source_stage: stage_from_name(path.as_ref()),
            source: None,

            module: None,
            module_info: None,
        })
    }

    fn collect(config: &Config) -> Vec<Shader> {
        collect_files(&config.src, |name| Language::from_file_name(name).is_some())
            .into_iter()
            .filter_map(|path| Shader::new(path))
            .collect()
    }

    pub fn load_shaders(config: &Config) -> Vec<Shader> {
        Shader::collect(&config)
            .into_iter()
            .map(|mut shader| {
                preprocess::preprocess_shader(&mut shader, config);
                shader
            })
            .filter_map(|mut shader| {
                if shader.parse().is_some() {
                    Some(shader)
                } else {
                    log::warn!("Couldn't parse: {}", shader.path.display());
                    None
                }
            })
            .filter_map(|mut shader| {
                if shader.validate(config).is_some() {
                    Some(shader)
                } else {
                    log::warn!(
                        "{} didn't pass validation; not transpiling.",
                        shader.path.display()
                    );
                    None
                }
            })
            .collect()
    }

    pub fn read(&mut self) -> Option<&ShaderCode> {
        if self.source.is_some() {
            return self.source.as_ref();
        }

        let read_result = if self.lang.is_binary() {
            std::fs::read(&self.path).map(|value| ShaderCode::Binary(value))
        } else {
            std::fs::read_to_string(&self.path).map(|value| ShaderCode::Text(value))
        };

        match read_result {
            Ok(shader_source) => self.source = Some(shader_source),
            Err(err) => {
                log::warn!("Unable to read shader file: {}", self.path.display());
                log::error!("{}", err);
                return None;
            }
        }

        self.source.as_ref()
    }

    pub fn parse(&mut self) -> Option<&Module> {
        self.lang.clone().parse(self)
    }

    pub fn validate(&mut self, config: &Config) -> Option<&ModuleInfo> {
        let mut validator = Validator::new(config.validation_flags, config.capabilities);

        let result = validator.validate(self.module.as_ref()?);
        self.module_info = match result {
            Ok(info) => Some(info),
            Err(err) => {
                log::error!("{}", err);
                None
            }
        };

        self.module_info.as_ref()
    }
}

#[allow(unreachable_code)]
pub(crate) fn stage_from_name(path: impl AsRef<Path>) -> Option<ShaderStage> {
    let ext = path.as_ref().long_ext()?.to_ascii_lowercase();

    Some(match ext.as_str() {
        #[cfg(feature = "glsl-in")]
        "vs" | "vert" | "vs.glsl" => ShaderStage::Vertex,
        #[cfg(feature = "glsl-in")]
        "fs" | "frag" | "fs.glsl" => ShaderStage::Fragment,
        #[cfg(feature = "glsl-in")]
        "cs" | "comp" | "cs.glsl" => ShaderStage::Compute,
        _ => return None,
    })
}