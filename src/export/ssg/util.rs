use std::path::{Path, PathBuf};

fn slugify_core(s: &str) -> String {
    s.to_ascii_lowercase()
        .split(&['-', '_', ' '])
        .filter(|sub| !sub.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

pub fn slugify_to_string<P: AsRef<Path>>(input: P) -> String {
    let mut segments = Vec::new();
    for component in input.as_ref().components() {
        let s = component.as_os_str().to_string_lossy();
        let slugged = slugify_core(&s);
        if !slugged.is_empty() {
            segments.push(slugged);
        }
    }
    segments.join("/") // 确保在 Windows 上也返回合法 URL 路径
}

pub fn slugify_to_path<P: AsRef<Path>>(input: P) -> PathBuf {
    let mut result = PathBuf::new();
    for component in input.as_ref().components() {
        let s = component.as_os_str().to_string_lossy();
        result.push(slugify_core(&s));
    }
    result
}
