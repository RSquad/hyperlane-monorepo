use std::env;
use std::path::Path;

pub fn resolve_abs_path<P: AsRef<Path>>(rel_path: P) -> String {
    let mut configs_path = env::current_dir().unwrap();
    configs_path.push(rel_path);
    configs_path
        .canonicalize()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned()
}
