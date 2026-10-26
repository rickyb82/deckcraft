//! An agent builds a small deck entirely through MCP and checks its work only through MCP.

use serde_json::{Value, json};

use crate::{Headless, Server};

fn call(s: &mut Server, id: u64, method: &str, params: Value) -> Value {
    let line = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}).to_string();
    let out = s.handle_line(&line).expect("a response");
    serde_json::from_str(&out).expect("json")
}

fn tool(s: &mut Server, name: &str, args: Value) -> Value {
    let r = call(s, 9, "tools/call", json!({"name": name, "arguments": args}));
    let res = &r["result"];
    assert_eq!(res["isError"], false, "{name} failed: {res}");
    let text =
        res["content"].as_array().unwrap().iter().find(|c| c["type"] == "text").map(|c| c["text"].as_str().unwrap().to_string()).unwrap_or_default();
    serde_json::from_str(&text).unwrap_or(Value::String(text))
}

#[test]
fn handshake_and_tool_list() {
    let mut s = Server::new(Box::new(Headless::new()));
    let r = call(&mut s, 1, "initialize", json!({"protocolVersion": "2025-06-18"}));
    assert_eq!(r["result"]["serverInfo"]["name"], "deckcraft");
    let tools = call(&mut s, 2, "tools/list", json!({}));
    let names: Vec<&str> = tools["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
    for n in ["run_command", "render_slide", "add_slide", "add_shape", "pointer", "inspect_slide"] {
        assert!(names.contains(&n), "{n}");
    }
    // Garbage never panics.
    assert!(s.handle_line("not json").is_some());
    assert!(s.handle_line("{}").is_some());
}

#[test]
fn agent_builds_a_deck() {
    let mut s = Server::new(Box::new(Headless::new()));
    tool(&mut s, "new_presentation", json!({"theme": "Harbor"}));
    let doc = tool(&mut s, "inspect_document", json!({}));
    let title_id = {
        let sl = tool(&mut s, "inspect_slide", json!({}));
        sl["shapes"][0]["id"].as_u64().unwrap()
    };
    let _ = doc;
    tool(&mut s, "set_text", json!({"id": title_id, "text": "Quarterly Review"}));
    tool(&mut s, "add_slide", json!({"layout": "titleAndContent", "title": "Highlights", "body": "Revenue up\n\tNew markets\nCosts down"}));
    let shape = tool(&mut s, "add_shape", json!({"preset": "roundRect", "x": 600, "y": 300, "w": 200, "h": 100, "text": "Goal", "fill": "accent2"}));
    let sid = shape["id"].as_u64().unwrap();
    tool(&mut s, "run_command", json!({"command": "shape.rotate", "params": {"id": sid, "deg": 10}}));
    // Drag the shape with the pointer.
    tool(
        &mut s,
        "pointer",
        json!({"events": [{"kind": "down", "x": 700, "y": 350}, {"kind": "drag", "x": 650, "y": 330}, {"kind": "up", "x": 650, "y": 330}]}),
    );
    tool(&mut s, "insert_chart", json!({"type": "pie", "categories": ["A", "B"], "series": [{"name": "S", "values": [3, 5]}]}));
    let sl = tool(&mut s, "inspect_slide", json!({}));
    assert_eq!(sl["title"], "Highlights");
    let shapes = sl["shapes"].as_array().unwrap();
    let goal = shapes.iter().find(|x| x["id"].as_u64() == Some(sid)).unwrap();
    assert_eq!(goal["text"], "Goal");
    assert_eq!(goal["rotation"], 10.0);
    assert!(goal["box"][0].as_f64().unwrap() < 600.0, "moved left: {goal}");
    assert!(shapes.iter().any(|x| x["kind"] == "chart"));
    let doc = tool(&mut s, "inspect_document", json!({}));
    assert_eq!(doc["slides"].as_array().unwrap().len(), 2);
    assert_eq!(doc["theme"], "Harbor");
    // Look at it.
    let r = call(&mut s, 10, "tools/call", json!({"name": "render_slide", "arguments": {"scale": 0.25}}));
    assert_eq!(r["result"]["content"][0]["type"], "image");
    // Keyboard: select the shape, delete it, undo.
    tool(&mut s, "select", json!({"ids": [sid]}));
    tool(&mut s, "key", json!({"key": "Delete"}));
    let n1 = tool(&mut s, "inspect_slide", json!({}))["shapes"].as_array().unwrap().len();
    tool(&mut s, "key", json!({"key": "z", "mods": {"cmd": true}}));
    let n2 = tool(&mut s, "inspect_slide", json!({}))["shapes"].as_array().unwrap().len();
    assert_eq!(n2, n1 + 1);
}

#[test]
fn errors_are_reported_not_panics() {
    let mut s = Server::new(Box::new(Headless::new()));
    let r = call(&mut s, 1, "tools/call", json!({"name": "run_command", "arguments": {"command": "no.such"}}));
    assert_eq!(r["result"]["isError"], true);
    let r = call(&mut s, 2, "tools/call", json!({"name": "screenshot", "arguments": {}}));
    assert_eq!(r["result"]["isError"], true);
    let r = call(&mut s, 3, "tools/call", json!({"name": "add_shape", "arguments": {"preset": "rect"}}));
    assert_eq!(r["result"]["isError"], true);
}

#[test]
fn export_tool_lists_pdf_and_its_options() {
    let mut s = Server::new(Box::new(Headless::new()));
    let tools = call(&mut s, 1, "tools/list", json!({}));
    let export = tools["result"]["tools"].as_array().unwrap().iter().find(|t| t["name"] == "export").unwrap().clone();
    assert!(export["description"].as_str().unwrap().contains("pdf"), "{export}");
    let props = &export["inputSchema"]["properties"];
    let formats = props["format"]["enum"].as_array().unwrap();
    for f in ["png", "jpeg", "pptx", "deckcraft", "outline", "pdf"] {
        assert!(formats.iter().any(|x| x == f), "format {f}: {export}");
    }
    assert_eq!(props["layout"]["enum"], json!(["slides", "notes", "handouts"]));
    assert_eq!(props["perPage"]["enum"], json!([1, 2, 3, 4, 6, 9]));
    for p in ["dpi", "slides", "includeHidden", "textLayer", "frame"] {
        assert!(props.get(p).is_some(), "{p}: {export}");
    }
    // What the schema describes is accepted.
    let dir = std::env::temp_dir().join(format!("deckcraft-mcp-export-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    tool(&mut s, "new_presentation", json!({}));
    for layout in ["slides", "notes", "handouts"] {
        let path = dir.join(format!("{layout}.pdf"));
        tool(&mut s, "export", json!({"path": path.to_string_lossy(), "format": "pdf", "layout": layout, "perPage": 3}));
        assert!(std::fs::read(&path).unwrap().starts_with(b"%PDF"), "{layout}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// `deckcraft-cli mcp --automation-read-root … --automation-write-root …`: a deck the agent reads
/// can't steer it into reading or writing files elsewhere, through any tool.
#[test]
fn tools_are_held_to_the_automation_roots() {
    use deckcraft_engine::file_access::{AutomationRoots, confine};
    let base = std::env::temp_dir().join(format!("deckcraft-mcp-roots-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    for d in ["in", "out", "outside"] {
        std::fs::create_dir_all(base.join(d)).unwrap();
    }
    std::fs::write(base.join("outside/secret.txt"), "top secret").unwrap();
    let p = |rel: &str| base.join(rel).to_string_lossy().into_owned();
    let roots = AutomationRoots::new(Some(&base.join("in")), Some(&base.join("out"))).unwrap();
    let refused = |s: &mut Server, name: &str, args: Value| -> String {
        let r = call(s, 9, "tools/call", json!({"name": name, "arguments": args}));
        assert_eq!(r["result"]["isError"], true, "{name} should fail: {r}");
        r["result"].to_string()
    };
    confine(Some(&roots), || {
        let mut s = Server::new(Box::new(Headless::with_document()));
        tool(&mut s, "save_presentation", json!({"path": p("out/deck.pptx")}));
        tool(&mut s, "render_slide", json!({"path": p("out/slide.png")}));
        assert!(refused(&mut s, "open_presentation", json!({"path": p("outside/secret.txt")})).contains("outside the read root"));
        assert!(refused(&mut s, "insert_picture", json!({"path": p("outside/secret.txt")})).contains("outside the read root"));
        assert!(refused(&mut s, "save_presentation", json!({"path": p("outside/deck.pptx")})).contains("outside the write root"));
        assert!(refused(&mut s, "export", json!({"path": p("outside/deck.pdf")})).contains("outside the write root"));
        assert!(refused(&mut s, "render_slide", json!({"path": p("outside/slide.png")})).contains("outside the write root"));
        let e = refused(&mut s, "run_command", json!({"command": "file.saveAs", "params": {"path": p("outside/x.deckcraft")}}));
        assert!(e.contains("outside the write root"), "{e}");
        // No roots at all (the server's default): no file by path.
        let none = AutomationRoots::new(None, None).unwrap();
        confine(Some(&none), || {
            assert!(refused(&mut s, "save_presentation", json!({"path": p("out/deck2.pptx")})).contains("--automation-write-root"));
        });
    });
    assert!(base.join("out/deck.pptx").is_file() && base.join("out/slide.png").is_file());
    assert_eq!(std::fs::read_dir(base.join("outside")).unwrap().count(), 1, "nothing written outside");
    assert!(!base.join("out/deck2.pptx").exists());
    let _ = std::fs::remove_dir_all(base);
}
