//! Safe extraction of downloaded archives. Archive contents are untrusted: every entry path is
//! validated so nothing can be written outside the destination folder.

use std::io::Read;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Default)]
pub struct Extracted {
    /// Commit id from the pax global header that GitHub (via `git archive`) writes.
    pub commit: Option<String>,
    pub files: usize,
    pub skipped_links: usize,
}

/// Extracts a `.tar.gz`, dropping the first path component (GitHub's `repo-<sha>/` folder)
/// when `strip_top` is set.
pub fn extract_tar_gz(archive: &Path, dest: &Path, strip_top: bool) -> std::io::Result<Extracted> {
    let file = std::fs::File::open(archive)?;
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(file));
    std::fs::create_dir_all(dest)?;
    let mut out = Extracted::default();

    for entry in tar.entries()? {
        let mut entry = entry?;
        let kind = entry.header().entry_type();
        if kind == tar::EntryType::XGlobalHeader {
            let mut body = String::new();
            entry.read_to_string(&mut body)?;
            out.commit = pax_comment(&body);
            continue;
        }
        let raw = entry.path()?.into_owned();
        let Some(rel) = safe_relative(&raw, strip_top) else {
            continue;
        };
        let target = dest.join(&rel);

        match kind {
            tar::EntryType::Directory => std::fs::create_dir_all(&target)?,
            tar::EntryType::Regular | tar::EntryType::Continuous => {
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                entry.unpack(&target)?;
                out.files += 1;
            }
            tar::EntryType::Symlink => {
                let link = entry.link_name()?.map(|l| l.into_owned());
                if !create_contained_symlink(dest, &rel, link.as_deref())? {
                    out.skipped_links += 1;
                }
            }
            // Hard links, devices and FIFOs have no place in a source tree.
            _ => out.skipped_links += 1,
        }
    }
    Ok(out)
}

/// Extracts one file (by its name, at any depth) from a `.tar.gz` or `.zip` runtime archive.
pub fn extract_single(archive: &Path, wanted: &str, dest_file: &Path) -> std::io::Result<()> {
    if let Some(parent) = dest_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let name = archive
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    if name.ends_with(".zip") {
        let mut zip = zip::ZipArchive::new(std::fs::File::open(archive)?)?;
        for i in 0..zip.len() {
            let mut f = zip.by_index(i)?;
            let name = f.name().map(|n| n.into_owned()).unwrap_or_default();
            if Path::new(&name).file_name().and_then(|n| n.to_str()) == Some(wanted) {
                let mut out = std::fs::File::create(dest_file)?;
                std::io::copy(&mut f, &mut out)?;
                return finish_executable(dest_file);
            }
        }
    } else {
        let file = std::fs::File::open(archive)?;
        let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(file));
        for entry in tar.entries()? {
            let mut entry = entry?;
            let matches = entry.path()?.file_name().and_then(|n| n.to_str()) == Some(wanted);
            if matches && entry.header().entry_type().is_file() {
                entry.unpack(dest_file)?;
                return finish_executable(dest_file);
            }
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("{wanted} not in {name}"),
    ))
}

fn finish_executable(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    }
    let _ = path;
    Ok(())
}

fn pax_comment(body: &str) -> Option<String> {
    // Records look like "52 comment=<40 hex>\n".
    body.lines().find_map(|line| {
        let value = line.split_once(' ')?.1.strip_prefix("comment=")?;
        (value.len() == 40 && value.chars().all(|c| c.is_ascii_hexdigit()))
            .then(|| value.to_owned())
    })
}

/// Returns a path made only of normal components, or `None` if the entry must be skipped.
pub(crate) fn safe_relative(raw: &Path, strip_top: bool) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    let mut comps = raw.components().peekable();
    if strip_top {
        match comps.next()? {
            Component::Normal(_) => {}
            _ => return None,
        }
    }
    for c in comps {
        match c {
            Component::Normal(part) => out.push(part),
            Component::CurDir => {}
            _ => return None,
        }
    }
    (!out.as_os_str().is_empty()).then_some(out)
}

/// Creates a relative symlink only if its target stays inside `root`. Windows needs special
/// privileges for symlinks, so they are skipped there.
fn create_contained_symlink(root: &Path, rel: &Path, link: Option<&Path>) -> std::io::Result<bool> {
    let Some(link) = link else { return Ok(false) };
    if link.is_absolute() {
        return Ok(false);
    }
    let mut depth: i32 = rel.components().count() as i32 - 1;
    for c in link.components() {
        match c {
            Component::ParentDir => depth -= 1,
            Component::Normal(_) => depth += 1,
            Component::CurDir => {}
            _ => return Ok(false),
        }
        if depth < 0 {
            return Ok(false);
        }
    }
    #[cfg(unix)]
    {
        let target = root.join(rel);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let _ = std::fs::remove_file(&target);
        std::os::unix::fs::symlink(link, &target)?;
        Ok(true)
    }
    #[cfg(not(unix))]
    {
        let _ = root;
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_escaping_paths() {
        assert_eq!(
            safe_relative(Path::new("top/a/b.txt"), true),
            Some(PathBuf::from("a/b.txt"))
        );
        assert_eq!(safe_relative(Path::new("top/"), true), None);
        assert_eq!(safe_relative(Path::new("top/../etc/passwd"), true), None);
        assert_eq!(safe_relative(Path::new("/etc/passwd"), false), None);
        assert_eq!(
            safe_relative(Path::new("a/./b"), false),
            Some(PathBuf::from("a/b"))
        );
    }

    #[test]
    fn extracts_and_reads_commit() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("a.tar.gz");
        let sha = "7fd1a60b01f91b314f59955a4e4d4e80d8edf11d";
        {
            let gz = flate2::write::GzEncoder::new(
                std::fs::File::create(&archive).unwrap(),
                flate2::Compression::fast(),
            );
            let mut b = tar::Builder::new(gz);
            let pax = format!("52 comment={sha}\n");
            let mut h = tar::Header::new_ustar();
            h.set_entry_type(tar::EntryType::XGlobalHeader);
            h.set_size(pax.len() as u64);
            h.set_cksum();
            b.append_data(&mut h, "pax_global_header", pax.as_bytes())
                .unwrap();
            let body = b"print('hi')\n";
            let mut h = tar::Header::new_gnu();
            h.set_size(body.len() as u64);
            h.set_mode(0o644);
            h.set_cksum();
            b.append_data(&mut h, "repo-x/app.py", &body[..]).unwrap();
            // A symlink that escapes the tree must be skipped.
            let mut h = tar::Header::new_gnu();
            h.set_entry_type(tar::EntryType::Symlink);
            h.set_size(0);
            h.set_cksum();
            b.append_link(&mut h, "repo-x/evil", "../../etc/passwd")
                .unwrap();
            b.into_inner().unwrap().finish().unwrap();
        }
        let dest = dir.path().join("out");
        let ex = extract_tar_gz(&archive, &dest, true).unwrap();
        assert_eq!(ex.commit.as_deref(), Some(sha));
        assert_eq!(ex.files, 1);
        assert_eq!(ex.skipped_links, 1);
        assert_eq!(
            std::fs::read_to_string(dest.join("app.py")).unwrap(),
            "print('hi')\n"
        );
        assert!(!dest.join("evil").exists());
    }
}
