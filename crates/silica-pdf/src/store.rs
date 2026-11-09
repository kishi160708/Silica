//! The sidecar file that keeps annotations across sessions.
//!
//! One JSON file per PDF, at `<vault>/.silica/annotations/<file>.json`, so a
//! reader never writes ink into the PDF itself and a vault can still be
//! copied, diffed, or have its annotations thrown away wholesale.
//!
//! Every read is defensive: a missing file is an empty layer, and anything
//! malformed — truncated JSON, an unknown version, a rectangle that is not
//! a rectangle — comes back as [`StoreError::Corrupt`] with the reason,
//! never as a silently half-loaded layer and never as a panic. Writes go
//! through a sibling temporary file and a rename, so a crash mid-write
//! leaves the previous file intact.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::annotate::{Annotation, AnnotationId, AnnotationLayer, Kind, Rect};
use crate::json::{self, Value};

/// The format this build writes and accepts. A sidecar carrying anything
/// else is refused rather than guessed at.
const FORMAT_VERSION: u64 = 1;

/// A problem with one sidecar file.
#[derive(Debug)]
pub enum StoreError {
    /// The file could not be read or written.
    Io(io::Error),
    /// The file is not a sidecar this build understands, and says why.
    Corrupt(String),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Io(why) => write!(f, "the annotations file could not be used: {why}"),
            StoreError::Corrupt(why) => write!(f, "the annotations file is not readable: {why}"),
        }
    }
}

impl std::error::Error for StoreError {}

/// The sidecar path for `pdf_file_name` under `root`.
/// `None` when the name cannot be one path segment — an empty name, `.`,
/// `..`, or one carrying a separator — because such a name would point
/// outside the sidecar directory.
pub fn sidecar_path(root: &Path, pdf_file_name: &str) -> Option<PathBuf> {
    let single_segment = !pdf_file_name.is_empty()
        && pdf_file_name != "."
        && pdf_file_name != ".."
        && !pdf_file_name.contains('/')
        && !pdf_file_name.contains('\\')
        && !pdf_file_name.contains('\0');
    single_segment.then(|| {
        root.join(".silica")
            .join("annotations")
            .join(format!("{pdf_file_name}.json"))
    })
}

/// The annotations of one PDF, backed by its sidecar file.
pub struct AnnotationStore {
    path: PathBuf,
    file_name: String,
    layer: AnnotationLayer,
    dirty: bool,
}

impl AnnotationStore {
    /// Open the sidecar for `pdf_file_name` under `root`. A file that is not
    /// there yet is an empty store that saves on first use; nothing is
    /// written until then.
    pub fn open(root: &Path, pdf_file_name: &str) -> Result<AnnotationStore, StoreError> {
        let path = sidecar_path(root, pdf_file_name).ok_or_else(|| {
            StoreError::Corrupt(format!("{pdf_file_name:?} is not one file name"))
        })?;
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(StoreError::Io(e)),
        };
        let layer = decode(&text, pdf_file_name)?;
        Ok(AnnotationStore {
            path,
            file_name: pdf_file_name.to_string(),
            layer,
            dirty: false,
        })
    }

    /// The sidecar file's path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The PDF's file name this store belongs to.
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    /// The annotations, in insertion order.
    pub fn annotations(&self) -> &AnnotationLayer {
        &self.layer
    }

    /// True while changes since the last save have not been written yet.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Add one annotation and write the sidecar, so a crash right after the
    /// user finishes a highlight keeps it.
    pub fn add(
        &mut self,
        page: usize,
        kind: Kind,
        rect: Rect,
        author: Option<String>,
    ) -> Result<AnnotationId, StoreError> {
        let id = self.layer.add(page, kind, rect, author);
        self.dirty = true;
        self.save()?;
        Ok(id)
    }

    /// Remove one annotation and write the sidecar. Removing an id that is
    /// not there is a no-op: the file is left alone.
    pub fn remove(&mut self, id: AnnotationId) -> Result<(), StoreError> {
        if self.layer.remove(id) {
            self.dirty = true;
            self.save()?;
        }
        Ok(())
    }

    /// Write the sidecar, creating `.silica/annotations/` if the vault has
    /// no annotations yet. Safe to call twice: the second write holds the
    /// same bytes.
    pub fn save(&mut self) -> Result<(), StoreError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(StoreError::Io)?;
        }
        let text = encode(&self.file_name, &self.layer);
        let mut temporary = self.path.clone().into_os_string();
        temporary.push(".tmp");
        let temporary = PathBuf::from(temporary);
        fs::write(&temporary, text).map_err(StoreError::Io)?;
        if let Err(e) = fs::rename(&temporary, &self.path) {
            let _ = fs::remove_file(&temporary);
            return Err(StoreError::Io(e));
        }
        self.dirty = false;
        Ok(())
    }
}

/// The sidecar text for a layer. The `file` entry is only a label; the path
/// it lives at is what ties it to a PDF.
fn encode(file_name: &str, layer: &AnnotationLayer) -> String {
    let annotations: Vec<Value> = layer
        .iter()
        .map(|item| {
            let (kind, contents) = kind_parts(&item.kind);
            let mut entries = vec![
                ("id".to_string(), Value::Number(item.id.as_u64() as f64)),
                ("page".to_string(), Value::Number(item.page as f64)),
                ("kind".to_string(), Value::Text(kind.to_string())),
                (
                    "rect".to_string(),
                    Value::Object(vec![
                        ("x0".to_string(), Value::Number(f64::from(item.rect.x0))),
                        ("y0".to_string(), Value::Number(f64::from(item.rect.y0))),
                        ("x1".to_string(), Value::Number(f64::from(item.rect.x1))),
                        ("y1".to_string(), Value::Number(f64::from(item.rect.y1))),
                    ]),
                ),
            ];
            if let Some(contents) = contents {
                entries.push(("contents".to_string(), Value::Text(contents.to_string())));
            }
            if let Some(author) = &item.author {
                entries.push(("author".to_string(), Value::Text(author.clone())));
            }
            Value::Object(entries)
        })
        .collect();
    let document = Value::Object(vec![
        ("version".to_string(), Value::Number(FORMAT_VERSION as f64)),
        ("file".to_string(), Value::Text(file_name.to_string())),
        ("annotations".to_string(), Value::Array(annotations)),
    ]);
    json::write(&document)
}

/// A layer read back from `text`. Empty text is an empty layer: an empty
/// file behaves like "no annotations yet". Anything that parses but does
/// not describe a sidecar — a wrong version, a missing list, a rectangle
/// that is not a rectangle — is refused, so a damaged file can never load
/// as half the annotations it claims to hold.
fn decode(text: &str, expected_file_name: &str) -> Result<AnnotationLayer, StoreError> {
    if text.trim().is_empty() {
        return Ok(AnnotationLayer::new());
    }
    let document = json::parse(text).map_err(StoreError::Corrupt)?;
    let version = document
        .entry("version")
        .and_then(Value::as_whole)
        .ok_or_else(|| StoreError::Corrupt("the version is missing or not a number".into()))?;
    if version != FORMAT_VERSION {
        return Err(StoreError::Corrupt(format!(
            "version {version} is not version {FORMAT_VERSION}"
        )));
    }
    if let Some(file) = document.entry("file").and_then(Value::as_text) {
        if file != expected_file_name {
            return Err(StoreError::Corrupt(format!(
                "the file holds annotations for {file:?}, not {expected_file_name:?}"
            )));
        }
    }
    let list = document
        .entry("annotations")
        .and_then(Value::as_array)
        .ok_or_else(|| StoreError::Corrupt("the annotations list is missing".into()))?;

    // Ids are optional in the file: without them the layer mints fresh ones
    // in order, which is what a hand-written sidecar gets.
    let mut next_id = 1u64;
    let mut items = Vec::with_capacity(list.len());
    for (index, entry) in list.iter().enumerate() {
        let id = match entry.entry("id").and_then(Value::as_whole) {
            Some(saved) => {
                next_id = next_id.max(saved.saturating_add(1));
                AnnotationId::from_raw(saved)
            }
            None => {
                let minted = AnnotationId::from_raw(next_id);
                next_id += 1;
                minted
            }
        };
        let page = entry
            .entry("page")
            .and_then(Value::as_whole)
            .ok_or_else(|| StoreError::Corrupt(format!("annotation {index} has no page")))?
            as usize;
        let kind_name = entry
            .entry("kind")
            .and_then(Value::as_text)
            .ok_or_else(|| StoreError::Corrupt(format!("annotation {index} has no kind")))?
            .to_string();
        let kind = kind_from_parts(&kind_name, entry.entry("contents")).map_err(|why| {
            StoreError::Corrupt(format!("annotation {index} of kind {kind_name:?}: {why}"))
        })?;
        let rect = rect_from_value(entry.entry("rect")).map_err(|why| {
            StoreError::Corrupt(format!("annotation {index} has an unusable rect: {why}"))
        })?;
        let author = entry
            .entry("author")
            .and_then(Value::as_text)
            .map(str::to_string);
        items.push(Annotation {
            id,
            page,
            kind,
            rect,
            author,
        });
    }
    Ok(AnnotationLayer::restored(items))
}

/// The stored name of a kind, plus its text for the note kind.
fn kind_parts(kind: &Kind) -> (&'static str, Option<&str>) {
    match kind {
        Kind::Highlight => ("highlight", None),
        Kind::Underline => ("underline", None),
        Kind::Squiggly => ("squiggly", None),
        Kind::StrikeOut => ("strikeout", None),
        Kind::Text { contents } => ("text", Some(contents.as_str())),
    }
}

fn kind_from_parts(name: &str, contents: Option<&Value>) -> Result<Kind, String> {
    match name {
        "highlight" => Ok(Kind::Highlight),
        "underline" => Ok(Kind::Underline),
        "squiggly" => Ok(Kind::Squiggly),
        "strikeout" => Ok(Kind::StrikeOut),
        "text" => {
            let contents = contents
                .and_then(Value::as_text)
                .ok_or_else(|| "a comment needs its contents".to_string())?;
            Ok(Kind::Text {
                contents: contents.to_string(),
            })
        }
        other => Err(format!("{other:?} is not a kind this build knows")),
    }
}

fn rect_from_value(value: Option<&Value>) -> Result<Rect, String> {
    let object = value.ok_or_else(|| "the rect is missing".to_string())?;
    let side = |name: &str| -> Result<f32, String> {
        object
            .entry(name)
            .and_then(Value::as_number)
            .map(|n| n as f32)
            .ok_or_else(|| format!("{name} is missing or not a number"))
    };
    let x0 = side("x0")?;
    let y0 = side("y0")?;
    let x1 = side("x1")?;
    let y1 = side("y1")?;
    // Tolerate the two corners arriving in either order: the rectangle on
    // the page is the same either way, and a hand-edited file should not
    // stop the PDF from opening.
    Ok(Rect {
        x0: x0.min(x1),
        y0: y0.min(y1),
        x1: x0.max(x1),
        y1: y0.max(y1),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal temp dir: the crate has no `tempfile` dependency and these
    /// tests are the only place a vault is needed.
    mod tempdir {
        use std::path::{Path, PathBuf};

        pub struct TempDir(PathBuf);

        impl TempDir {
            pub fn new(prefix: &str) -> std::io::Result<TempDir> {
                use std::sync::atomic::{AtomicU64, Ordering};
                static NEXT: AtomicU64 = AtomicU64::new(0);
                // * A counter keeps two tests started in the same
                //   millisecond apart, since one's `Drop` removes the
                //   directory the other is using.
                let path = std::env::temp_dir().join(format!(
                    "{prefix}-{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                ));
                std::fs::create_dir_all(&path)?;
                Ok(TempDir(path))
            }

            pub fn path(&self) -> &Path {
                &self.0
            }
        }

        impl Drop for TempDir {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
    }

    fn vault() -> tempdir::TempDir {
        tempdir::TempDir::new("silica-pdf-store").expect("a temp vault")
    }

    fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
        Rect { x0, y0, x1, y1 }
    }

    #[test]
    fn sidecar_path_lives_under_the_annotations_dir() {
        let path = sidecar_path(Path::new("D:/vault"), "paper.pdf").expect("a plain name");
        assert_eq!(
            path,
            PathBuf::from("D:/vault/.silica/annotations/paper.pdf.json")
        );

        for bad in ["", ".", "..", "a/b", "a\\b", "..\\..\\x", "\0"] {
            assert!(
                sidecar_path(Path::new("D:/vault"), bad).is_none(),
                "{bad:?} must not address a file"
            );
        }
    }

    #[test]
    fn a_missing_file_is_an_empty_store() {
        let root = vault();
        let store = AnnotationStore::open(root.path(), "paper.pdf").expect("opens");
        assert!(store.annotations().is_empty());
        assert!(!store.is_dirty());
        assert!(!store.path().exists(), "nothing is written yet");
        assert_eq!(store.file_name(), "paper.pdf");
    }

    #[test]
    fn add_persists_and_reloads_every_annotation() {
        let root = vault();
        let mut store = AnnotationStore::open(root.path(), "paper.pdf").expect("opens");
        let highlight = store
            .add(
                0,
                Kind::Highlight,
                rect(10.0, 20.0, 30.0, 40.0),
                Some("alice".into()),
            )
            .expect("saved");
        let comment = store
            .add(
                1,
                Kind::Text {
                    contents: "second read".into(),
                },
                rect(0.0, 0.0, 5.0, 5.0),
                None,
            )
            .expect("saved");
        assert!(
            !store.is_dirty(),
            "saving right after adding clears the flag"
        );

        let reopened = AnnotationStore::open(root.path(), "paper.pdf").expect("reopens");
        let items: Vec<&Annotation> = reopened.annotations().iter().collect();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id, highlight);
        assert_eq!(items[0].page, 0);
        assert_eq!(items[0].kind, Kind::Highlight);
        assert_eq!(items[0].rect, rect(10.0, 20.0, 30.0, 40.0));
        assert_eq!(items[0].author.as_deref(), Some("alice"));
        assert_eq!(items[1].id, comment);
        assert_eq!(
            items[1].kind,
            Kind::Text {
                contents: "second read".into()
            }
        );
        assert_eq!(items[1].author, None);
    }

    #[test]
    fn removal_survives_a_reload_and_keeps_the_remaining_ids() {
        let root = vault();
        let mut store = AnnotationStore::open(root.path(), "paper.pdf").expect("opens");
        let first = store
            .add(0, Kind::Highlight, rect(0.0, 0.0, 1.0, 1.0), None)
            .unwrap();
        let second = store
            .add(0, Kind::Underline, rect(0.0, 0.0, 1.0, 1.0), None)
            .unwrap();
        store.remove(first).expect("removed");

        let reopened = AnnotationStore::open(root.path(), "paper.pdf").expect("reopens");
        assert_eq!(reopened.annotations().len(), 1);
        assert_eq!(
            reopened
                .annotations()
                .annotation(second)
                .map(|item| item.id),
            Some(second)
        );

        // Adding after a reload does not reuse the removed id.
        let mut reopened = reopened;
        let third = reopened
            .add(0, Kind::Squiggly, rect(0.0, 0.0, 1.0, 1.0), None)
            .unwrap();
        assert_eq!(third.as_u64(), second.as_u64() + 1);
    }

    #[test]
    fn save_creates_the_directory_and_leaves_no_temp_file() {
        let root = vault();
        let mut store = AnnotationStore::open(root.path(), "paper.pdf").expect("opens");
        store
            .add(0, Kind::Highlight, rect(0.0, 0.0, 1.0, 1.0), None)
            .unwrap();
        assert!(store.path().exists());
        let sibling = store.path().with_file_name("paper.pdf.json.tmp");
        assert!(!sibling.exists(), "the temp file is renamed away");
    }

    #[test]
    fn saving_twice_holds_the_same_bytes() {
        let root = vault();
        let mut store = AnnotationStore::open(root.path(), "paper.pdf").expect("opens");
        store
            .add(0, Kind::Highlight, rect(0.0, 0.0, 1.0, 1.0), None)
            .unwrap();
        let first = std::fs::read_to_string(store.path()).unwrap();
        store.save().expect("second save");
        let second = std::fs::read_to_string(store.path()).unwrap();
        assert_eq!(first, second);
        assert!(first.contains("\"version\": 1"), "versioned file: {first}");
    }

    #[test]
    fn an_empty_file_loads_as_empty_and_still_accepts_annotations() {
        let root = vault();
        let path = sidecar_path(root.path(), "paper.pdf").unwrap();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "").unwrap();
        let mut store = AnnotationStore::open(root.path(), "paper.pdf").expect("opens");
        assert!(store.annotations().is_empty());
        store
            .add(0, Kind::Highlight, rect(0.0, 0.0, 1.0, 1.0), None)
            .unwrap();
        let reopened = AnnotationStore::open(root.path(), "paper.pdf").expect("reopens");
        assert_eq!(reopened.annotations().len(), 1);
    }

    #[test]
    fn a_file_this_build_cannot_read_is_refused() {
        let root = vault();
        let path = sidecar_path(root.path(), "paper.pdf").unwrap();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();

        let cases = [
            "{\"version\": 1, \"file\": \"paper.pdf\", \"ann",
            "{\"version\": 1, \"file\": \"other.pdf\", \"annotations\": []}",
            "[1, 2, 3]",
            "{\"version\": 2, \"file\": \"paper.pdf\", \"annotations\": []}",
            "{\"version\": 1, \"file\": \"paper.pdf\"}",
            "{\"version\": 1, \"file\": \"paper.pdf\", \"annotations\": [{\"kind\": \"highlight\"}]}",
            "{\"version\": 1, \"file\": \"paper.pdf\", \"annotations\": [{\"page\": 0, \"kind\": \"highlight\", \"rect\": {\"x0\": 0, \"y0\": 0, \"x1\": 1}}]}",
            "{\"version\": 1, \"file\": \"paper.pdf\", \"annotations\": [{\"page\": -1, \"kind\": \"highlight\", \"rect\": {\"x0\": 0, \"y0\": 0, \"x1\": 1, \"y1\": 1}}]}",
            "{\"version\": 1, \"file\": \"paper.pdf\", \"annotations\": [{\"page\": 0, \"kind\": \"glitter\", \"rect\": {\"x0\": 0, \"y0\": 0, \"x1\": 1, \"y1\": 1}}]}",
            "{\"version\": 1, \"file\": \"paper.pdf\", \"annotations\": [{\"page\": 0, \"kind\": \"text\", \"rect\": {\"x0\": 0, \"y0\": 0, \"x1\": 1, \"y1\": 1}}]}",
        ];
        for text in cases {
            std::fs::write(&path, text).unwrap();
            let error = AnnotationStore::open(root.path(), "paper.pdf")
                .err()
                .unwrap_or_else(|| panic!("{text} must be refused"));
            assert!(
                matches!(error, StoreError::Corrupt(_)),
                "a data problem, not an IO one: {error}"
            );
        }
    }

    #[test]
    fn a_hand_written_file_loads_and_gets_orderly_ids() {
        let root = vault();
        let path = sidecar_path(root.path(), "paper.pdf").unwrap();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            r#"{"version": 1, "annotations": [
                {"page": 0, "kind": "highlight", "rect": {"x0": 4, "y0": 3, "x1": 2, "y1": 9}},
                {"page": 0, "kind": "text", "rect": {"x0": 0, "y0": 0, "x1": 1, "y1": 1}, "contents": "hi"}
            ]}"#,
        )
        .unwrap();
        let mut store = AnnotationStore::open(root.path(), "paper.pdf").expect("opens");
        let items: Vec<&Annotation> = store.annotations().iter().collect();
        assert_eq!(items.len(), 2);
        // Corners swapped in the file come back ordered.
        assert_eq!(items[0].rect, rect(2.0, 3.0, 4.0, 9.0));
        let next = store
            .add(0, Kind::Highlight, rect(0.0, 0.0, 1.0, 1.0), None)
            .unwrap();
        assert_eq!(next.as_u64(), 3, "ids continue past the hand-written two");
    }

    #[test]
    fn an_unusable_name_is_a_data_problem() {
        let root = vault();
        let error = AnnotationStore::open(root.path(), "../escape.pdf")
            .err()
            .expect("must be refused");
        assert!(matches!(error, StoreError::Corrupt(_)), "{error}");
    }
}
