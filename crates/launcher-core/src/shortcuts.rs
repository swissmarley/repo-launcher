//! Shortcuts that open a project with one click, without opening Repo Launcher's window:
//! a Start menu and desktop `.lnk` on Windows, an applications-menu `.desktop` file on Linux,
//! a tiny app in ~/Applications on macOS (DECISIONS D-012: we never edit the Dock).
//!
//! Windows and Linux shortcuts start the Launcher with `--launch <id>`. The macOS app runs
//! `open repolauncher://launch/<id>`, so it keeps working when the Launcher is moved.

use crate::store::ShortcutKind;
use std::io;
use std::path::{Path, PathBuf};

/// What shortcuts start: the Launcher itself, plus its icon for the shortcut files.
#[derive(Debug, Clone, Default)]
pub struct Launcher {
    pub exe: PathBuf,
    pub icon_png: Option<Vec<u8>>,
    pub icon_icns: Option<Vec<u8>>,
}

impl Launcher {
    /// The running Launcher. An AppImage runs from a temporary mount, so its shortcuts must
    /// point at the AppImage file itself.
    pub fn current() -> io::Result<Self> {
        let exe = match std::env::var_os("APPIMAGE") {
            Some(image) if cfg!(target_os = "linux") => PathBuf::from(image),
            _ => std::env::current_exe()?,
        };
        Ok(Self {
            exe,
            ..Self::default()
        })
    }

    /// What a shortcut opens; stored so shortcuts can be repaired when it changes.
    pub fn target(&self, id: &str) -> String {
        if cfg!(target_os = "macos") {
            deep_link(id)
        } else {
            self.exe.to_string_lossy().into_owned()
        }
    }
}

pub fn deep_link(id: &str) -> String {
    format!("repolauncher://launch/{id}")
}

/// Where shortcuts go. Tests point these at a temporary folder.
#[derive(Debug, Clone)]
pub struct ShortcutDirs {
    /// macOS ~/Applications, Linux's applications folder, or our Start menu folder on Windows.
    pub menu: PathBuf,
    /// The desktop, where we put shortcuts too (Windows only).
    pub desktop: Option<PathBuf>,
}

impl ShortcutDirs {
    pub fn for_current_user() -> Self {
        let home = dirs::home_dir().unwrap_or_default();
        if cfg!(target_os = "macos") {
            Self {
                menu: home.join("Applications"),
                desktop: None,
            }
        } else if cfg!(windows) {
            let programs = dirs::data_dir()
                .unwrap_or_default()
                .join(r"Microsoft\Windows\Start Menu\Programs\Repo Launcher");
            Self {
                menu: programs,
                desktop: dirs::desktop_dir(),
            }
        } else {
            Self {
                menu: dirs::data_dir()
                    .unwrap_or_else(|| home.join(".local/share"))
                    .join("applications"),
                desktop: None,
            }
        }
    }

    pub fn at(root: &Path) -> Self {
        Self {
            menu: root.join("menu"),
            desktop: cfg!(windows).then(|| root.join("desktop")),
        }
    }
}

/// The shortcut files a project gets on this OS. A path that already holds something other
/// than this project's own shortcut (`taken`) is never overwritten; a numbered name is used.
pub fn plan(
    dirs: &ShortcutDirs,
    name: &str,
    id: &str,
    taken: impl Fn(&Path) -> bool,
) -> Vec<(ShortcutKind, PathBuf)> {
    let base = file_name(name);
    let free = |dir: &Path, ext: &str| {
        (1..10)
            .map(|n| match n {
                1 => format!("{base}.{ext}"),
                n => format!("{base} ({n}).{ext}"),
            })
            .chain([format!("{base} ({id}).{ext}")])
            .map(|f| dir.join(f))
            .find(|p| !taken(p))
            .unwrap_or_else(|| dir.join(format!("{base} ({id}).{ext}")))
    };
    if cfg!(target_os = "macos") {
        vec![(ShortcutKind::Applications, free(&dirs.menu, "app"))]
    } else if cfg!(windows) {
        let mut out = vec![(ShortcutKind::StartMenu, free(&dirs.menu, "lnk"))];
        if let Some(desktop) = &dirs.desktop {
            out.push((ShortcutKind::Desktop, free(desktop, "lnk")));
        }
        out
    } else {
        // Linux menus show the Name inside the file, so the file name only has to be unique.
        vec![(
            ShortcutKind::AppMenu,
            dirs.menu.join(format!("repolauncher-{id}.desktop")),
        )]
    }
}

/// Writes (or rewrites) one shortcut file.
pub fn write(
    path: &Path,
    name: &str,
    id: &str,
    launcher: &Launcher,
    icons: &Path,
) -> io::Result<()> {
    if !id.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return Err(io::Error::other("unexpected project id"));
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    #[cfg(target_os = "macos")]
    {
        let _ = icons;
        write_mac_app(path, name, id, launcher)
    }
    #[cfg(windows)]
    {
        let _ = icons;
        windows_lnk::write(
            path,
            &launcher.exe,
            &format!("--launch {id}"),
            &format!("{name} (Repo Launcher)"),
        )
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let icon = match &launcher.icon_png {
            Some(png) => {
                std::fs::create_dir_all(icons)?;
                let file = icons.join("repo-launcher.png");
                if std::fs::read(&file).ok().as_deref() != Some(png.as_slice()) {
                    std::fs::write(&file, png)?;
                }
                file.to_string_lossy().into_owned()
            }
            None => "repo-launcher".to_owned(),
        };
        write_desktop_entry(path, name, id, &launcher.exe, &icon)
    }
}

/// Removes a shortcut; one that is already gone is fine.
pub fn remove(path: &Path) -> io::Result<()> {
    let result = if path.extension().is_some_and(|e| e == "app") && path.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    };
    match result {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
        _ => {
            // Our own Start menu folder goes away with its last shortcut.
            if cfg!(windows)
                && let Some(dir) = path.parent()
                && dir.file_name().is_some_and(|n| n == "Repo Launcher")
            {
                let _ = std::fs::remove_dir(dir);
            }
            Ok(())
        }
    }
}

/// A file name for the shortcut from the project's display name.
fn file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            c if c.is_control() => '-',
            c => c,
        })
        .take(60)
        .collect();
    let cleaned = cleaned.trim_matches(|c: char| c == '.' || c.is_whitespace());
    if cleaned.is_empty() {
        "Project".to_owned()
    } else {
        cleaned.to_owned()
    }
}

#[cfg(target_os = "macos")]
fn write_mac_app(path: &Path, name: &str, id: &str, launcher: &Launcher) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    // Built beside the final location, then swapped in, so a half-written app never shows.
    let staging = path.with_extension(format!("app.incoming-{id}"));
    let _ = std::fs::remove_dir_all(&staging);
    let contents = staging.join("Contents");
    std::fs::create_dir_all(contents.join("MacOS"))?;
    std::fs::create_dir_all(contents.join("Resources"))?;
    let icon = match &launcher.icon_icns {
        Some(icns) => {
            std::fs::write(contents.join("Resources").join("icon.icns"), icns)?;
            "<key>CFBundleIconFile</key><string>icon</string>"
        }
        None => "",
    };
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key><string>launch</string>
  <key>CFBundleIdentifier</key><string>app.repolauncher.desktop.shortcut.{id}</string>
  <key>CFBundleName</key><string>{name}</string>
  <key>CFBundleDisplayName</key><string>{name}</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>1.0</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>LSMinimumSystemVersion</key><string>10.15</string>
  <key>LSUIElement</key><true/>
  {icon}
</dict>
</plist>
"#,
        name = xml_escape(name),
    );
    std::fs::write(contents.join("Info.plist"), plist)?;
    let script = contents.join("MacOS").join("launch");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\n# Opens this project with Repo Launcher.\nexec /usr/bin/open \"{}\"\n",
            deep_link(id)
        ),
    )?;
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))?;
    remove(path)?;
    std::fs::rename(&staging, path)
}

#[cfg(target_os = "macos")]
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(all(unix, not(target_os = "macos")))]
fn write_desktop_entry(
    path: &Path,
    name: &str,
    id: &str,
    exe: &Path,
    icon: &str,
) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let text = format!(
        "[Desktop Entry]\nType=Application\nVersion=1.5\nName={name}\nComment=Opens with Repo Launcher\n\
         Exec={exec} --launch {id}\nIcon={icon}\nTerminal=false\nCategories=Development;\n\
         StartupNotify=false\nX-RepoLauncher-Project={id}\n",
        name = desktop_string(name),
        exec = desktop_string(&exec_arg(&exe.to_string_lossy())),
        icon = desktop_string(icon),
    );
    let staging = path.with_extension("desktop.incoming");
    std::fs::write(&staging, text)?;
    std::fs::set_permissions(&staging, std::fs::Permissions::from_mode(0o755))?;
    std::fs::rename(&staging, path)
}

/// One argument of an `Exec=` line, quoted per the Desktop Entry spec.
#[cfg(all(unix, not(target_os = "macos")))]
fn exec_arg(arg: &str) -> String {
    let arg = arg.replace('%', "%%");
    let reserved = |c: char| " \t\n\"'\\><~|&;$*?#()`".contains(c);
    if !arg.chars().any(reserved) {
        return arg;
    }
    let mut out = String::from('"');
    for c in arg.chars() {
        if matches!(c, '"' | '`' | '$' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

/// A Desktop Entry string value (applied after `exec_arg`'s quoting, as the spec says).
#[cfg(all(unix, not(target_os = "macos")))]
fn desktop_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
        .replace('\r', "\\r")
}

#[cfg(windows)]
pub mod windows_lnk {
    //! `.lnk` files through the Shell's own `IShellLinkW` (Microsoft's `windows` crate).

    use std::io;
    use std::path::Path;
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
        CoUninitialize, IPersistFile, STGM_READ,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
    use windows::core::{HSTRING, Interface};

    /// Runs `f` on a fresh thread with COM initialized, whatever the caller's thread is.
    fn with_com<T: Send>(f: impl FnOnce() -> windows::core::Result<T> + Send) -> io::Result<T> {
        std::thread::scope(|s| {
            s.spawn(|| {
                // SAFETY: COM is initialized and uninitialized on this thread only.
                unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.ok()?;
                let out = f();
                unsafe { CoUninitialize() };
                out
            })
            .join()
            .map_err(|_| windows::core::Error::empty())
        })
        .and_then(|r| r)
        .map_err(|e| io::Error::other(e.to_string()))
    }

    pub fn write(path: &Path, exe: &Path, args: &str, description: &str) -> io::Result<()> {
        with_com(|| unsafe {
            let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
            link.SetPath(&HSTRING::from(exe))?;
            link.SetArguments(&HSTRING::from(args))?;
            link.SetDescription(&HSTRING::from(description))?;
            link.SetIconLocation(&HSTRING::from(exe), 0)?;
            if let Some(dir) = exe.parent() {
                link.SetWorkingDirectory(&HSTRING::from(dir))?;
            }
            link.cast::<IPersistFile>()?
                .Save(&HSTRING::from(path), true)
        })
    }

    /// The program and arguments a `.lnk` file starts (for tests and repair).
    pub fn read(path: &Path) -> io::Result<(String, String)> {
        with_com(|| unsafe {
            let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
            link.cast::<IPersistFile>()?
                .Load(&HSTRING::from(path), STGM_READ)?;
            let mut target = [0u16; 1024];
            link.GetPath(&mut target, std::ptr::null_mut(), 0)?;
            let mut args = [0u16; 1024];
            link.GetArguments(&mut args)?;
            let text = |b: &[u16]| {
                String::from_utf16_lossy(&b[..b.iter().position(|&c| c == 0).unwrap_or(b.len())])
            };
            Ok((text(&target), text(&args)))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn launcher(root: &Path) -> Launcher {
        Launcher {
            exe: root.join("Repo Launcher").join(if cfg!(windows) {
                "repo-launcher.exe"
            } else {
                "repo-launcher"
            }),
            icon_png: Some(b"png".to_vec()),
            icon_icns: Some(b"icns".to_vec()),
        }
    }

    #[test]
    fn file_names_are_safe() {
        assert_eq!(file_name("My App"), "My App");
        assert_eq!(file_name("a/b:c*?"), "a-b-c--");
        assert_eq!(file_name("  ..  "), "Project");
        assert_eq!(file_name(&"x".repeat(100)).len(), 60);
    }

    #[test]
    fn never_overwrites_something_that_isnt_ours() {
        let dirs = ShortcutDirs::at(Path::new("/s"));
        let first = plan(&dirs, "Demo", "k3f9x2ab", |_| false);
        let taken = first[0].1.clone();
        let second = plan(&dirs, "Demo", "k3f9x2ab", |p| p == taken);
        if cfg!(target_os = "linux") {
            assert_eq!(first, second, "Linux file names are per project");
        } else {
            assert_ne!(first[0].1, second[0].1);
            assert!(second[0].1.to_string_lossy().contains("Demo (2)"));
        }
    }

    #[test]
    fn creates_and_removes_a_shortcut() {
        let root = tempfile::tempdir().unwrap();
        let dirs = ShortcutDirs::at(root.path());
        let l = launcher(root.path());
        let planned = plan(&dirs, "Demo <&> App", "k3f9x2ab", |p| p.exists());
        assert!(!planned.is_empty());
        for (_, path) in &planned {
            write(
                path,
                "Demo <&> App",
                "k3f9x2ab",
                &l,
                &root.path().join("icons"),
            )
            .unwrap();
            assert!(path.exists(), "{}", path.display());
        }
        let (_, path) = &planned[0];

        #[cfg(target_os = "macos")]
        {
            let plist = std::fs::read_to_string(path.join("Contents/Info.plist")).unwrap();
            assert!(
                plist.contains("<string>Demo &lt;&amp;&gt; App</string>"),
                "{plist}"
            );
            assert!(plist.contains("<key>LSUIElement</key><true/>"));
            let script = std::fs::read_to_string(path.join("Contents/MacOS/launch")).unwrap();
            assert!(
                script.contains("open \"repolauncher://launch/k3f9x2ab\""),
                "{script}"
            );
            assert!(path.join("Contents/Resources/icon.icns").is_file());
        }
        #[cfg(windows)]
        {
            let (target, args) = windows_lnk::read(path).unwrap();
            assert_eq!(Path::new(&target), l.exe);
            assert_eq!(args, "--launch k3f9x2ab");
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            let text = std::fs::read_to_string(path).unwrap();
            assert!(text.contains("Name=Demo <&> App\n"), "{text}");
            assert!(
                text.contains(&format!("Exec=\"{}\" --launch k3f9x2ab\n", l.exe.display())),
                "{text}"
            );
            assert!(text.contains("Icon=/"), "{text}");
        }

        // Rewriting in place (repair) works, then removal leaves nothing behind.
        write(path, "Demo", "k3f9x2ab", &l, &root.path().join("icons")).unwrap();
        for (_, p) in &planned {
            remove(p).unwrap();
            assert!(!p.exists());
            remove(p).unwrap();
        }
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn exec_lines_quote_odd_paths() {
        assert_eq!(exec_arg("/opt/rl/repo-launcher"), "/opt/rl/repo-launcher");
        assert_eq!(exec_arg("/home/a b/rl"), "\"/home/a b/rl\"");
        assert_eq!(exec_arg("/x/$HOME`"), "\"/x/\\$HOME\\`\"");
        assert_eq!(exec_arg("/x/100%"), "/x/100%%");
    }
}
