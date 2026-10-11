//! A project's own icon: the file its `launcher.json` names, or a logo or favicon found in the
//! usual places. It is copied into the project's `meta` folder when the source arrives, shown on
//! the project's card, and, when it's a square PNG, used for its shortcuts too (wrapped as
//! `.icns` / `.ico` without re-encoding the image). Projects without one keep the owner's
//! avatar or a monogram on the card and the Launcher's icon on shortcuts.

use crate::detect::FileTree;
use base64::Engine as _;
use std::path::{Path, PathBuf};

/// Larger files are not icons (or not worth putting in every project list).
const MAX_BYTES: usize = 256 * 1024;

/// Where projects usually keep a logo, best first. Matched case-insensitively.
const CANDIDATES: &[&str] = &[
    "icon.png",
    "logo.png",
    "app-icon.png",
    "assets/icon.png",
    "assets/logo.png",
    "public/icon.png",
    "public/logo.png",
    "public/apple-touch-icon.png",
    "static/icon.png",
    "static/logo.png",
    ".github/logo.png",
    "docs/logo.png",
    "images/logo.png",
    "img/logo.png",
    "logo.svg",
    "assets/logo.svg",
    "public/logo.svg",
    "public/favicon.svg",
    "public/favicon.png",
    "public/favicon.ico",
    "static/favicon.svg",
    "static/favicon.png",
    "static/favicon.ico",
    "favicon.svg",
    "favicon.png",
    "favicon.ico",
    "public/vite.svg",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Png,
    Svg,
    Ico,
    Jpeg,
    Webp,
}

impl Format {
    fn sniff(bytes: &[u8]) -> Option<Self> {
        if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            Some(Self::Png)
        } else if bytes.starts_with(&[0, 0, 1, 0]) {
            Some(Self::Ico)
        } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
            Some(Self::Jpeg)
        } else if bytes.len() > 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
            Some(Self::Webp)
        } else if std::str::from_utf8(&bytes[..bytes.len().min(2048)])
            .is_ok_and(|head| head.contains("<svg"))
        {
            Some(Self::Svg)
        } else {
            None
        }
    }

    fn ext(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Svg => "svg",
            Self::Ico => "ico",
            Self::Jpeg => "jpg",
            Self::Webp => "webp",
        }
    }

    fn mime(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Svg => "image/svg+xml",
            Self::Ico => "image/x-icon",
            Self::Jpeg => "image/jpeg",
            Self::Webp => "image/webp",
        }
    }

    const ALL: [Self; 5] = [Self::Png, Self::Svg, Self::Ico, Self::Jpeg, Self::Webp];
}

/// The project's icon file in its source tree: the declared one if it is usable, otherwise
/// the first usual location that holds a real image.
pub fn find(tree: &FileTree, declared: Option<&str>) -> Option<(String, Vec<u8>, Format)> {
    let usable = |rel: &str| -> Option<(Vec<u8>, Format)> {
        if rel.starts_with('/') || rel.split(['/', '\\']).any(|part| part == "..") {
            return None;
        }
        // Never through a link: it could point anywhere on the computer.
        let path = tree.root().join(rel);
        if !std::fs::symlink_metadata(&path).ok()?.is_file() {
            return None;
        }
        let bytes = std::fs::read(path).ok()?;
        if bytes.len() > MAX_BYTES {
            return None;
        }
        Format::sniff(&bytes).map(|f| (bytes, f))
    };
    if let Some(rel) = declared.map(|d| d.trim_start_matches("./"))
        && tree.has(rel)
        && let Some((bytes, format)) = usable(rel)
    {
        return Some((rel.to_owned(), bytes, format));
    }
    let files: Vec<&str> = tree.files().collect();
    CANDIDATES.iter().find_map(|want| {
        let rel = files.iter().find(|f| f.eq_ignore_ascii_case(want))?;
        usable(rel).map(|(bytes, format)| ((*rel).to_owned(), bytes, format))
    })
}

/// Saves the icon as `meta/icon.<ext>`, replacing any earlier one; `None` removes it.
pub fn save(meta: &Path, icon: Option<(&[u8], Format)>) -> std::io::Result<()> {
    for f in Format::ALL {
        let old = meta.join(format!("icon.{}", f.ext()));
        if icon.is_none_or(|(_, new)| new != f) && old.exists() {
            std::fs::remove_file(old)?;
        }
    }
    if let Some((bytes, format)) = icon {
        std::fs::create_dir_all(meta)?;
        let path = meta.join(format!("icon.{}", format.ext()));
        if std::fs::read(&path).ok().as_deref() != Some(bytes) {
            std::fs::write(path, bytes)?;
        }
    }
    Ok(())
}

/// The saved icon, if any.
pub fn load(meta: &Path) -> Option<(Vec<u8>, Format)> {
    Format::ALL.iter().find_map(|&f| {
        let bytes = std::fs::read(meta.join(format!("icon.{}", f.ext()))).ok()?;
        (Format::sniff(&bytes) == Some(f)).then_some((bytes, f))
    })
}

pub fn data_url(bytes: &[u8], format: Format) -> String {
    format!(
        "data:{};base64,{}",
        format.mime(),
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

/// Width and height from a PNG's header.
fn png_size(png: &[u8]) -> Option<(u32, u32)> {
    if png.len() < 24 || Format::sniff(png) != Some(Format::Png) || &png[12..16] != b"IHDR" {
        return None;
    }
    let num = |at: usize| u32::from_be_bytes(png[at..at + 4].try_into().unwrap_or_default());
    Some((num(16), num(20)))
}

/// A square PNG wrapped as a one-image `.icns` (macOS reads PNG data in these slots), if its
/// size has a slot.
pub fn icns_from_png(png: &[u8]) -> Option<Vec<u8>> {
    let (w, h) = png_size(png)?;
    let kind: &[u8; 4] = match (w == h).then_some(w)? {
        16 => b"icp4",
        32 => b"icp5",
        64 => b"icp6",
        128 => b"ic07",
        256 => b"ic08",
        512 => b"ic09",
        1024 => b"ic10",
        _ => return None,
    };
    let entry_len = u32::try_from(8 + png.len()).ok()?;
    let mut out = Vec::with_capacity(png.len() + 16);
    out.extend_from_slice(b"icns");
    out.extend_from_slice(&(entry_len + 8).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(&entry_len.to_be_bytes());
    out.extend_from_slice(png);
    Some(out)
}

/// A square PNG of at most 256 pixels wrapped as a one-image `.ico` (Windows reads PNG data
/// in icon files).
pub fn ico_from_png(png: &[u8]) -> Option<Vec<u8>> {
    let (w, h) = png_size(png)?;
    if w != h || !(16..=256).contains(&w) {
        return None;
    }
    let side = if w == 256 { 0 } else { w as u8 };
    let len = u32::try_from(png.len()).ok()?;
    let mut out = Vec::with_capacity(png.len() + 22);
    out.extend_from_slice(&[0, 0, 1, 0, 1, 0]);
    out.extend_from_slice(&[side, side, 0, 0, 1, 0, 32, 0]);
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&22u32.to_le_bytes());
    out.extend_from_slice(png);
    Some(out)
}

/// The per-project icon file for shortcuts (Linux `.desktop` files and Windows `.lnk` files
/// point at a file).
pub fn shortcut_icon_path(icons: &Path, id: &str, ext: &str) -> PathBuf {
    icons.join(format!("{id}.{ext}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal PNG header for a `side`×`side` image (enough for the size checks).
    fn png(side: u32) -> Vec<u8> {
        let mut b = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
        b.extend_from_slice(&side.to_be_bytes());
        b.extend_from_slice(&side.to_be_bytes());
        b.extend_from_slice(&[8, 6, 0, 0, 0]);
        b
    }

    fn tree(files: &[(&str, &[u8])]) -> (tempfile::TempDir, FileTree) {
        let dir = tempfile::tempdir().unwrap();
        for (name, bytes) in files {
            let path = dir.path().join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
        }
        let t = FileTree::scan(dir.path()).unwrap();
        (dir, t)
    }

    #[test]
    fn finds_declared_then_usual_icons() {
        let logo = png(128);
        let (_d, t) = tree(&[
            (
                "Public/Favicon.svg",
                b"<svg xmlns='http://www.w3.org/2000/svg'/>",
            ),
            ("art/mark.png", &logo),
            ("README.md", b"# hi"),
        ]);
        let (rel, _, f) = find(&t, Some("./art/mark.png")).unwrap();
        assert_eq!((rel.as_str(), f), ("art/mark.png", Format::Png));
        let (rel, _, f) = find(&t, Some("../outside.png")).unwrap();
        assert_eq!((rel.as_str(), f), ("Public/Favicon.svg", Format::Svg));
        let (_d, t) = tree(&[("logo.png", b"not an image")]);
        assert!(find(&t, None).is_none());
    }

    #[test]
    fn wraps_square_pngs_for_shortcuts() {
        let icns = icns_from_png(&png(256)).unwrap();
        assert_eq!(&icns[..4], b"icns");
        assert_eq!(
            u32::from_be_bytes(icns[4..8].try_into().unwrap()) as usize,
            icns.len()
        );
        assert_eq!(&icns[8..12], b"ic08");
        assert!(icns_from_png(&png(200)).is_none());

        let ico = ico_from_png(&png(256)).unwrap();
        assert_eq!(&ico[..6], &[0, 0, 1, 0, 1, 0]);
        assert_eq!(ico[6], 0, "256 is written as 0");
        assert_eq!(&ico[22..26], b"\x89PNG");
        assert!(ico_from_png(&png(512)).is_none());
    }

    #[test]
    fn saves_one_icon_at_a_time() {
        let dir = tempfile::tempdir().unwrap();
        save(dir.path(), Some((&png(64), Format::Png))).unwrap();
        save(dir.path(), Some((b"<svg/>", Format::Svg))).unwrap();
        assert!(!dir.path().join("icon.png").exists());
        assert_eq!(load(dir.path()).unwrap().1, Format::Svg);
        save(dir.path(), None).unwrap();
        assert!(load(dir.path()).is_none());
        assert!(data_url(b"<svg/>", Format::Svg).starts_with("data:image/svg+xml;base64,"));
    }
}
