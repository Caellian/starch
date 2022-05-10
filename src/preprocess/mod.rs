pub mod matcher;

use crate::config::Config;
use crate::shader::{Shader, ShaderCode};
use regex::Regex;
use std::path::PathBuf;

lazy_static::lazy_static! {
    pub static ref INCLUDE_MACRO: Regex = {
        let path_str = r"((\.|\.\.|[\w\d\-_\.]+)((\\|/)(\.\.|[\w\d\-_\.]+))*)";
        let expected = format!("use\\s+('({0})'|\"({0})\")", path_str);
        Regex::new(&expected).unwrap()
    };
}

fn proc_includes(buffer: &mut String, _config: &Config) {
    let mut includes: Vec<(usize, usize)> = vec![];

    while INCLUDE_MACRO.find(buffer).is_some() {
        for captures in INCLUDE_MACRO.captures_iter(buffer) {
            let whole = captures.get(0).unwrap();
            let path = PathBuf::from(captures.get(1).unwrap().as_str());
            log::debug!("found include path: {}", path.display());

            includes.push((whole.start(), whole.end()));
        }
    }
}

pub fn preprocess_shader<'a>(
    shader: &'a mut Shader,
    config: &'a Config,
) -> Option<&'a ShaderCode> {
    let full_path = config.src.join(&shader.path);
    let mut result = ShaderCode::read(&full_path, shader.lang.is_binary()).ok()?;

    match &mut result {
        ShaderCode::Text(value) => {
            proc_includes(value, config);
        }
        ShaderCode::Binary(_) => {}
    }

    shader.source = Some(result);
    shader.source.as_ref()
}
