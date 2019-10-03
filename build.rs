use lazy_static::lazy_static;
use regex::Regex;
use std::env::var_os;
use std::ffi::OsStr;
use std::fmt::Write;
use std::fs::{read_dir, read_to_string, write};
use std::io::{ErrorKind, Result};
use std::path::{Path, PathBuf};

fn main() {
    if let Err(err) = preprocess_shaders("src/shaders", "glsl", "include") {
        panic!("error: failed to preprocess GLSL shader files: {}", err);
    }
}

fn preprocess_shaders(dir: &str, ext: &str, include_path: &str) -> Result<()> {
    let mut generated_file = String::from("/* autogenerated, do not edit */");

    for entry in read_dir(dir)? {
        let entry = entry?;

        if !entry.metadata()?.is_file() {
            continue;
        }

        let path = entry.path();

        if path.extension() == Some(&OsStr::new(ext)) {
            preprocess_glsl_shader(path, include_path)?;
            let path = PathBuf::from(entry.file_name());

            let mut name = path.file_stem().unwrap().to_str().unwrap().to_owned();
            name.make_ascii_uppercase(); // try to follow Rust's style conventions

            write!(
                generated_file,
                "\n/// GLSL source for the `{}` shader file.\n",
                path.display(),
            )
            .unwrap();

            write!(
                generated_file,
                "pub const {}: &str = include_str!(\"{}\");\n",
                name,
                path.display(),
            )
            .unwrap();
        }
    }

    let out_dir: PathBuf = var_os("OUT_DIR").unwrap().into();
    write(out_dir.join("glsl_shaders.rs"), &generated_file)?;

    Ok(())
}

fn preprocess_glsl_shader(path: PathBuf, include_path: &str) -> Result<()> {
    println!("cargo:rerun-if-changed={}", path.display());
    let (mut expanding, mut processed) = (vec![], vec![]);

    let shader = preprocess(
        &read_to_string(&path)?,
        path.file_name().unwrap().to_str().unwrap(),
        path.parent().unwrap(),
        &PathBuf::from(include_path),
        &mut expanding,
        &mut processed,
        false,
    )?;

    let out_dir: PathBuf = var_os("OUT_DIR").unwrap().into();
    write(out_dir.join(path.file_name().unwrap()), &shader)?;

    Ok(())
}

lazy_static! {
    static ref REGEX: Regex = Regex::new(r#"^\s*#\s*include\s+<([[:graph:]]*)>\s*$"#).unwrap();
}

fn preprocess(
    text: &str,
    name: &str,
    relative_path: &Path,
    includes_path: &Path,
    expanding: &mut Vec<String>,
    processed: &mut Vec<String>,
    placeholder: bool,
) -> Result<String> {
    println!(
        "cargo:rerun-if-changed={}",
        relative_path.join(name).display()
    );

    expanding.push(name.to_owned());

    let mut shader = format!("// __POS__ {}:0\n", name);

    if placeholder {
        shader += text;
    }

    for (index, line) in text.lines().enumerate() {
        if placeholder {
            break;
        }

        if let Some(captures) = REGEX.captures(line) {
            let header = captures.get(1).unwrap().as_str();

            if vec_contains(processed, &header) {
                continue; // previously included
            }

            if vec_contains(expanding, &header) {
                panic!("circular inclusion of GLSL header: {}", header);
            }

            // If we don't find the file, simply leave the #include in there to be populated
            // dynamically by the renderer. The file/line marker comments are still added in
            // which allows compilation errors inside dynamic includes to still be reported.

            let result = read_to_string(&relative_path.join(includes_path.join(&header)));

            let (text, placeholder) = match result {
                Err(err) if err.kind() == ErrorKind::NotFound => {
                    (format!("#include <{}>", header), true)
                }
                result => (result?, false),
            };

            let included = preprocess(
                &text,
                &header,
                relative_path,
                includes_path,
                expanding,
                processed,
                placeholder,
            )?;

            write!(shader, "{}\n// __POS__ {}:{}\n", included, name, index + 1).unwrap();
        } else {
            write!(shader, "{}\n", line).unwrap();
        }
    }

    processed.push(name.to_owned());

    Ok(shader)
}

fn vec_contains(vec: &Vec<String>, item: &str) -> bool {
    vec.iter().any(|x| x == item) // see issue #42671
}
