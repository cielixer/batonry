//! The shell from outside: the Simulator opens it headless, input rides the
//! router, and discipline rules (colours, strings) hold as files on disk.

use baton_ui::{App, Message};
use iced_test::simulator;

/// A short marker: headless, no widget ever sends a Resize, so the grid
/// keeps alacritty's narrow default and a long marker wraps and stops
/// matching by line.
const MARKER: &str = "BRTOK";

/// The hint the tests inject and then look for; the assertions match
/// injected values, not anything baked into the crate -- nothing is,
/// that is the point.
const HINT: &str = "Panel";

fn sh_app() -> App {
    let (app, _task) = App::new(
        "/bin/sh".into(),
        "Baton".into(),
        "local".into(),
        HINT.into(),
        "Type a command...".into(),
        "recent".into(),
        None,
    );
    app
}

/// The app opens headless under `iced_test`'s `Simulator`, and what the
/// terminal drew is asserted on the grid dump, not pixels -- proving
/// `baton-ui` stayed a library the Simulator can drive.
///
/// The round trip is the real one: bytes go in through `update`, which
/// routes them through `baton_core::route_input` (A11), the pty echoes, and
/// the reply lands in the grid.
#[test]
fn the_shell_opens_headless_and_the_terminal_answers() {
    let mut app = sh_app();
    assert!(app.dump_grid().is_some(), "the stage-1 terminal spawns");

    let mut ui = simulator(app.view());
    assert!(
        ui.find(HINT).is_ok(),
        "the collapsed right dock exists from day one (A9)"
    );
    drop(ui);

    let input = format!("printf '%s\n' {MARKER}\r");
    let write = baton_term::Event::BackendCall(
        0,
        baton_term::BackendCommand::Write(input.into_bytes()),
    );
    let _task = app.update(Message::Terminal(write));

    let deadline =
        std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let dump = app.dump_grid().expect("terminal stays alive");
        // The echoed command contains the marker too; demand the *output*
        // line, which is the marker alone at the start of a line.
        if dump.lines().any(|l| l.trim_end() == MARKER) {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the pty did not answer within 10s; last grid:\n{dump}"
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

/// The observable difference between "through the router" and "straight to
/// the pty": the router consults focus. With no pane focused, a Write must
/// be dropped by `route_input` -- a shell that bypassed the router would
/// deliver it anyway.
#[test]
fn an_unfocused_write_is_dropped_by_the_router() {
    let mut app = sh_app();
    app.focus(None);

    let write = baton_term::Event::BackendCall(
        0,
        baton_term::BackendCommand::Write(b"printf NOPE_%s X\r".to_vec()),
    );
    let _task = app.update(Message::Terminal(write));

    std::thread::sleep(std::time::Duration::from_millis(1500));
    let dump = app.dump_grid().expect("terminal stays alive");
    assert!(
        !dump.contains("NOPE_"),
        "an unfocused write reached the pty; input bypassed the router:\n{dump}"
    );
}

/// A shell that cannot spawn leaves the app alive with no terminal, and
/// messages for the gone terminal are normal traffic, dropped quietly --
/// one panicking message would kill the whole app.
#[test]
fn a_message_for_a_gone_terminal_is_quietly_dropped() {
    let (mut app, _task) = App::new(
        "/definitely/not/a/shell".into(),
        "Baton".into(),
        "local".into(),
        HINT.into(),
        "Type a command...".into(),
        "recent".into(),
        None,
    );
    assert!(app.dump_grid().is_none(), "spawn failure means no terminal");

    let write = baton_term::Event::BackendCall(
        0,
        baton_term::BackendCommand::Write(b"anything".to_vec()),
    );
    let _task = app.update(Message::Terminal(write));
    let _task = app.update(Message::Terminal(baton_term::Event::BackendCall(
        0,
        baton_term::BackendCommand::Scroll(3),
    )));

    // Still standing: the view builds for the content-less centre.
    let _ui = simulator(app.view());
}

/// Colour literals live in theme.rs alone. Grep-style, same shape as the
/// boundary tests (#16); this file scans, so it lives outside src/.
#[test]
fn colours_have_one_home() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let colour_tokens = ["color!(", "Color::from", "from_rgb"];
    let mut files = Vec::new();
    rust_sources(&src, &mut files);
    assert!(files.len() >= 6, "the walk missed the source tree");
    let mut hits = Vec::new();
    for path in files {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if name == "theme.rs" {
            continue;
        }
        let body = std::fs::read_to_string(&path).expect("readable");
        for (n, line) in body.lines().enumerate() {
            for token in colour_tokens {
                if line.contains(token) {
                    hits.push(format!("{name}:{}: {token}", n + 1));
                }
            }
        }
    }
    assert!(
        hits.is_empty(),
        "colour literals outside theme.rs; the Theme struct is the only \
         place a colour may be written, so themes stay swappable:\n{}",
        hits.join("\n")
    );
}

/// Recurses so a future module directory cannot silently exempt its files.
fn rust_sources(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("readable dir") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            rust_sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}
