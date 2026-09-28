//! File families, mirrored by `FileKind` in the UI (`ui/src/lib/types.ts`).

use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileKind {
    Pdf,
    Word,
    Excel,
    Powerpoint,
    Text,
    Code,
    Archive,
    Email,
    /// Photos, scans, screenshots: their text is read by OCR (lot 5.2).
    Image,
}

const TEXT_EXT: &[&str] = &[
    "txt", "md", "markdown", "rst", "log", "csv", "tsv", "json", "jsonl", "xml", "yaml", "yml", "toml", "ini", "cfg",
    "conf", "properties", "env", "html", "htm", "srt", "vtt", "tex", "adoc", "org", "epub",
];

const CODE_EXT: &[&str] = &[
    "rs", "ts", "tsx", "js", "jsx", "mjs", "cjs", "py", "pyw", "java", "kt", "kts", "go", "c", "h", "cc", "cpp", "cxx",
    "hpp", "hh", "cs", "fs", "vb", "php", "rb", "swift", "scala", "sh", "bash", "zsh", "fish", "bat", "cmd", "ps1",
    "psm1", "sql", "css", "scss", "sass", "less", "vue", "svelte", "astro", "lua", "dart", "r", "pl", "pm", "ex", "exs",
    "erl", "hs", "clj", "groovy", "gradle", "m", "mm", "zig", "nim", "jl", "sol", "proto", "graphql", "dockerfile",
    "makefile", "cmake",
];

impl FileKind {
    pub fn as_str(self) -> &'static str {
        match self {
            FileKind::Pdf => "pdf",
            FileKind::Word => "word",
            FileKind::Excel => "excel",
            FileKind::Powerpoint => "powerpoint",
            FileKind::Text => "text",
            FileKind::Code => "code",
            FileKind::Archive => "archive",
            FileKind::Email => "email",
            FileKind::Image => "image",
        }
    }

    pub fn from_code(s: &str) -> Option<FileKind> {
        Some(match s {
            "pdf" => FileKind::Pdf,
            "word" => FileKind::Word,
            "excel" => FileKind::Excel,
            "powerpoint" => FileKind::Powerpoint,
            "text" => FileKind::Text,
            "code" => FileKind::Code,
            "archive" => FileKind::Archive,
            "email" => FileKind::Email,
            "image" => FileKind::Image,
            _ => return None,
        })
    }

    /// Family of a file from its extension (or its name, for `Dockerfile`…).
    pub fn from_path(path: &Path) -> Option<FileKind> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .or_else(|| path.file_name().and_then(|n| n.to_str()).map(str::to_ascii_lowercase))?;
        let ext = ext.as_str();
        Some(match ext {
            "pdf" => FileKind::Pdf,
            "docx" | "docm" | "doc" | "odt" | "rtf" => FileKind::Word,
            "xlsx" | "xlsm" | "xls" | "ods" => FileKind::Excel,
            "pptx" | "pptm" | "ppt" | "odp" => FileKind::Powerpoint,
            "zip" | "jar" | "rar" | "7z" | "tar" | "gz" | "tgz" | "bz2" | "tbz2" | "tbz" => FileKind::Archive,
            "eml" | "msg" | "pst" | "ost" | "mbox" => FileKind::Email,
            "png" | "jpg" | "jpeg" | "jfif" | "tif" | "tiff" | "bmp" | "gif" | "webp" => FileKind::Image,
            e if CODE_EXT.contains(&e) => FileKind::Code,
            e if TEXT_EXT.contains(&e) => FileKind::Text,
            _ => return None,
        })
    }
}
