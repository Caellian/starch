use std::error::Error;
use std::path::{Path, PathBuf};

pub trait PathExt {
    fn long_ext(&self) -> Option<&str>;
}

impl<T: AsRef<Path>> PathExt for T {
    fn long_ext(&self) -> Option<&str> {
        let start = self.as_ref().file_stem()?.len() + 1;
        Some(&self.as_ref().to_str()?[start..])
    }
}

pub fn collect_files<F: Fn(&Path) -> bool>(
    root: impl AsRef<Path>,
    filter: F,
) -> Vec<PathBuf> {
    let root = match root.as_ref().canonicalize() {
        Ok(root) => root,
        Err(_) => {
            log::error!("unable to collect path: {}", root.as_ref().display());
            panic!("unable to canonicalize");
        }
    };

    collect_files_impl(&root, &root, &filter)
}

fn collect_files_impl<F: Fn(&Path) -> bool>(
    root: impl AsRef<Path>,
    path: impl AsRef<Path>,
    filter: &F,
) -> Vec<PathBuf> {
    let mut result = vec![];

    let read_dir = std::fs::read_dir(path.as_ref()).unwrap();
    for entry_result in read_dir {
        if let Ok(dir_entry) = entry_result {
            let sub_path = dir_entry.path();

            if sub_path.is_dir() {
                result.append(&mut collect_files_impl(root.as_ref(), &sub_path, filter));
                continue;
            } else if !sub_path.is_file() || !(&filter)(&sub_path) {
                continue;
            }

            let rel_path = sub_path
                .canonicalize()
                .expect("unable to canonicalize path")
                .strip_prefix(root.as_ref())
                .expect("unable to strip prefix")
                .to_path_buf();

            result.push(rel_path);
        } else {
            panic!("unable to read shader dir entry");
        }
    }

    result
}

pub(crate) trait LogResult<T> {
    fn ok_or_log(self) -> Option<T>;
}

impl<T, E: Error> LogResult<T> for Result<T, E> {
    fn ok_or_log(self) -> Option<T> {
        match self {
            Ok(value) => Some(value),
            Err(err) => {
                log::error!("{}", err);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collection_works() {
        let test_path = PathBuf::from("./src/");

        let test = collect_files(&test_path, |path| {
            path.extension().map(|os_str| os_str.to_str()).flatten() == Some("rs")
        });

        assert!(test.len() > 0)
    }
}
