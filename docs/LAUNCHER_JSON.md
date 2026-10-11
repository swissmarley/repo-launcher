# `launcher.json`

Add a `launcher.json` to the root of your repository to tell Repo Launcher exactly how to install
and run your project. Without one, Repo Launcher works it out from your files (which is usually
right for common Python and Node projects). With one, it does exactly what you say.

Editors can validate the file with [`schema/launcher.schema.json`](../schema/launcher.schema.json).

## A complete example

```json
{
  "$schema": "https://raw.githubusercontent.com/swissmarley/repo-launcher/main/schema/launcher.schema.json",
  "version": 1,
  "type": "web",
  "name": "Photo Restorer",
  "description": "Fix old photos in your browser.",
  "requires": { "python": ">=3.10,<3.13", "system": ["ffmpeg"] },
  "install": [
    { "name": "Install Python helpers", "python": { "requirements": ["requirements.txt"] } },
    { "name": "Install website helpers", "node": {}, "cwd": "web" },
    { "name": "Build the website", "run": ["npm", "run", "build"], "cwd": "web" },
    { "name": "Download the model", "run": ["python", "scripts/fetch_model.py"] }
  ],
  "run": { "command": ["python", "app.py", "--port", "{port}", "--host", "{host}"] },
  "web": { "port": 7860, "path": "/" },
  "env": [
    {
      "name": "OPENAI_API_KEY",
      "label": "OpenAI API key",
      "description": "Used to write captions.",
      "required": true,
      "helpUrl": "https://platform.openai.com/api-keys"
    }
  ],
  "overrides": {
    "windows": { "run": { "command": ["python", "app.py", "--port", "{port}", "--no-reload"] } }
  }
}
```

## Fields

| Field | Required | Meaning |
|---|---|---|
| `version` | yes | Always `1`. |
| `type` | yes | `web` opens in the browser, `terminal` runs in the built-in console, `gui` is a desktop window. |
| `run.command` | yes | The program and its arguments, as a list. |
| `run.cwd`, `run.env` | | Working folder (relative to the repo) and extra environment variables. |
| `name`, `description`, `icon` | | Shown on the project card. |
| `requires.python` | | Python version range ([PEP 440](https://peps.python.org/pep-0440/)). Repo Launcher installs a private copy. |
| `requires.node` | | Node version range. Repo Launcher installs a private copy (current LTS lines). |
| `requires.packageManager` | | `npm`, `pnpm`, `yarn` or `bun`, optionally `@version`. Defaults to what your lockfile implies. |
| `requires.system` | | Programs the user must already have (e.g. `ffmpeg`). Checked before launch, never installed. |
| `install[]` | | Steps run in order. Each has exactly one of `run` (a command), `python` (`requirements`, `packages`, `project`) or `node` (install dependencies). Optional: `name`, `cwd`, `env`, `platforms`. |
| `web.port` | | Preferred port. If it's busy, another free port is used. |
| `web.portEnv` | | An environment variable that receives the port, for apps that read e.g. `PORT`. |
| `web.path` | | What to open in the browser. Default `/`. |
| `web.discover` | | `true` if your app chooses its own port: Repo Launcher reads the address from its output. Implied when the port can't be passed in. |
| `web.readyTimeoutSeconds` | | How long to wait for the first response. Default 300. |
| `env[]` | | Settings the user enters, such as API keys. Stored in the OS keychain and passed only to your app. `secret` defaults to `true`. |
| `platforms` | | Limit to some of `macos`, `windows`, `linux`. |
| `overrides.<os>` | | Values merged over the rest of the file on that OS. |

### Placeholders

Use these inside commands and environment values:

| Placeholder | Becomes |
|---|---|
| `{port}` | The port chosen for this launch. |
| `{host}` | `127.0.0.1`, so the app is only reachable from this computer. |
| `{token}` | A random value, new on every launch (use it to protect a local server). |
| `{src}` | The project's folder. |
| `{env}` | The project's private Python environment folder. |

### Command names

The first item of a command is resolved in this order:

1. `python`, `pip`, `uv` → the project's private Python environment.
2. `node`, `npm`, `npx`, `pnpm`, `yarn`, `bun` → the project's private Node and package manager.
3. `./path/to/script` → a file in your repository.
4. Anything else → a program in the project's environment (`.venv`, `node_modules/.bin`) or listed
   in `requires.system`.

## What Repo Launcher refuses

To keep users safe, a `launcher.json` is rejected (with a plain explanation) if a command:

- asks for administrator rights (`sudo`, `su`, `doas`, `runas`, `pkexec`, `gsudo`);
- points at an absolute path outside the project (`/usr/bin/...`, `C:\...`);
- runs inline shell code (`bash -c "..."`, `cmd /c ...`, `powershell -Command ...`): put the code
  in a script file in your repository instead, where users can read it;
- or if the file has unknown fields (usually typos).
