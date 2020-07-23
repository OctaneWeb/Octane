use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufReader, Read, Result};
use std::path::PathBuf;

pub struct FileHandler {
    pub file_name: String,
    pub contents: Vec<u8>,
    extension: String,
}

impl FileHandler {
    pub fn handle_file(path: &PathBuf) -> Result<Option<Self>> {
        let file = File::open(path)?;
        if file.metadata()?.file_type().is_file() {
            let extension = path.as_path().extension().and_then(OsStr::to_str).unwrap();

            let mut buf_reader = BufReader::new(file);
            let mut contents = Vec::new();
            buf_reader.read_to_end(&mut contents)?;
            Ok(Some(FileHandler {
                file_name: path.file_name().and_then(OsStr::to_str).unwrap().to_owned(),
                contents,
                extension: extension.to_owned(),
            }))
        } else {
            Ok(None)
        }
    }

    pub fn get_404_file() -> std::io::Result<Vec<u8>> {
        let file = File::open("templates/error.html")?;
        let mut buf_reader = BufReader::new(file);
        let mut contents = Vec::new();
        buf_reader.read_to_end(&mut contents)?;
        Ok(contents)
    }
    pub fn get_mime_type(&self) -> String {
        match self.extension.to_lowercase().as_str() {
            "html" | "htm" => "text/html",
            "css" => "text/css",
            "js" => "text/javascript",
            "txt" => "text/plain",
            "json" => "application/json",
            _ => "text/plain",
        }
        .to_owned()
    }
}
