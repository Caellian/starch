use crate::config::Config;
use crate::transpile::{Language, ResultInFile, ResultOutFile, TranspileStatus};
use naga::ShaderStage;
use std::io::Error;
use std::path::Path;

fn static_statement(name: impl AsRef<str>, value: impl AsRef<Path>, indent: usize) -> String {
    format!(
        "{}pub static {}: &'static str = include_str!(\"{}\");\n",
        "    ".repeat(indent),
        name.as_ref(),
        value.as_ref().display(),
    )
}

pub trait ToStaticStatement {
    fn to_static_statement(&self, indent: usize) -> String;
}

impl ToStaticStatement for ResultInFile {
    fn to_static_statement(&self, indent: usize) -> String {
        let shader_name = self
            .path
            .file_stem()
            .map(|os_str| os_str.to_str())
            .flatten()
            .expect("invalid shader file name");

        let name = shader_name.to_ascii_uppercase().replace(".", "_");

        static_statement(name, &self.path, indent)
    }
}

pub(crate) fn stage_name(stage: ShaderStage) -> &'static str {
    match stage {
        ShaderStage::Vertex => "VERT",
        ShaderStage::Fragment => "FRAG",
        ShaderStage::Compute => "COMP",
    }
}

impl ToStaticStatement for ResultOutFile {
    fn to_static_statement(&self, indent: usize) -> String {
        let shader_name = self
            .path
            .file_stem()
            .map(|os_str| os_str.to_str())
            .flatten()
            .expect("invalid shader file name");

        let mut name = shader_name.to_ascii_uppercase().replace(".", "_");
        match self.language {
            Language::GLSL => {
                name.push('_');
                name.push_str(stage_name(self.stage))
            }
            _ => {}
        }

        static_statement(name, &self.path, indent)
    }
}

fn gen_statements(input_files: &Vec<ResultOutFile>, indent: usize) -> String {
    let mut result = Vec::with_capacity(input_files.len());

    result.append(
        &mut input_files
            .into_iter()
            .map(|shader| shader.to_static_statement(indent))
            .collect(),
    );

    result.concat()
}

pub trait GenerateSources {
    fn generate_sources(self, config: &Config) -> Result<(), std::io::Error>;
}

impl GenerateSources for TranspileStatus {
    fn generate_sources(self, config: &Config) -> Result<(), Error> {
        let TranspileStatus { inputs, generated } = self;

        if generated.is_empty() {
            return Ok(());
        }

        let mut result = inputs
            .iter()
            .map(|result| result.to_static_statement(0))
            .fold(String::new(), |acc, it| acc + it.as_str());

        for (lang, output) in generated.iter() {
            if output.is_empty() {
                continue;
            }

            result.push_str("\npub mod ");
            result.push_str(lang.to_str());
            result.push_str(" {\n");
            result.push_str(&gen_statements(&output, 1));
            result.push_str("}\n");
        }

        if config.generated.exists() {
            std::fs::remove_file(&config.generated)?;
        }

        std::fs::write(&config.generated, result)
    }
}
