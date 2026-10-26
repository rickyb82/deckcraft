# Control protocol

Start the app with `--control <port>`. The server listens on `127.0.0.1` only and speaks JSON
lines: one request object per line, with the window's key (see [Keys](#keys)), one reply per line.

```json
{"id": 1, "key": "…", "method": "engine.execute", "params": {"command": "shape.insert", "params": {"preset": "roundRect", "rect": [100, 100, 240, 120], "text": "Hello"}}}
{"id": 1, "ok": true, "result": {"id": 258}}
```

Errors come back as `{"id": …, "ok": false, "error": "message"}`. A line that isn't valid JSON
gets one error reply, then the server closes the connection (so an HTTP request sent to the port
can't run a command); reconnect to continue.

## Keys

Any local program (and possibly a web page) can reach a loopback port, so every request needs the
window's key. Once the port is bound, the app writes a random key (256 bits, as hex) to
`<settings>/control-key.local-<port>` and removes the file when the window closes; a new start
writes a new key. `<settings>` is the folder that holds the preferences (`ui.json`): on Linux
`$XDG_CONFIG_HOME/deckcraft` (by default `~/.config/deckcraft`), on macOS
`~/Library/Application Support/DeckCraft`, on Windows `%APPDATA%\DeckCraft`. On Unix the file has
mode 0600.

```sh
cat ~/.config/deckcraft/control-key.local-7990   # Linux, port 7990
```

- Send the key as top-level `"key"` with every request, or once per connection with
  `{"method": "auth", "key": "…"}` (reply `{"ok": true, "result": {}}`); later requests on that
  connection may leave `key` out. Only `auth` reads `params.key`; for other methods `params.key`
  is an ordinary parameter (`ui.key {"key": "Enter"}` presses Enter).
- The key is checked on every request. A missing or wrong key gets
  `{"ok": false, "error": "unauthorized"}`, and the app closes the connection.
- A line over 1 MiB gets `{"ok": false, "error": "line too long"}`, and the app closes the
  connection.
- A connection that sends no authorised request within 30 s of being accepted is closed. That is
  one limit: blank lines and bad JSON do not extend it. After an authorised request the
  connection may stay idle.
- At most 16 connections are served at once; a further one gets
  `{"ok": false, "error": "too many connections"}` and is closed. Close connections you no longer
  need.

`deckcraft-cli mcp --connect` and `deckcraft-cli app` take the key from `DECKCRAFT_CONTROL_KEY`,
or else from the key file for the port they connect to, and read it again on every new connection
(the app may have restarted). They send the key only to loopback addresses (`127.0.0.0/8`, `::1`,
`localhost`). For any other host they send no key, say so, and the app refuses their requests: to
drive an app on another machine, forward its port to `127.0.0.1` (for example
`ssh -L 7990:127.0.0.1:7990 host`) and set `DECKCRAFT_CONTROL_KEY` to that app's key.

## Methods

Coordinates are slide points (1/72 inch; a 16:9 slide is 960 × 540) unless noted.

| Method | Params | What it does |
|---|---|---|
| `engine.execute` / `ui.menu.invoke` / `command` | `{command, params}` | Run any command (see `engine.commands`; `deckcraft-cli describe ID` documents one) |
| `engine.commands` | — | Every command: id, label, menu path, shortcut, params doc, enabled / disabled reason |
| `document.inspect` | — | Slides, sections, layouts, shapes, selection |
| `ui.inspect` | — | UI state, active tool, window/canvas/slide rects, dialog, Home (start screen over the open presentations), slide show state, documents, perf (fps, render ms) |
| `ui.tool.select` | `{tool, preset?}` | `select`, `text`, `shape` + preset, any preset name, `curve`, `freeform`, `scribble`, `pen`, `highlighter`, `eraser` |
| `ui.pointer` | `{events: [{kind: down\|drag\|up\|move\|doubleclick\|tripleclick, x, y, mods?}], mods?}` | Drive the active tool in slide points through the same code path as the mouse |
| `ui.key` / `ui.text` | `{key, shift?, alt?, cmd?}` / `{text}` | Keyboard input (also drives a running slide show) |
| `ui.move` / `ui.click` / `ui.drag` | screen points | Real egui pointer input — reaches every widget, menu and pane |
| `ui.set` | `{tab?, view?, pane?, notes?, zoom?, brightness?, ruler?, gridlines?, guides?, …}` | UI state (returns the full state) |
| `ui.dialog.open` / `ui.dialog.close` | `{id, fields?}` | Open or close a dialog by id; `fields` pre-fills its fields (About: `{"tab": "contributors"}` or `"models"`) |
| `ui.screenshot` | `{path?}` | PNG of the whole window (base64 when no path) |
| `ui.render` | `{path?, slide?, scale?}` | Render a slide headlessly |
| `ui.resize` / `ui.focus` | | Window control |
| `show.start` / `show.end` | `{from?: slide index, presenter?, reading?}` | Slide show |
| `app.open` / `app.save` / `app.export` / `app.quit` | `{path, …}` | Files (export takes the `file.export` params, e.g. `{"path": "deck.pdf", "layout": "handouts", "perPage": 3}`) |

The MCP server (`deckcraft-cli mcp`, see [mcp.md](mcp.md)) wraps the same methods for Claude
and other agents; `deckcraft-cli app METHOD [JSON]` sends one request from a shell.
