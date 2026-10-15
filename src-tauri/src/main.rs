#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

/// Join a vault folder with a relative path, refusing anything that could escape it.
fn safe_join(base: &str, rel: &str) -> Result<PathBuf, String> {
    let root = PathBuf::from(base);
    if !root.is_dir() {
        return Err("Vault folder not found".into());
    }
    let mut p = root;
    for c in Path::new(rel).components() {
        match c {
            Component::Normal(x) => p.push(x),
            Component::CurDir => {}
            _ => return Err("Invalid path".into()),
        }
    }
    Ok(p)
}

#[tauri::command]
fn append_note(vault: String, rel: String, content: String) -> Result<(), String> {
    let p = safe_join(&vault, &rel)?;
    if let Some(dir) = p.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&p)
        .map_err(|e| e.to_string())?;
    f.write_all(content.as_bytes()).map_err(|e| e.to_string())
}

#[tauri::command]
fn copy_file_into(vault: String, rel_dir: String, src: String) -> Result<String, String> {
    let dir = safe_join(&vault, &rel_dir)?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let s = PathBuf::from(&src);
    let name = s
        .file_name()
        .ok_or("Bad file")?
        .to_string_lossy()
        .to_string();
    let stem = s
        .file_stem()
        .map(|x| x.to_string_lossy().to_string())
        .unwrap_or_default();
    let ext = s
        .extension()
        .map(|x| format!(".{}", x.to_string_lossy()))
        .unwrap_or_default();
    let mut final_name = name;
    let mut n = 1;
    while dir.join(&final_name).exists() {
        final_name = format!("{}-{}{}", stem, n, ext);
        n += 1;
    }
    fs::copy(&s, dir.join(&final_name)).map_err(|e| e.to_string())?;
    Ok(final_name)
}

fn walk(dir: &Path, out: &mut Vec<String>) {
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().map(|x| x == "md").unwrap_or(false) {
                if let Ok(t) = fs::read_to_string(&p) {
                    out.push(t);
                }
            }
        }
    }
}

#[tauri::command]
fn read_notes(vault: String, rel: String) -> Result<Vec<String>, String> {
    let p = safe_join(&vault, &rel)?;
    let mut out = Vec::new();
    if p.is_dir() {
        walk(&p, &mut out);
    }
    Ok(out)
}

#[tauri::command]
fn open_link(url: String) -> Result<(), String> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("Only http(s) links are allowed".into());
    }
    open::that(url).map_err(|e| e.to_string())
}

#[tauri::command]
fn open_file(vault: String, rel: String) -> Result<(), String> {
    let p = safe_join(&vault, &rel)?;
    open::that(p).map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            append_note,
            copy_file_into,
            read_notes,
            open_link,
            open_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running the application");
}
