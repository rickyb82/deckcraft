# MCP server

`deckcraft-cli mcp` runs a Model Context Protocol server on stdio. By default it drives its own
headless engine (no window); with `--connect PORT` it drives a running DeckCraft started with
`--control PORT`, so an agent and a person can work on the same deck. `--sample` opens the sample
deck.

## Files the agent may read and write

A deck or a picture the agent opens can carry instructions aimed at it (prompt injection). So the
headless server reaches the file system only inside two folders given at start:

```sh
deckcraft-cli mcp --automation-read-root /work/talk --automation-write-root /work/talk/out
```

- Every path a tool or command reads (`open_presentation`, `file.open`, `insert_picture`,
  `insert.picture`/`video`/`audio`, `picture.change`, picture fills) must lie inside the read root;
  every path it writes (`save_presentation`, `export`, `file.save`/`saveAs`/`saveTemplate`/`export`,
  `render_slide {path}`) inside the write root. Paths are made absolute against the working
  directory and their links are followed before the comparison: `../` out of a root, an absolute
  path elsewhere and a link that leads out are refused (`automation path rejected: …`), and
  nothing is written.
- The two roots are separate: a write root is not readable unless it is also inside the read root.
- **Without the flags the headless server reads and writes no file by path** (`automation file
  access refused: no read root …`). Decks can still come and go as data: `file.openBytes` and
  `file.saveBytes`, pictures as `data` (base64). To give an agent your whole disk, say so:
  `--automation-read-root / --automation-write-root /`.
- The roots confine the headless server only: with `--connect` the agent drives your running app
  with your rights, and the flags are refused.

With `--connect`, the bridge sends the app's control key, which it reads from
`DECKCRAFT_CONTROL_KEY` or from the key file the app writes at start. It sends the key only to
loopback addresses; see [Keys](control-protocol.md#keys).

```json
{
  "mcpServers": {
    "deckcraft": { "command": "deckcraft-cli", "args": ["mcp", "--connect", "7990"] },
    "deckcraft-headless": { "command": "deckcraft-cli", "args": ["mcp", "--automation-read-root", "/work/talk", "--automation-write-root", "/work/talk"] }
  }
}
```

## Tools

| Tool | What it does |
|---|---|
| `list_commands` | Every command (id, label, menu, shortcut, params doc), optionally filtered |
| `run_command` | Run one command: `{command, params}` — every menu item, ribbon button and gesture is a command |
| `batch` | Run several commands in order and stop at the first error |
| `new_presentation`, `open_presentation`, `save_presentation`, `export` | Files: native `.deckcraft`, PowerPoint `.pptx`, PDF (slides, notes pages, handouts), PNG/JPEG, outline |
| `inspect_document`, `inspect_slide` | Structure of the deck / one slide (shapes, text, placeholders, animations) |
| `render_slide` | A slide as an image the model can look at |
| `add_slide`, `go_to_slide` | Slides by layout |
| `add_shape`, `add_text_box`, `set_text` | Shapes and text |
| `insert_picture`, `insert_table`, `insert_chart` | Content |
| `select`, `pointer`, `select_tool`, `key`, `type_text` | The same selection, mouse and keyboard paths as the UI |
| `screenshot` | The whole window, UI included (connected mode) |
| `call_app` | Any [control protocol](control-protocol.md) method (connected mode) |

Useful commands for building decks: `slide.new {layout, title, body}`, `shape.insert {preset,
rect, text}`, `shape.connect {from, to, preset}`, `shape.freeform {points, closed, smooth}`,
`shape.merge {op, ids}`, `format.*`, `design.theme`, `transition.set`, `animation.set`,
`chart.*`, `table.*`, `file.export {path, layout}`. `deckcraft-cli describe ID` prints any
command's parameters.
