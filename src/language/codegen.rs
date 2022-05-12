use crate::config::Config;
use crate::prelude::{ResultFile, ShaderLanguage};
use naga::ShaderStage;
use std::collections::HashMap;
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

impl ToStaticStatement for ResultFile {
    fn to_static_statement(&self, c: &mut Context) -> String {
        let path = self.path.as_path();

        let shader_name = path
            .file_stem()
            .map(|os_str| os_str.to_str())
            .flatten()
            .expect("invalid shader file name");

        let mut name = shader_name.to_ascii_uppercase().replace(".", "_");
        if let Some(stage) = self.stage {
            name.push('_');
            name.push_str(stage_name(stage))
        }

        format_static_statement(name, &self.path, c.indent)
    }
}

pub trait GenerateSources {
    fn generate_sources(self, config: &Config) -> Result<(), std::io::Error>;
}

#[derive(Debug, Default)]
pub struct CodegenData {
    pub includes: HashMap<ShaderLanguage, Vec<ResultFile>>,
}

impl CodegenData {
    pub fn register_result(&mut self, language: ShaderLanguage, result_file: ResultFile) {
        match self.includes.get_mut(&language) {
            Some(it) => (*it).push(result_file),
            None => {
                self.includes.insert(language, vec![result_file]);
            }
        };
    }
}

impl AddAssign for CodegenData {
    fn add_assign(&mut self, rhs: Self) {
        for (k, mut v) in rhs.includes {
            if let Some(it) = self.includes.get_mut(&k) {
                it.append(&mut v);
            } else {
                self.includes.insert(k, v);
            }
        }
    }
}

impl GenerateSources for CodegenData {
    fn generate_sources(self, config: &Config) -> Result<(), Error> {
        let mut c = Context::default();

        let mut result = String::from("// GENERATED SOURCE FILE. DO NOT EDIT.\n");
        for (lang, includes) in self.includes {
            result
                .write_fmt(format_args!("\npub mod {} {{\n", lang.to_str()))
                .expect("can't write mod header");
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

pub(crate) fn stage_name(stage: ShaderStage) -> &'static str {
    match stage {
        ShaderStage::Vertex => "VERT",
        ShaderStage::Fragment => "FRAG",
        ShaderStage::Compute => "COMP",
    }
}
