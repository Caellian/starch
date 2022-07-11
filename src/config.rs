use crate::prelude_build::ShaderLanguage;
use naga::valid::{Capabilities, ValidationFlags, Validator};
#[cfg(feature = "config-file")]
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
#[cfg(feature = "config-file")]
use std::fs::File;
#[cfg(feature = "config-file")]
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "config-file", derive(Serialize, Deserialize))]
pub struct Config {
    pub src: PathBuf,
    pub out: PathBuf,
    pub generated: PathBuf,

    pub targets: Vec<ShaderLanguage>,
    pub validation_flags: ValidationFlags,
    pub capabilities: Capabilities,
}

fn env_var_list<K: AsRef<OsStr>>(key: K) -> Option<Vec<String>> {
    std::env::var(key)
        .ok()
        .map(|it| it.split(',').map(|s| s.to_string()).collect())
}

macro_rules! path_field {
    ($field: ident, $source: ident, $env_var: literal, $root: ident, $default: literal) => {
        let $field = std::env::var($env_var)
            .ok()
            .map(|value| PathBuf::from(value))
            .or($source.as_ref().map(|c: &Config| c.$field.clone()))
            .unwrap_or($root.as_ref().join("src").join($default));
    };
}

impl Default for Config {
    fn default() -> Self {
        Config::init(".")
    }
}

impl Config {
    #[allow(clippy::let_and_return)]
    pub fn init(root: impl AsRef<Path>) -> Config {
        #[cfg(feature = "config-file")]
        let local: Option<Config> =
            Config::load_from_file(&root.as_ref().join("starch.yml"));
        #[cfg(not(feature = "config-file"))]
        let local: Option<Config> = None;

        path_field!(src, local, "STARCH_SHADER_SRC", root, "");
        path_field!(out, local, "STARCH_SHADER_OUT", root, "gen");
        path_field!(generated, local, "STARCH_SHADER_GEN", root, "lib.rs");

        let targets = env_var_list("STARCH_SHADER_TARGETS")
            .map(|env| {
                env.into_iter()
                    .filter_map(|text| ShaderLanguage::from_str(&text).ok())
                    .collect()
            })
            .or_else(|| local.as_ref().map(|l| l.targets.clone()))
            .unwrap_or_else(|| {
                vec![
                    #[cfg(feature = "spv-out")]
                    ShaderLanguage::SPV,
                    #[cfg(feature = "glsl-out")]
                    ShaderLanguage::GLSL,
                    #[cfg(feature = "hlsl-out")]
                    ShaderLanguage::HLSL,
                    #[cfg(feature = "wgsl-out")]
                    ShaderLanguage::WGSL,
                    #[cfg(feature = "msl-out")]
                    ShaderLanguage::MSL,
                ]
            });

        let validation_flags = std::env::var("STARCH_SHADER_VALIDATION")
            .ok()
            .and_then(|env| u8::from_str(&env).ok().and_then(ValidationFlags::from_bits))
            .or_else(|| local.as_ref().map(|l| l.validation_flags))
            .unwrap_or(ValidationFlags::all());

        let capabilities = std::env::var("STARCH_SHADER_CAPABILITIES")
            .ok()
            .and_then(|env| u8::from_str(&env).ok().and_then(Capabilities::from_bits))
            .or_else(|| local.as_ref().map(|l| l.capabilities))
            .unwrap_or(Capabilities::all());

        let result = Config {
            src,
            out,
            generated,
            targets,
            validation_flags,
            capabilities,
        };

        #[cfg(feature = "config-file")]
        {
            if local.is_none() {
                result.write_to_file(&root.as_ref().join("starch.yml"));
            }
        }

        result
    }

    #[cfg(feature = "config-file")]
    pub fn load_from_file(path: impl AsRef<Path>) -> Option<Config> {
        let path = path.as_ref();

        if !path.exists() || !path.is_file() {
            return None;
        }

        let file = File::open(path).ok()?;
        let reader = BufReader::new(file);
        Some(serde_yaml::from_reader(reader).ok()?)
    }

    #[cfg(feature = "config-file")]
    pub fn write_to_file(&self, path: impl AsRef<Path>) -> Result<(), std::io::Error> {
        let path = path.as_ref();

        if path.exists() {
            std::fs::remove_file(path)?;
        } else if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?
            }
        }

        let file = File::create(path)?;
        let writer = BufWriter::new(file);

        serde_yaml::to_writer(writer, self)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))
    }

    pub fn validator(&self) -> Validator {
        Validator::new(self.validation_flags, self.capabilities)
    }

    pub fn out_relative(&self) -> &Path {
        self.out.strip_prefix(&self.src).unwrap()
    }
}
