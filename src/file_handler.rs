use crate::util::AsyncReader;
use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::fs::{File, Metadata};
use std::path::PathBuf;
/// The FileHandler structure is a helper struct
/// to manage files, contents and extensions also
/// to decide their mime types accordingly
pub struct FileHandler {
    pub file_name: String,
    pub file: AsyncReader<File>,
    pub extension: String,
    pub meta: Metadata,
}

#[derive(Debug, Copy, Clone)]
/// Custom error type for file handling errors
struct FileHandlerError {
    err_type: u8,
}

impl FileHandlerError {
    pub fn new(err_type: u8) -> Self {
        FileHandlerError { err_type }
    }
}

impl FileHandler {
    /// Takes a Pathbuf and returns a FileHandler struct
    pub fn handle_file(path: &PathBuf) -> Result<Self, Box<dyn Error>> {
        if let Ok(file) = File::open(path) {
            if file.metadata()?.file_type().is_file() {
                let extension = path
                    .as_path()
                    .extension()
                    .and_then(OsStr::to_str)
                    .unwrap_or("");
                let meta = file.metadata()?;
                Ok(FileHandler {
                    file_name: path.file_name().and_then(OsStr::to_str).unwrap().to_owned(),
                    file: AsyncReader::new(file),
                    extension: extension.to_owned(),
                    meta,
                })
            } else {
                Err(Box::new(FileHandlerError::new(1)))
            }
        } else {
            Err(Box::new(FileHandlerError::new(0)))
        }
    }
    /// A helper method to get extension from a
    /// PathBuf
    pub fn get_extension(path: &PathBuf) -> String {
        Self::mime_type(
            path.as_path()
                .extension()
                .and_then(OsStr::to_str)
                .unwrap_or("")
                .to_string(),
        )
    }
    /// Perform a match on the extension and
    /// return the mime type accordingly
    pub fn mime_type(extension: String) -> String {
        match extension.to_lowercase().as_str() {
            // text types
            "css" => "text/css",
            "csv" => "text/csv",
            "html" | "htm" => "text/html",
            "ics" => "text/calendar",
            "js" | "mjs" => "text/javascript",
            "txt" => "text/plain",
            // application types
            "abw" => "application/x-abiword",
            "arc" => "application/x-freearc",
            "azw" => "application/vnd.amazon.ebook",
            "bin" => "applcation/octet-stream",
            "bz" => "application/x-bzip",
            "bz2" => "application/x-bzip2",
            "csh" => "application/x-csh",
            "doc" => "application/msword",
            "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "eot" => "application/vnd.ms-fontobject",
            "epub" => "application/epub+zip",
            "gz" => "application/gzip",
            "java" => "application/java-archive",
            "json" => "application/json",
            "jsonld" => "application/ld+json",
            "mpkg" => "application/vnd.apple.installer+xml",
            "odp" => "application/vnd.oasis.opendocument.presentation",
            "ods" => "application/vnd.oasis.opendocument.spreadsheet",
            "odt" => "application/vnd.oasis.opendocument.text",
            "ogx" => "application/ogg",
            "pdf" => "application/pdf",
            "php" => "application/x-httpd-php",
            "ppt" => "application/vnd.ms-powerpoint",
            "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            "rar" => "application/vnd.rar",
            "rtf" => "application/rtf",
            "sh" => "application/x-sh",
            "swf" => "application/x-shockwave-flash",
            "tar" => "application/x-tar",
            "vsd" => "application/vnd.visio",
            "xhtml" => "application/xhtml+xml",
            "xls" => "application/vnd.ms-excel",
            "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            // TODO: let the user decide which type of xml they want to send
            "xml" => "application/xml",
            "xul" => "application/vnd.mozilla.xul+xml",
            "zip" => "application/zip",
            "7z" => "application/x-7z-compressed",
            // image types
            "bmp" => "image/bmp",
            "gif" => "image/gif",
            "ico" => "image/vnd.microsoft.icon",
            "jpeg" | "jpg" => "image/jpeg",
            "png" => "image/png",
            "svg" => "image/svg+xml",
            "tif" | "tiff" => "image/tiff",
            "webp" => "image/webp",
            // audio types
            "aac" => "audio/aac",
            "mid" => "audio/midi",
            "midi" => "audio/x-midi",
            "mp3" => "audio/mpeg",
            "oga" => "audio/ogg",
            "opus" => "audio/opus",
            "wav" => "audio/wav",
            "weba" => "audio/webm",
            // video types
            "avi" => "video/x-msvideo",
            "mpeg" => "video/mpeg",
            "ogv" => "video/ogg",
            "ts" => "video/mp2t",
            "webm" => "video/webm",
            // font types
            "otf" => "font/otf",
            "ttf" => "font/ttf",
            "woff" => "font/woff",
            "woff2" => "font/woff2",
            _ => "text/plain",
        }
        .to_owned()
    }
}

impl Display for FileHandlerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        let err_string = match self.err_type {
            0 => "File not found",
            1 => "Not a file",
            _ => "Unknown error",
        };
        write!(f, "File Handling error, {}", err_string)
    }
}

impl Error for FileHandlerError {}
