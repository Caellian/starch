pub mod config;
pub mod error;
pub mod language;
pub mod preprocess;
pub mod shader;
pub(crate) mod util;

pub mod prelude {
    pub use super::config::Config as StarchConfig;
    pub use super::language::codegen::{CodegenData, GenerateSources};
    pub use super::language::transpile::*;
    pub use super::preprocess::preprocess_shader;
    pub use super::shader::*;
}

#[cfg(test)]
mod tests {
    use super::prelude::*;
    use log::LevelFilter;

    #[test]
    fn full_test() {
        env_logger::builder()
            .filter_level(LevelFilter::Trace)
            .init();

        let config = StarchConfig::init("../test");

        let shaders = Shader::load_shaders(&config);
        let result: CodegenData = shaders.transpile(&config).unwrap();
        result.generate_sources(&config);
    }
}
