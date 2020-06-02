use std::ffi::OsStr;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::io::BufReader;

pub struct FileHandler {
    pub file_name: String,
    pub contents: Vec<u8>,
    extension: String,
}

impl FileHandler {
    pub async fn handle_file(file_name: &str) -> std::io::Result<Option<Self>> {
        let file = File::open(file_name).await?;
        if file.metadata().await?.file_type().is_file() {
            let extension = Path::new(&file_name)
                .extension()
                .and_then(OsStr::to_str)
                .unwrap();
            let mut buf_reader = BufReader::new(file);
            let mut contents = Vec::new();
            buf_reader.read_to_end(&mut contents).await?;
            Ok(Some(FileHandler {
                file_name: file_name.to_owned(),
                contents,
                extension: extension.to_owned(),
            }))
        } else {
            Ok(None)
        }
    }
    pub fn get_mime_type(&self) -> String {
        match self.extension.to_lowercase().as_str() {
            "html" | "htm" => "text/html",
            "css" => "text/css",
            "js" => "text/javascript",
            "txt" => "text/plain",
            _ => "",
        }
        .to_owned()
    }
}
