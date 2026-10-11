# Manual QA

What automated tests can't check: real installers on clean machines, system dialogs, keychain
prompts, menus outside the WebView, screen readers. Run it for every release (PLAN §12), with the
exact files from the draft release or a manual Release run ([RELEASING.md](RELEASING.md)).

**The definition of done (PLAN §14)** is this whole page passing on a clean VM of each OS: a fresh
user account with no git, Python, Node or developer tools installed.

Record each run in the release's notes: OS and version, installer file, who ran it, and any ✗.

## Clean VMs

| OS | Image | Make sure |
|---|---|---|
| macOS | The oldest and newest supported macOS (≥ 10.15; ideally 14 and the current one), Apple silicon and Intel | No Xcode Command Line Tools (`xcode-select -p` fails), no Homebrew |
| Windows | Windows 11 and Windows 10 22H2, x64 | No Python, Node or git; WebView2 is present on 11 (the installer fetches it on 10 if missing) |
| Linux | Ubuntu 22.04 and 24.04 (GNOME), Fedora (current, KDE) | No `python3-venv`, Node or git beyond what the desktop installs; one VM with **no** Secret Service |

## Every OS

### Install and first run
- [ ] Download the installer in a browser (so it's quarantined / marked as downloaded).
- [ ] Signed builds: no warning (macOS opens normally; no SmartScreen prompt). Unsigned builds:
      note exactly what the OS showed.
- [ ] The app opens to the welcome: "Run projects from GitHub — no setup needed", five starter
      projects, and the link field has the keyboard focus.
- [ ] "Try it" on **Uber pickups in New York**: the review opens straight away; Install; it
      finishes and Launch opens the map in the browser. Time from first launch to the map:
      ____ (goal: under 30 s of the user's own time, plus download time).
- [ ] Try each of the other four starters; each installs and launches (cowsay opens its
      console window and prints a cow).
- [ ] A link with a typo shows a plain sentence, not a code.

### Links, shortcuts, background mode
- [ ] `repolauncher://launch/<id>` typed in the browser's address bar asks to open Repo Launcher
      and launches the project (the installer registered the scheme).
- [ ] Add a shortcut for a web project; quit Repo Launcher; open the shortcut: the project opens
      in the browser **without** the main window; the tray / menu bar icon appears.
- [ ] Turn on "Start when I log in" for a project; log out and in: the project starts, no window.
- [ ] Settings → "Start Repo Launcher when I log in" on and off: the system's login items agree.

### Tray / menu bar
- [ ] The icon is visible and clear on light and dark menu bars / panels (macOS: a monochrome
      template icon that follows the menu bar's color).
- [ ] With nothing running the menu says "No projects running"; with a project running it lists
      it with **Open** (web) or **Show window** (console) and **Stop**, each working.
- [ ] **Show Repo Launcher** brings the window back after closing it.
- [ ] **Quit Repo Launcher** with a project running asks first ("Quit Repo Launcher?" listing the
      project); **Stop and quit** stops it (its port is free afterwards); **Cancel** keeps it.
- [ ] Every word in the tray menu and (macOS) the app menu is the wording from `en.json`; run
      once with `REPO_LAUNCHER_PSEUDO=1` and check every menu label is in the ⟦pseudo⟧ form.

### Keys and GitHub
- [ ] A project that needs a key asks for it once before its first launch; the key reaches the
      project and appears nowhere on disk (search the data folder for it).
- [ ] The system's keychain shows the entry under `app.repolauncher.desktop`.
- [ ] (When the GitHub App is registered) Connect GitHub, open a private repository, disconnect.

### "Open in my terminal"
- [ ] For cowsay: ⋯ → "Open in my terminal" on, then Launch: the system's terminal opens and
      runs it; after it ends, Enter closes the window.
- [ ] A project with a key, through the terminal: the program gets the key (macOS may show a
      keychain prompt once for `repo-launcher`; allow it); the `.command` script in the project's
      `meta` folder holds only the app's path and the id.

### Updates (once updates are set up)
- [ ] Install the previous release, then publish the new one: within a minute of starting (or
      with Settings → Updates → Check for updates) "Repo Launcher x.y.z is ready to install"
      appears; **Update and restart** asks first if projects are running, then restarts into the
      new version with the library intact.
- [ ] Turning off "Check for updates automatically" stops the daily check.

### Accessibility
- [ ] Keyboard only, from the first screen: add a link, review, install, launch, open the ⋯ menu,
      open and close Settings. Focus is always visible and returns to where it was when a dialog
      closes.
- [ ] Screen reader: VoiceOver (macOS), Narrator or NVDA (Windows), Orca (Linux). The project
      cards read as "article, <name>", progress is announced, problems are announced, dialogs
      have a name and a description.
- [ ] System text size at its largest: nothing is cut off or overlaps.
- [ ] "Reduce motion" on: dialogs and cards appear without animation.
- [ ] Dark mode: everything readable.

### Removing everything
- [ ] Settings → Remove all data… lists what will be deleted; **Remove everything** stops the
      running projects and quits.
- [ ] Afterwards: no shortcuts, no login item, no keychain entries under
      `app.repolauncher.desktop`, and the data folder is gone (macOS
      `~/Library/Application Support/app.repolauncher.desktop`, Windows
      `%LOCALAPPDATA%\app.repolauncher.desktop`, Linux `~/.local/share/repo-launcher`).

## macOS only
- [ ] The dmg opens with the app and an Applications link; dragging installs it.
- [ ] ⌘Q from the app menu with a project running asks first (D-037); with nothing running it
      quits at once.
- [ ] The app menu's items (About, Hide, Services, Quit) and the Edit and Window menus work, and
      ⌘C / ⌘V work in the link field.
- [ ] The shortcut app in ~/Applications shows the project's own icon when the project has a
      square PNG logo, otherwise Repo Launcher's.
- [ ] Logging out with projects running: they stop (D-037); nothing is left running.
- [ ] Sleep and wake with a project running: it's still "Running" and still answers.

## Windows only
- [ ] setup.exe installs for the current user without an administrator prompt; the msi asks for
      administrator rights and installs for all users.
- [ ] Shortcuts appear in the Start menu (folder "Repo Launcher") and on the desktop, with the
      project's icon when it has one.
- [ ] Uninstall (Settings → Apps) of the setup.exe version, with "Delete the application data"
      ticked: shortcuts, the `Run` registry value, Credential Manager entries
      (`app.repolauncher.desktop`) and `%LOCALAPPDATA%\app.repolauncher.desktop` are all gone.
      Without the tick: the same, except the data folder stays.
- [ ] Uninstall of the msi version: shortcuts, login item and Credential Manager entries are gone.
- [ ] An update (setup.exe) keeps shortcuts, keys and projects.
- [ ] Windows Defender doesn't flag the app or its downloaded tools; note install times.

## Linux only
- [ ] deb on Ubuntu, rpm on Fedora: install through the software center or `apt`/`dnf`; the app
      appears in the applications menu.
- [ ] AppImage: runs after `chmod +x`; links and shortcuts point at the AppImage itself; moving
      the AppImage and starting it again repairs its shortcuts.
- [ ] GNOME without the AppIndicator extension: no tray, and the app still works (closing the
      window quits when nothing runs).
- [ ] KDE: the tray menu works.
- [ ] The VM without a Secret Service: projects without keys work; the keys form explains why it
      can't save.
- [ ] Wayland and X11 sessions both show the window correctly.

## Phase 4 items that need a screen

These were built in Phase 4 and are covered above; listed here so none is missed on the first
full run:

- the tray / menu bar menu and its clicks, and the quit confirmation;
- ⌘Q from the macOS app menu;
- "Open in my terminal" opening Terminal.app, and a project with keys through `--exec`
  (keychain prompt);
- the monochrome template icon on the macOS menu bar;
- the macOS Edit and Window menus and the tray strings coming from `en.json` (pseudo-locale run).
