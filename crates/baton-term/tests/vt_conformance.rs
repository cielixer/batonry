//! What the terminal actually put on the grid.
//!
//! Truecolor, underline variants, SGR combinations, CJK width, emoji, box
//! drawing, the 256-colour palette and OSC 8 all go onto one page; then we
//! assert against `Terminal::dump_grid()`.
//!
//! **There is no pixel golden, on purpose.** A PNG comparison only says that
//! something changed, never what is correct, and it drags in a bundled font
//! (system font fallback differs per machine -- a missing bold face alone made
//! two platforms differ by 801 pixels). The dump is text, so it gives the same
//! answer on macOS, Linux and CI.
//!
//! When something has to be seen rather than asserted, run the app.

use std::io::Write;
use std::time::{Duration, Instant};

use baton_term::{Terminal, TerminalView};
use iced_test::Simulator;

/// Marker on the last line. Once it shows up on the grid, every byte before it
/// has been parsed.
const PAGE_SENTINEL: &str = "eof-baton";

fn page() -> String {
    let mut s = String::new();

    // 24-bit truecolor gradient. Banding or clumping means the SGR 38;2 path
    // is wrong.
    s.push_str("truecolor ");
    for i in 0..48 {
        let r = (i * 5) as u8;
        let g = 255u8.saturating_sub((i * 5) as u8);
        s.push_str(&format!("\x1b[48;2;{r};{g};128m "));
    }
    s.push_str("\x1b[0m\r\n");

    // Underline variants: curly (4:3), dotted (4:4), dashed (4:5), double (21),
    // plus a coloured underline (SGR 58) that must differ from the text colour.
    s.push_str("underline  ");
    s.push_str("\x1b[4msingle\x1b[0m ");
    s.push_str("\x1b[4:3mcurly\x1b[0m ");
    s.push_str("\x1b[4:4mdotted\x1b[0m ");
    s.push_str("\x1b[4:5mdashed\x1b[0m ");
    s.push_str("\x1b[21mdouble\x1b[0m ");
    s.push_str("\x1b[4m\x1b[58;2;255;80;80mred-under\x1b[59m\x1b[0m\r\n");

    s.push_str(
        "sgr        \x1b[1mbold\x1b[0m \x1b[2mdim\x1b[0m \x1b[3mitalic\x1b[0m ",
    );
    s.push_str(
        "\x1b[7minverse\x1b[0m \x1b[9mstrike\x1b[0m \x1b[5mblink\x1b[0m\r\n",
    );

    // CJK width. Each character must occupy exactly two cells to line up with
    // the ruler above it.
    s.push_str("cjk-ruler  |....|....|....|....|....|\r\n");
    s.push_str("cjk-wide   한글이제대로정렬되는가\r\n");
    s.push_str("cjk-mixed  ab한글cd漢字ef가나다\r\n");

    // Emoji, including a variation selector and a regional indicator pair.
    s.push_str(
        "emoji      \u{1f600}\u{1f680}\u{2764}\u{fe0f}A\u{1f1f0}\u{1f1f7}B\r\n",
    );

    // Box drawing. Lines have to meet at cell boundaries; leaving this to the
    // font does not achieve that, which is why procedural box drawing is
    // planned.
    s.push_str("box        \u{250c}\u{2500}\u{2500}\u{252c}\u{2500}\u{2500}\u{2510} \u{2502}\u{2502} \u{2514}\u{2500}\u{2534}\u{2500}\u{2518} \u{2588}\u{2593}\u{2592}\u{2591}\r\n");

    s.push_str("ansi16     ");
    for i in 0..16 {
        s.push_str(&format!("\x1b[48;5;{i}m  "));
    }
    s.push_str("\x1b[0m\r\n");

    s.push_str("osc8       \x1b]8;;https://example.com\x07link text\x1b]8;;\x07 plain\r\n");

    s.push_str(PAGE_SENTINEL);
    s.push_str("\r\n");
    s
}

/// Feed the page through a pty and wait until the sentinel appears.
///
/// **No sleeping for a fixed time.** An earlier version slept 250 ms three
/// times, and that was a race: on a slower machine the page had not fully
/// arrived and the failure looked like a font problem.
fn render_and_dump() -> String {
    let dir = std::env::temp_dir().join("baton-vt-conformance");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("page.txt");
    std::fs::File::create(&path)
        .unwrap()
        .write_all(page().as_bytes())
        .unwrap();

    let mut term = Terminal::new(
        0,
        baton_term::settings::Settings {
            font: baton_term::settings::FontSettings {
                size: 14.0,
                ..Default::default()
            },
            backend: baton_term::settings::BackendSettings {
                program: "/bin/sh".into(),
                args: vec![
                    "-c".into(),
                    format!("cat {}; sleep 30", path.display()),
                ],
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .expect("pty");

    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        // The widget reads the layout and asks for a grid size; we hand that
        // command back. Same path as the real app: size comes from window
        // geometry and font metrics, never from a render result.
        let msgs: Vec<baton_term::Event> = {
            let mut ui: Simulator<baton_term::Event> =
                Simulator::new(TerminalView::show(&term));
            // One draw is needed for layout to settle. The pixels are unused.
            let _ = ui.snapshot(&iced::Theme::Dark);
            ui.into_messages().collect()
        };
        for baton_term::Event::BackendCall(_, cmd) in msgs {
            let _ = term.handle(baton_term::Command::ProxyToBackend(cmd));
        }

        // `dump_grid` runs a sync, so this is "what is on screen right now".
        let dump = term.dump_grid();
        if dump.contains(PAGE_SENTINEL) {
            return dump;
        }
        assert!(
            Instant::now() < deadline,
            "page did not fully arrive within 20s. grid so far:\n{dump}"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

#[test]
fn vt_parsing_and_cell_layout_are_correct() {
    let dump = render_and_dump();

    for expected in [
        "truecolor",
        "cjk-ruler  |....|....|....|....|....|",
        "cjk-wide   한글이제대로정렬되는가",
        "cjk-mixed  ab한글cd漢字ef가나다",
        "box        \u{250c}\u{2500}\u{2500}\u{252c}\u{2500}\u{2500}\u{2510}",
        "osc8       link text plain",
        PAGE_SENTINEL,
    ] {
        assert!(
            dump.contains(expected),
            "{expected:?} is missing from the grid. That is a VT parsing bug, \
             not a pixel problem.\ngrid:\n{dump}"
        );
    }

    // A spacer leaking out between two double-width characters means the width
    // calculation is wrong.
    assert!(
        !dump.contains("한 글"),
        "a spacer leaked between two Hangul syllables -- width is wrong"
    );
}
