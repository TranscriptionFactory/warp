use std::path::{Path, PathBuf};
const MARKDOWN_EXTENSIONS: &[&str] = &["md", "markdown"];
const MARKDOWN_FILE_NAMES: &[&str] = &["README", "CHANGELOG", "LICENSE"];
fn is_markdown_file(path: impl AsRef<Path>) -> bool {
    let path = path.as_ref();
    match path.extension() {
        Some(ext) => MARKDOWN_EXTENSIONS.iter().any(|m| ext.eq_ignore_ascii_case(m)),
        None => path.file_name().is_some_and(|fname| {
            MARKDOWN_FILE_NAMES.iter().any(|m| fname.eq_ignore_ascii_case(m))
        }),
    }
}
fn lang(p: &str) -> PathBuf { PathBuf::from(p) } // mirrors Remote language_path()
fn main() {
    let cases = [
        ("/home/user/notes/README.md", true),
        ("/home/user/doc.markdown", true),
        ("/srv/CHANGELOG", true),
        ("/home/user/src/main.rs", false),
        ("/home/user/data.json", false),
    ];
    let mut ok = true;
    for (p, want) in cases {
        let got = is_markdown_file(lang(p));
        println!("{:40} => {got} (want {want}) {}", p, if got==want {"OK"} else {ok=false; "MISMATCH"});
    }
    std::process::exit(if ok {0} else {1});
}
