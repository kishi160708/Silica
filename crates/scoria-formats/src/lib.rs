//! On-disk formats  must read and write byte-compatibly with Obsidian.
//!
//! Every type here round-trips unknown keys (`#[serde(flatten)] extra`) so that
//! a vault edited by Obsidian after  never loses data it does not understand.

pub mod base;
pub mod canvas;
pub mod config;

/// How Obsidian treats a file, decided purely by extension (case-insensitive).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileKind {
    Markdown,
    Canvas,
    Base,
    Image,
    Audio,
    Video,
    Pdf,
    /// Not a known type; hidden unless "Show all file types" is on.
    Other,
}

/// Extension tables as Obsidian 1.12.7 classifies files.
pub const IMAGE_EXTENSIONS: &[&str] = &["bmp", "png", "jpg", "jpeg", "gif", "svg", "webp", "avif"];
pub const AUDIO_EXTENSIONS: &[&str] = &["mp3", "wav", "m4a", "3gp", "flac", "ogg", "oga", "opus"];
pub const VIDEO_EXTENSIONS: &[&str] = &["mp4", "webm", "ogv", "mov", "mkv"];

impl FileKind {
    pub fn from_extension(ext: &str) -> FileKind {
        let ext = ext.to_ascii_lowercase();
        match ext.as_str() {
            "md" => FileKind::Markdown,
            "canvas" => FileKind::Canvas,
            "base" => FileKind::Base,
            "pdf" => FileKind::Pdf,
            e if IMAGE_EXTENSIONS.contains(&e) => FileKind::Image,
            e if AUDIO_EXTENSIONS.contains(&e) => FileKind::Audio,
            e if VIDEO_EXTENSIONS.contains(&e) => FileKind::Video,
            _ => FileKind::Other,
        }
    }

    /// Classifies a vault-relative path such as `folder/Note.md`.
    pub fn from_path(path: &str) -> FileKind {
        match path.rsplit_once('.') {
            Some((stem, ext)) if !stem.is_empty() && !ext.contains('/') => {
                FileKind::from_extension(ext)
            }
            _ => FileKind::Other,
        }
    }

    /// Whether a link to a file of this kind may omit its extension (`[[Note]]`).
    pub fn is_markdown(self) -> bool {
        matches!(self, FileKind::Markdown)
    }
}

/// All extensions Obsidian resolves when a link omits one, in the order tried
/// (`.md` first, then every other known type).
pub fn link_resolution_extensions() -> impl Iterator<Item = &'static str> {
    std::iter::once("md")
        .chain(["canvas", "base", "pdf"])
        .chain(IMAGE_EXTENSIONS.iter().copied())
        .chain(AUDIO_EXTENSIONS.iter().copied())
        .chain(VIDEO_EXTENSIONS.iter().copied())
}

/// Characters Obsidian refuses in file names.
pub const FORBIDDEN_NAME_CHARS: &[char] = &['*', '"', '\\', '/', '<', '>', ':', '|', '?'];

/// Validates a note title the way the explorer's rename does.
pub fn validate_file_name(name: &str) -> Result<(), &'static str> {
    if name.trim().is_empty() {
        return Err("File name cannot be empty.");
    }
    if name.starts_with('.') {
        return Err("File name must not start with a dot.");
    }
    if name.contains(FORBIDDEN_NAME_CHARS) {
        return Err("File name cannot contain any of these characters: * \" \\ / < > : | ?");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_by_extension_case_insensitively() {
        assert_eq!(FileKind::from_path("a/b/Note.MD"), FileKind::Markdown);
        assert_eq!(FileKind::from_path("Board.canvas"), FileKind::Canvas);
        assert_eq!(FileKind::from_path("x.base"), FileKind::Base);
        assert_eq!(FileKind::from_path("img.WebP"), FileKind::Image);
        assert_eq!(FileKind::from_path("clip.opus"), FileKind::Audio);
        assert_eq!(FileKind::from_path("movie.mkv"), FileKind::Video);
        assert_eq!(FileKind::from_path("doc.pdf"), FileKind::Pdf);
        assert_eq!(FileKind::from_path("archive.zip"), FileKind::Other);
        assert_eq!(FileKind::from_path("noext"), FileKind::Other);
        assert_eq!(FileKind::from_path("dir.v1/noext"), FileKind::Other);
    }

    #[test]
    fn file_name_rules() {
        assert!(validate_file_name("Good name").is_ok());
        assert!(validate_file_name(".hidden").is_err());
        assert!(validate_file_name("bad:name").is_err());
        assert!(validate_file_name("   ").is_err());
    }
}
