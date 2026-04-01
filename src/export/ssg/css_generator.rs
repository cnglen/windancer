use std::path::Path;

use encre_css::Config;
use walkdir::WalkDir;

pub fn generate<P: AsRef<Path>>(f_output: P) -> std::io::Result<String> {
    let mut config = Config::default();
    encre_css_typography::register(&mut config);
    // println!("config={:?}", config);

    let mut sources = Vec::new();
    let directory = f_output
        .as_ref()
        .parent()
        .expect("should have parent directory");
    for entry in WalkDir::new(directory).into_iter().filter_map(|e| e.ok()) {
        if entry.metadata().unwrap().is_file() {
            let from = entry.path();
            let from_filename = from.file_name().expect("xx").to_string_lossy().to_string();
            if from.is_file()
                && from.extension() == Some(std::ffi::OsStr::new("html"))
                && !from_filename.starts_with(&['.', '#'])
            {
                tracing::debug!("  tailwincss from content: {:?}", from);
                let content = std::fs::read_to_string(from).expect("read to string");
                sources.push(content);
            }
        }
    }

    let css = encre_css::generate(sources.iter().map(|s| s.as_str()), &config);
    let css = format!(
        r##"@layer encre_base {{
{}
}}"##,
        css
    );
    let _ = std::fs::write(f_output, css);

    Ok(String::from(""))
}
