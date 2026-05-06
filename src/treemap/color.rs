use crate::scanner::{DirNode, Entry, FileEntry};
use egui::Color32;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum FileCategory {
    Document, Image, Video, Audio, Archive,
    Code, Executable, System, Temp, Other,
}

impl FileCategory {
    /// 色块主体颜色（上部 60% 纯色区域）
    pub fn color(&self) -> Color32 {
        match self {
            FileCategory::Document   => Color32::from_rgb(40, 100, 200),
            FileCategory::Image      => Color32::from_rgb(46, 139, 87),
            FileCategory::Video      => Color32::from_rgb(220, 20, 60),
            FileCategory::Audio      => Color32::from_rgb(255, 140, 0),
            FileCategory::Archive    => Color32::from_rgb(128, 0, 128),
            FileCategory::Code       => Color32::from_rgb(0, 128, 128),
            FileCategory::Executable => Color32::from_rgb(184, 134, 11),
            FileCategory::System     => Color32::from_rgb(120, 120, 140),
            FileCategory::Temp       => Color32::from_rgb(255, 100, 100),
            FileCategory::Other      => Color32::from_rgb(160, 120, 200),
        }
    }

    /// 渐变色目标（下部 40% 渐变终点）—— 同色系更浅的颜色
    pub fn gradient_end(&self) -> Color32 {
        match self {
            FileCategory::Document   => Color32::from_rgb(140, 180, 230),
            FileCategory::Image      => Color32::from_rgb(130, 200, 160),
            FileCategory::Video      => Color32::from_rgb(240, 120, 130),
            FileCategory::Audio      => Color32::from_rgb(255, 200, 120),
            FileCategory::Archive    => Color32::from_rgb(190, 120, 190),
            FileCategory::Code       => Color32::from_rgb(100, 190, 190),
            FileCategory::Executable => Color32::from_rgb(220, 190, 110),
            FileCategory::System     => Color32::from_rgb(180, 180, 200),
            FileCategory::Temp       => Color32::from_rgb(255, 170, 170),
            FileCategory::Other      => Color32::from_rgb(210, 180, 240),
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            FileCategory::Document   => "文档",
            FileCategory::Image      => "图片",
            FileCategory::Video      => "视频",
            FileCategory::Audio      => "音频",
            FileCategory::Archive    => "压缩包",
            FileCategory::Code       => "代码",
            FileCategory::Executable => "可执行",
            FileCategory::System     => "系统",
            FileCategory::Temp       => "临时",
            FileCategory::Other      => "其他",
        }
    }
}

pub fn categorize(path: &Path) -> FileCategory {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "txt" | "doc" | "docx" | "pdf" | "xls" | "xlsx" | "ppt" | "pptx" | "rtf" | "odt" => FileCategory::Document,
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" | "webp" | "ico" | "tiff" | "tif" | "raw" | "heic" => FileCategory::Image,
        "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" | "m4v" | "mpg" | "mpeg" => FileCategory::Video,
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" | "m4a" | "opus" | "ape" => FileCategory::Audio,
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "lz" | "cab" | "iso" => FileCategory::Archive,
        "rs" | "py" | "js" | "ts" | "java" | "c" | "cpp" | "h" | "hpp" | "go" | "rb" | "php" | "swift" | "kt" | "scala" | "html" | "css" | "xml" | "json" | "yaml" | "yml" | "toml" | "sql" | "sh" | "bat" | "ps1" => FileCategory::Code,
        "exe" | "msi" | "com" | "scr" => FileCategory::Executable,
        "dll" | "sys" | "drv" | "ocx" | "cpl" | "efi" => FileCategory::System,
        "tmp" | "temp" | "bak" | "old" | "log" | "cache" => FileCategory::Temp,
        _ => FileCategory::Other,
    }
}

pub fn categorize_entry(entry: &Entry) -> FileCategory {
    match entry {
        Entry::File(f) => categorize(Path::new(&f.name)),
        Entry::Dir(d) => categorize(Path::new(&d.name)),
        Entry::Others(o) => categorize(Path::new(&o.name)),
        Entry::Symlink(p) => categorize(p),
        Entry::AccessDenied { path } => categorize(path),
    }
}

/// 预计算目录的主导颜色（在 finish() 中调用一次，结果存入 DirNode.dominant_cat）
pub fn compute_dominant(dir: &DirNode) -> FileCategory {
    let mut size_by_cat: std::collections::HashMap<FileCategory, u64> = std::collections::HashMap::new();
    accumulate_categories(dir, &mut size_by_cat);
    size_by_cat.into_iter()
        .max_by_key(|&(_, size)| size)
        .map(|(cat, _)| cat)
        .unwrap_or(FileCategory::Other)
}

/// 向后兼容：运行时计算（慢，不推荐在热路径使用）
pub fn dominant_category(dir: &DirNode) -> FileCategory {
    dir.dominant_cat
}

fn accumulate_categories(dir: &DirNode, acc: &mut std::collections::HashMap<FileCategory, u64>) {
    for child in &dir.children {
        match child {
            Entry::Dir(d) => accumulate_categories(d, acc),
            _ => {
                let cat = categorize_entry(child);
                *acc.entry(cat).or_insert(0) += child.size();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_document() {
        assert_eq!(categorize(Path::new("test.txt")), FileCategory::Document);
        assert_eq!(categorize(Path::new("report.pdf")), FileCategory::Document);
        assert_eq!(categorize(Path::new("data.xlsx")), FileCategory::Document);
    }

    #[test]
    fn test_categorize_image() {
        assert_eq!(categorize(Path::new("photo.jpg")), FileCategory::Image);
        assert_eq!(categorize(Path::new("icon.png")), FileCategory::Image);
    }

    #[test]
    fn test_categorize_video() {
        assert_eq!(categorize(Path::new("movie.mp4")), FileCategory::Video);
        assert_eq!(categorize(Path::new("clip.mkv")), FileCategory::Video);
    }

    #[test]
    fn test_categorize_audio() {
        assert_eq!(categorize(Path::new("song.mp3")), FileCategory::Audio);
        assert_eq!(categorize(Path::new("track.flac")), FileCategory::Audio);
    }

    #[test]
    fn test_categorize_archive() {
        assert_eq!(categorize(Path::new("data.zip")), FileCategory::Archive);
        assert_eq!(categorize(Path::new("backup.7z")), FileCategory::Archive);
    }

    #[test]
    fn test_categorize_code() {
        assert_eq!(categorize(Path::new("main.rs")), FileCategory::Code);
        assert_eq!(categorize(Path::new("app.py")), FileCategory::Code);
    }

    #[test]
    fn test_categorize_executable() {
        assert_eq!(categorize(Path::new("app.exe")), FileCategory::Executable);
        assert_eq!(categorize(Path::new("setup.msi")), FileCategory::Executable);
    }

    #[test]
    fn test_categorize_system() {
        assert_eq!(categorize(Path::new("kernel.dll")), FileCategory::System);
        assert_eq!(categorize(Path::new("driver.sys")), FileCategory::System);
    }

    #[test]
    fn test_categorize_temp() {
        assert_eq!(categorize(Path::new("cache.tmp")), FileCategory::Temp);
        assert_eq!(categorize(Path::new("old.log")), FileCategory::Temp);
    }

    #[test]
    fn test_categorize_other() {
        assert_eq!(categorize(Path::new("data.xyz")), FileCategory::Other);
        assert_eq!(categorize(Path::new("noext")), FileCategory::Other);
    }

    #[test]
    fn test_document_color() {
        let c = FileCategory::Document.color();
        assert_eq!(c.r(), 40);
        assert_eq!(c.g(), 100);
        assert_eq!(c.b(), 200);
    }

    #[test]
    fn test_dominant_category_documents() {
        use std::path::PathBuf;
        let dir = DirNode {
            path: PathBuf::from(r"C:\docs"),
            name: "docs".to_string(),
            total_size: 300,
            file_count: 3,
            children: vec![
                Entry::File(FileEntry { name: "a.txt".to_string(), size: 100 }),
                Entry::File(FileEntry { name: "b.pdf".to_string(), size: 100 }),
                Entry::File(FileEntry { name: "c.jpg".to_string(), size: 100 }),
            ],
            access_denied: false,
            dominant_cat: FileCategory::Other,
        };
        let dom = compute_dominant(&dir);
        assert_eq!(dom, FileCategory::Document);
    }
}
