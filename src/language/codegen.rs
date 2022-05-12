use crate::config::Config;
use crate::prelude::{ShaderFile, ShaderLanguage};
use crate::util::file_prefix;
use naga::ShaderStage;
use std::collections::HashSet;
use std::fmt::{Debug, Write};
use std::io::Error;
use std::ops::AddAssign;
use std::path::Path;

#[derive(Debug, Default)]
pub struct Context {
    indent: usize,
}

fn format_static_statement(
    name: impl AsRef<str>,
    value: impl AsRef<Path>,
    indent: usize,
) -> String {
    format!(
        "{}pub static {}: &'static str = include_str!(\"./{}\");\n",
        "    ".repeat(indent),
        name.as_ref(),
        value.as_ref().display(),
    )
}

pub trait ToStaticStatement {
    fn to_static_statement(&self, c: &mut Context) -> String;
}

impl ToStaticStatement for ShaderFile {
    fn to_static_statement(&self, c: &mut Context) -> String {
        let path = self.path.as_path();

        let shader_name = file_prefix(path)
            .map(|os_str| os_str.to_str())
            .flatten()
            .expect("invalid shader file name");

        let name = shader_name.to_ascii_uppercase().replace(".", "_");

        format_static_statement(name, &self.path, c.indent)
    }
}

pub trait GenerateSources {
    fn generate_sources(self, config: &Config) -> Result<(), std::io::Error>;
}

#[derive(Debug, Default)]
pub struct CodegenData {
    pub sources: [HashSet<ShaderFile>; ShaderLanguage::COUNT],
    pub includes: [HashSet<ShaderFile>; ShaderLanguage::COUNT],
}

impl CodegenData {
    pub fn register_source(&mut self, language: ShaderLanguage, result_file: ShaderFile) {
        self.sources[language as usize].insert(result_file);
    }

    pub fn register_result(&mut self, language: ShaderLanguage, result_file: ShaderFile) {
        self.includes[language as usize].insert(result_file);
    }
}

impl AddAssign for CodegenData {
    fn add_assign(&mut self, mut rhs: Self) {
        for lang in ShaderLanguage::ALL {
            for shader in rhs.sources[lang as usize].drain() {
                self.sources[lang as usize].insert(shader);
            }
            for shader in rhs.includes[lang as usize].drain() {
                self.includes[lang as usize].insert(shader);
            }
        }
    }
}

impl GenerateSources for CodegenData {
    fn generate_sources(self, config: &Config) -> Result<(), Error> {
        let mut c = Context::default();

        let mut result = String::from("// GENERATED SOURCE FILE. DO NOT EDIT.\n");

        for lang in ShaderLanguage::ALL {
            let includes: HashSet<&ShaderFile> = self.sources[lang as usize]
                .union(&self.includes[lang as usize])
                .collect();

            if includes.is_empty() {
                continue;
            }

            result
                .write_fmt(format_args!("\npub mod {} {{\n", lang.to_str()))
                .expect("can't write module header");
            c.indent += 1;

            for include in includes {
                result
                    .write_str(&include.to_static_statement(&mut c))
                    .expect("can't write static statements");
            }

            c.indent -= 1;
            result.write_str("}\n").expect("can't close module");
        }

        std::fs::write(&config.generated, result)
    }
}
