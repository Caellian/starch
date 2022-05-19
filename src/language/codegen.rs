use crate::config::Config;
use crate::prelude::{ShaderFile, ShaderLanguage};
use std::collections::BTreeSet;
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

#[derive(Debug, Default)]
pub struct CodegenData {
    pub sources: [BTreeSet<ShaderFile>; ShaderLanguage::COUNT],
    pub includes: [BTreeSet<ShaderFile>; ShaderLanguage::COUNT],
}

impl CodegenData {
    pub fn register_source(&mut self, language: ShaderLanguage, result_file: ShaderFile) {
        self.sources[language as usize].insert(result_file);
    }

    pub fn register_result(&mut self, language: ShaderLanguage, result_file: ShaderFile) {
        self.includes[language as usize].insert(result_file);
    }

    pub fn generate_sources(self, config: &Config) -> Result<(), Error> {
        let mut c = Context::default();

        let mut result = String::from("// GENERATED SOURCE FILE. DO NOT EDIT.\n");

        for lang in ShaderLanguage::ALL {
            let includes: BTreeSet<&ShaderFile> = self.sources[lang as usize]
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
                    .write_str(&format_static_statement(
                        include.name(),
                        &include.path,
                        c.indent,
                    ))
                    .expect("can't write static statements");
            }

            c.indent -= 1;
            result.write_str("}\n").expect("can't close module");
        }

        std::fs::write(&config.generated, result)
    }
}

impl AddAssign for CodegenData {
    fn add_assign(&mut self, mut rhs: Self) {
        for lang in ShaderLanguage::ALL {
            self.sources[lang as usize].append(&mut rhs.sources[lang as usize]);
            self.includes[lang as usize].append(&mut rhs.includes[lang as usize]);
        }
    }
}
