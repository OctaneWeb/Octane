use std::ffi::OsStr;
use std::fs::{File, Metadata};
use std::io::{BufReader, Read, Result};
use std::path::PathBuf;
use crate::util::AsyncReader;

pub struct FileHandler {
    pub file_name: String,
    pub file: AsyncReader<File>,
    pub extension: String,
    pub meta: Metadata
}

impl FileHandler {
    pub fn handle_file(path: &PathBuf) -> Result<Option<Self>> {
        let file = File::open(path)?;
        if file.metadata()?.file_type().is_file() {
            let extension = path
                .as_path()
                .extension()
                .and_then(OsStr::to_str)
                .unwrap_or("");
            let meta = file.metadata()?;
            Ok(Some(FileHandler {
                file_name: path.file_name().and_then(OsStr::to_str).unwrap().to_owned(),
                file: AsyncReader::new(file),
                extension: extension.to_owned(),
                meta
            }))
        } else {
            Ok(None)
        }
    }
    pub fn get_extension(path: &PathBuf) -> String {
        Self::mime_type(
            path.as_path()
                .extension()
                .and_then(OsStr::to_str)
                .unwrap_or("")
                .to_string(),
        )
    }

    pub fn get_404_file() -> std::io::Result<Vec<u8>> {
        let file = File::open("templates/error.html")?;
        let mut buf_reader = BufReader::new(file);
        let mut contents = Vec::new();
        buf_reader.read_to_end(&mut contents)?;
        Ok(contents)
    }
    pub fn mime_type(extension: String) -> String {
        match extension.to_lowercase().as_str() {
            "html" | "htm" => "text/html",
            "css" => "text/css",
            "js" => "text/javascript",
            "txt" => "text/plain",
            "json" => "application/json",
            "png" => "image/png",
            _ => "text/plain",
        }
        .to_owned()
    }
}
