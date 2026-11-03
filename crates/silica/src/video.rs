//! Local video in Silica's own player: `ffmpeg` decodes, gpui draws.
//!
//! There is no video decoder in the dependency tree and no intention of
//! vendoring one, so the host's `ffmpeg` does the decoding. It is spawned
//! once per play with `-re`, which paces it at the file's own frame rate,
//! and it writes raw RGBA frames to its stdout; this module reads one frame
//! at a time, so the pipe's backpressure keeps the decode from running
//! ahead. Nothing blocks the UI thread: the read happens on the background
//! executor and the frame is installed when it lands.
//!
//! `ffprobe` answers size and duration once, on open. A machine without
//! ffmpeg gets `None` from [`probe`] and the embed falls back to opening the
//! file in the system player, which is what every embed did before.
//!
//! Frames are drawn as a gpui texture per frame, the previous one dropped
//! with [`gpui::App::drop_image`]. Audio is not decoded — a video plays
//! silent here.

use gpui::{App, ImageSource, RenderImage};
use image::{Frame, RgbaImage};
use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;

/// The widest the player renders, in logical pixels. A frame is decoded at
/// its display size, so a 4K file does not cost 4K of memory per frame.
pub const MAX_WIDTH: u32 = 720;

/// What `ffprobe` says about a file.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Info {
    /// The frame's own size, before any scaling.
    pub width: u32,
    pub height: u32,
    pub fps: f32,
    pub duration: f32,
}

impl Info {
    /// The size frames are decoded at: the file's aspect, no wider than
    /// [`MAX_WIDTH`], both sides even (H.264's chroma wants that).
    pub fn decode_size(&self) -> (u32, u32) {
        let width = self.width.clamp(2, MAX_WIDTH);
        let height = ((width as f32) * (self.height as f32) / (self.width.max(1) as f32)).round();
        let even = |n: u32| (n.max(2) / 2) * 2;
        (even(width), even(height as u32))
    }

    /// The display size, in logical pixels, for an available width.
    pub fn display_size(&self, available: f32) -> (f32, f32) {
        let (w, h) = self.decode_size();
        let scale = (available.max(80.0) / w as f32).min(1.0);
        (w as f32 * scale, h as f32 * scale)
    }
}

/// Whether `ffmpeg` and `ffprobe` are both on the path, checked once.
pub fn available() -> bool {
    use std::sync::OnceLock;
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        ["ffmpeg", "ffprobe"].iter().all(|name| {
            Command::new(name)
                .arg("-version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|s| s.success())
        })
    })
}

/// Reads a file's size, frame rate and duration. `None` when ffprobe is
/// missing, the file will not open, or it carries no video stream.
pub fn probe(path: &Path) -> Option<Info> {
    if !available() {
        return None;
    }
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height,avg_frame_rate",
            "-show_entries",
            "format=duration",
            "-of",
            "json",
        ])
        .arg(path)
        .stdin(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    let stream = json.get("streams")?.get(0)?;
    let width = stream.get("width")?.as_u64()? as u32;
    let height = stream.get("height")?.as_u64()? as u32;
    if width == 0 || height == 0 {
        return None;
    }
    // * "30000/1001" or "0/0" for a still.
    let fps = stream
        .get("avg_frame_rate")
        .and_then(|v| v.as_str())
        .and_then(|s| {
            let (num, den) = s.split_once('/')?;
            let num: f32 = num.parse().ok()?;
            let den: f32 = den.parse().ok()?;
            (den > 0.0 && num > 0.0).then(|| num / den)
        })
        .filter(|f| f.is_finite() && *f > 0.0)
        .unwrap_or(25.0);
    let duration = json
        .get("format")
        .and_then(|f| f.get("duration"))
        .and_then(|d| d.as_str())
        .and_then(|s| s.parse::<f32>().ok())
        .filter(|d| d.is_finite() && *d > 0.0)
        .unwrap_or(0.0);
    Some(Info {
        width,
        height,
        fps,
        duration,
    })
}

/// The decoder for one play, from `start` seconds. Dropping it kills ffmpeg.
pub struct Decoder {
    child: Child,
    size: (u32, u32),
    buffer: Vec<u8>,
}

impl Decoder {
    /// Spawns `ffmpeg` at `start` seconds, decoding at the file's display
    /// size. `-re` paces it at real time; `-an` drops the audio, which this
    /// player does not decode.
    pub fn spawn(path: &Path, info: &Info, start: f32) -> Option<Decoder> {
        let (width, height) = info.decode_size();
        let mut command = Command::new("ffmpeg");
        command.args(["-v", "error", "-nostdin"]);
        if start > 0.0 {
            command.args(["-ss", &format!("{start:.3}")]);
        }
        command
            .args(["-re", "-i"])
            .arg(path)
            .args(["-an", "-f", "rawvideo", "-pix_fmt", "rgba", "-s"])
            .arg(format!("{width}x{height}"))
            .arg("-")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let child = command.spawn().ok()?;
        Some(Decoder {
            child,
            size: (width, height),
            buffer: vec![0; (width * height * 4) as usize],
        })
    }

    /// The next frame as a texture, or `None` at the end of the file or when
    /// ffmpeg dies. Blocks until the frame is decoded — call it on the
    /// background executor.
    pub fn next_frame(&mut self) -> Option<Arc<RenderImage>> {
        let stdout = self.child.stdout.as_mut()?;
        stdout.read_exact(&mut self.buffer).ok()?;
        Some(to_image(&self.buffer, self.size))
    }

    /// The frame at `seconds`, decoded on its own: the still shown before
    /// play, while paused, and after a seek.
    pub fn still(path: &Path, info: &Info, seconds: f32) -> Option<Arc<RenderImage>> {
        if !available() {
            return None;
        }
        let (width, height) = info.decode_size();
        let out = Command::new("ffmpeg")
            .args(["-v", "error", "-nostdin"])
            .args(["-ss", &format!("{seconds:.3}")])
            .arg("-i")
            .arg(path)
            .args(["-frames:v", "1", "-an", "-f", "rawvideo", "-pix_fmt", "rgba", "-s"])
            .arg(format!("{width}x{height}"))
            .arg("-")
            .stdin(Stdio::null())
            .output()
            .ok()?;
        if !out.status.success() || out.stdout.len() != (width * height * 4) as usize {
            return None;
        }
        Some(to_image(&out.stdout, (width, height)))
    }

    /// Stops ffmpeg now; dropping the handle reaps it.
    pub fn stop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// RGBA bytes as gpui's texture: `RenderImage` is BGRA, so red and blue
/// swap. A video frame is opaque, so the swap is the whole conversion.
fn to_image(rgba: &[u8], (width, height): (u32, u32)) -> Arc<RenderImage> {
    let mut bytes = rgba.to_vec();
    for pixel in bytes.chunks_exact_mut(4) {
        pixel.swap(0, 2);
        pixel[3] = 255;
    }
    let buffer = RgbaImage::from_raw(width, height, bytes).expect("buffer is sized to the frame");
    Arc::new(RenderImage::new(vec![Frame::new(buffer)]))
}

/// The frame at `seconds`, on its own: the still shown before play, while
/// paused, and after a seek. `None` without ffmpeg or when the read fails.
pub fn still(path: &Path, info: &Info, seconds: f32) -> Option<Arc<RenderImage>> {
    Decoder::still(path, info, seconds)
}

/// An [`ImageSource`] for a frame already decoded.
pub fn frame_source(frame: Arc<RenderImage>) -> ImageSource {
    ImageSource::Render(frame)
}

/// Replacing a frame drops the previous texture; gpui keeps it until told.
pub fn drop_frame(previous: Option<Arc<RenderImage>>, cx: &mut App) {
    if let Some(image) = previous {
        cx.drop_image(image, None);
    }
}

/// A path's extension is one of the ones a player can open.
#[allow(dead_code)]
pub fn is_video(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| silica_formats::VIDEO_EXTENSIONS.iter().any(|v| v.eq_ignore_ascii_case(ext)))
}


#[cfg(test)]
mod tests {
    use super::*;

    fn info(width: u32, height: u32) -> Info {
        Info {
            width,
            height,
            fps: 25.0,
            duration: 3.0,
        }
    }

    #[test]
    fn decode_size_caps_the_width_and_keeps_the_aspect() {
        // * 1080 * 720/1920 = 405, then even: 404. The aspect is kept to a
        //   pixel, which is what the decode is for.
        assert_eq!(info(1920, 1080).decode_size(), (720, 404));
        assert_eq!(info(3840, 2160).decode_size(), (720, 404));
        assert_eq!(info(320, 180).decode_size(), (320, 180));
    }

    #[test]
    fn display_size_fits_the_available_width() {
        let (w, h) = info(1920, 1080).display_size(360.0);
        assert!((w - 360.0).abs() < 0.01, "{w}");
        // * The frame is 720 by 404, so half of it is 202.
        assert!((h - 202.0).abs() < 1.0, "{h}");
    }

    #[test]
    fn a_narrow_available_width_shrinks_the_frame() {
        let (w, _) = info(320, 180).display_size(160.0);
        assert!((w - 160.0).abs() < 0.01, "{w}");
    }

    #[test]
    fn a_frame_swaps_to_bgra_and_pins_the_alpha() {
        let image = to_image(&[10, 20, 30, 0], (1, 1));
        assert_eq!(image.as_bytes(0), Some(&[30, 20, 10, 255][..]));
    }

    #[test]
    fn video_extensions_are_recognised() {
        assert!(is_video(Path::new("/v/a.mp4")));
        assert!(is_video(Path::new("/v/a.MKV")));
        assert!(!is_video(Path::new("/v/a.mp3")));
    }
}
