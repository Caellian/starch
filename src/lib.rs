pub mod codegen;
pub mod config;
pub mod error;
pub mod preprocess;
pub mod shader;
pub mod transpile;
pub(crate) mod util;
pub mod wgsl;

pub mod prelude {
    pub use super::config::Config as StarchConfig;
    pub use super::preprocess::preprocess_shader;
    pub use super::shader::*;
    pub use super::transpile::*;
}

#[cfg(test)]
mod tests {
    use super::prelude::*;
    use crate::codegen::GenerateSources;
    use crate::config::Config;
    use log::LevelFilter;
    use std::path::Path;

    #[test]
    fn full_test() {
        env_logger::builder()
            .filter_level(LevelFilter::Trace)
            .init();

        let config = Config::init("./shaders");

        let shaders = Shader::load_shaders(&config);
        let result: TranspileStatus = shaders.transpile(&config).unwrap();
        result.generate_sources(&config);
    }
}
