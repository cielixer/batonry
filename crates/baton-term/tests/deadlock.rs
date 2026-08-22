//! Deadlock regression.
//!
//! `iced_term` 0.8.0 called `blocking_send` from inside `pty_read`, which runs
//! while `FairMutex<Term>` is held. The only task that can drain that channel
//! is the UI thread, and the UI thread is waiting for the same lock, so the
//! app wedges forever. Reproduced 100% of the time with
//! `yes | head -c 100000`.
//!
//! **What this file asserts:**
//!   1. The load **finishes** (a deadlock blows the deadline and fails)
//!   2. `EVENTS_DROPPED == 0` -- `Exit` and `PtyWrite` are never thrown away
//!   3. `Exit` really arrives and becomes `Action::Shutdown`, so a pane learns
//!      its process died instead of becoming a dead end
//!   4. The consumer still acquires the lock quickly under load -- the
//!      observable form of "never block while holding the lock"
//!
//! The metrics are global, so these tests run **serially** (`SERIAL`).

use std::sync::atomic::Ordering::Relaxed;
use std::sync::{mpsc, Mutex};
use std::time::{Duration, Instant};

use baton_term::actions::Action;
use baton_term::settings::{BackendSettings, Settings};
use baton_term::{metrics, BackendCommand, Command, Event, Terminal};
use iced::futures::StreamExt;

/// Global counters, so one test at a time.
static SERIAL: Mutex<()> = Mutex::new(());

/// Keep going even if an earlier panic poisoned the lock; all we want from it
/// is serialisation.
fn serial() -> std::sync::MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

/// If the deadlock comes back this has to **fail, not hang**.
///
/// So the pump lives on its own OS thread and reports through a channel: even
/// with the pump asleep on the lock, the test thread is alive to judge the
/// deadline. (`tokio::time::timeout` cannot do this -- a future blocked
/// synchronously is never cancelled.)
const DEADLINE: Duration = Duration::from_secs(45);

struct Outcome {
    saw_shutdown: bool,
    /// Window titles we received. **This is the proof that the load actually
    /// flowed**: order is guaranteed within a VT stream, so receiving `DONE`
    /// means every byte before it was parsed. It is also exactly the event the
    /// deadlock destroyed -- when we first hit it, the `DONE` title vanished
    /// and the measurement never terminated.
    titles: Vec<String>,
    commands: u64,
    /// Longest wait to acquire the `Backend` lock while under load.
    max_lock_wait: Duration,
    elapsed: Duration,
}

/// `consumer_delay` stands in for render cost.
///
/// The deadlock window only opens while the producer holds the lock and waits
/// on the channel *and* the consumer wants that lock. An infinitely fast
/// consumer almost never hits it. The real app's consumer is a 60 fps
/// renderer; without this delay the test passes even with the bug
/// reintroduced -- verified by putting `blocking_send` back.
fn run_load(
    program: &str,
    deadline: Duration,
    consumer_delay: Duration,
) -> Option<Outcome> {
    let (tx, rx) = mpsc::channel();
    let program = program.to_string();

    // `Terminal` holds an iced canvas `Cache`, which is not meant to move
    // between threads, so build and drop it inside this thread.
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let outcome = rt.block_on(async move {
            let mut term = Terminal::new(
                0,
                Settings {
                    backend: BackendSettings {
                        program: "/bin/sh".into(),
                        args: vec!["-c".into(), program],
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .expect("pty");

            let mut stream = term.event_stream();
            let started = Instant::now();
            let mut saw_shutdown = false;
            let mut titles = Vec::new();
            let mut max_lock_wait = Duration::ZERO;

            // **Do not stop at `Shutdown`.** The `biased` select checks the
            // lossless path first, so `Exit` overtakes pending wakeups. That
            // is fine -- we assign no meaning to ordering between sources --
            // but it means we have to drain the tail before judging.
            loop {
                let quiet = if saw_shutdown { 150 } else { 5_000 };
                let next = tokio::time::timeout(
                    Duration::from_millis(quiet),
                    stream.next(),
                )
                .await;
                let cmd = match next {
                    Ok(Some(Event::BackendCall(_, cmd))) => cmd,
                    // Stream ended, or went quiet.
                    Ok(None) | Err(_) => break,
                };

                // Time the lock wait: `handle` takes `FairMutex<Term>`.
                let t0 = Instant::now();
                let action = term.handle(Command::ProxyToBackend(cmd));
                max_lock_wait = max_lock_wait.max(t0.elapsed());

                match action {
                    Action::Shutdown => saw_shutdown = true,
                    Action::ChangeTitle(t) => titles.push(t),
                    Action::Ignore => {},
                }
                if !consumer_delay.is_zero() {
                    tokio::time::sleep(consumer_delay).await;
                }
                if started.elapsed() > deadline {
                    break;
                }
            }

            Outcome {
                saw_shutdown,
                titles,
                commands: metrics::COMMANDS.load(Relaxed),
                max_lock_wait,
                elapsed: started.elapsed(),
            }
        });
        let _ = tx.send(outcome);
    });

    rx.recv_timeout(deadline).ok()
}

#[test]
fn sustained_output_does_not_deadlock_and_loses_nothing() {
    let _guard = serial();
    metrics::reset();

    // The load that wedged the original. OSC 0 markers bracket it: receiving
    // `DONE` means everything before it was parsed, and that title going
    // missing was exactly the symptom of the deadlock.
    let outcome = run_load(
        "printf '\\033]0;GO\\007'; yes | head -c 2000000; \
         printf '\\033]0;DONE\\007'",
        DEADLINE,
        Duration::from_millis(4),
    );

    let Some(o) = outcome else {
        panic!(
            "deadlock: did not finish within {DEADLINE:?} -- code that waits on a \
             channel while holding the lock is back"
        );
    };

    // 1. Zero loss. This is the property the fix exists for.
    assert_eq!(
        metrics::EVENTS_DROPPED.load(Relaxed),
        0,
        "a non-wakeup event was dropped: a lost Exit leaves a dead-end pane, a \
         lost PtyWrite swallows a DA/DSR reply"
    );

    // 2. `Exit` arrived, so a pane knows its process died.
    assert!(
        o.saw_shutdown,
        "Exit never arrived: the shell finished but the pane does not know, which \
         is a dead end. commands={} elapsed={:?}",
        o.commands, o.elapsed
    );

    // 3. The load ran to completion. **Not measured by command count** --
    //    source-side coalescing is efficient enough that 100 KB can arrive in
    //    a handful of commands (we measured four). Check the end marker.
    assert_eq!(
        o.titles,
        vec!["GO".to_string(), "DONE".to_string()],
        "unexpected title markers: {:?} -- no DONE means we either never finished \
         parsing the 100 KB or lost a Title",
        o.titles
    );

    // 4. The unbounded path really was safe: the queue stayed shallow.
    let qmax = metrics::EVENTS_QUEUE_MAX.load(Relaxed);
    assert!(
        qmax < 10_000,
        "the unbounded queue grew to {qmax}; the assumption that these events are \
         rare is broken and Title/PtyWrite would need coalescing too"
    );

    // 5. The observable form of "never block while holding the lock": if the
    //    sender sleeps inside it, this jumps to seconds.
    assert!(
        o.max_lock_wait < Duration::from_secs(2),
        "lock wait reached {:?} under load: the sender is waiting while holding it",
        o.max_lock_wait
    );

    eprintln!(
        "load ok: commands={} coalesced={} queue_max={} max_lock_wait={:?} elapsed={:?}",
        o.commands,
        metrics::WAKEUPS_COALESCED.load(Relaxed),
        qmax,
        o.max_lock_wait,
        o.elapsed
    );
}

/// A quiet exit does not lose `Exit` either -- the no-load control.
#[test]
fn quiet_exit_still_arrives() {
    let _guard = serial();
    metrics::reset();

    let o =
        run_load("printf hi", DEADLINE, Duration::ZERO).expect("must finish");
    assert!(
        o.saw_shutdown,
        "Exit must arrive even with almost no output"
    );
    assert_eq!(metrics::EVENTS_DROPPED.load(Relaxed), 0);
}

/// A source-level contract, kept as a test rather than a comment:
/// `blocking_send` must not reappear in `backend.rs`. That call *was* the
/// deadlock.
#[test]
fn backend_never_blocks_while_holding_the_term_lock() {
    let src = include_str!("../src/backend.rs");
    // The word appears in prose above, so only look at code.
    for (i, line) in src.lines().enumerate() {
        let code = line.split("//").next().unwrap_or("");
        assert!(
            !code.contains("blocking_send"),
            "blocking_send is back at backend.rs:{}. That call happens inside \
             FairMutex<Term>, and it is the deadlock",
            i + 1
        );
        assert!(
            !code.contains("blocking_recv"),
            "backend.rs:{}: blocking_recv is out for the same reason",
            i + 1
        );
    }
}

/// Confirms at the path level that wakeups may be dropped and nothing else
/// may be. The wakeup channel has capacity 1, so it certainly overflows under
/// load -- and loss must still be zero.
#[test]
fn wakeup_channel_overflow_is_coalescing_not_loss() {
    let _guard = serial();
    metrics::reset();

    // Change the title repeatedly, then exit. Titles must take the unbounded
    // path while the wakeups in between get coalesced.
    // Mix heavy output with 200 titles. In the original, wakeups would fill
    // the queue and push titles out. We must receive all 200.
    const N: usize = 200;
    let o = run_load(
        "i=0; while [ $i -lt 200 ]; do \
           printf '\\033]0;t%d\\007' $i; yes | head -c 2000; \
           i=$((i+1)); \
         done",
        DEADLINE,
        Duration::from_millis(2),
    )
    .expect("must finish");

    assert_eq!(
        metrics::EVENTS_DROPPED.load(Relaxed),
        0,
        "loss appeared with titles plus heavy output"
    );
    let expected: Vec<String> = (0..N).map(|i| format!("t{i}")).collect();
    assert_eq!(
        o.titles,
        expected,
        "only {1} of {0} titles arrived, or the order broke -- this is the property \
         the deadlock fix has to preserve",
        N,
        o.titles.len()
    );
    eprintln!(
        "titles+load: titles={} commands={} coalesced={} queue_max={} shutdown={}",
        o.titles.len(),
        o.commands,
        metrics::WAKEUPS_COALESCED.load(Relaxed),
        metrics::EVENTS_QUEUE_MAX.load(Relaxed),
        o.saw_shutdown
    );
}

/// Keeps the raw-write path alive: we must be able to send bytes without a
/// forced newline. Later milestones prefill a command and let the user press
/// Enter, which is impossible if anything on this path appends a newline.
#[test]
fn write_path_sends_raw_bytes_without_newline() {
    let _guard = serial();
    let mut term = Terminal::new(
        99,
        Settings {
            backend: BackendSettings {
                program: "/bin/sh".into(),
                args: vec!["-c".into(), "sleep 1".into()],
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .expect("pty");
    let action = term.handle(Command::ProxyToBackend(BackendCommand::Write(
        b"echo no newline".to_vec(),
    )));
    assert!(matches!(action, Action::Ignore));
}

/// Instrumentation must not be able to kill the thread it measures.
///
/// `EVENTS_QUEUED` is a gauge, and `metrics::reset()` can land between a push
/// and its matching pop: a finished `Terminal` leaves alacritty's detached
/// "PTY reader" thread alive, so its events get drained after the next test has
/// already zeroed the counter. With a wrapping `fetch_sub` the gauge goes to
/// `u64::MAX` and the following `fetch_add(1) + 1` **panics on overflow in a
/// debug build**, taking the reader thread with it.
///
/// That is not hypothetical -- it is why
/// `wakeup_channel_overflow_is_coalescing_not_loss` failed intermittently under
/// `cargo test --workspace`: the reader died mid-load, output stopped, and the
/// missing titles showed up as an assertion failure in a *different* test.
#[test]
fn queue_gauge_survives_a_reset_between_push_and_pop() {
    let _guard = serial();
    metrics::reset();

    metrics::queue_push();
    metrics::reset(); // the pop below is now unmatched
    metrics::queue_pop(); // wrapping would land on u64::MAX here
    assert_eq!(
        metrics::EVENTS_QUEUED.load(Relaxed),
        0,
        "the gauge went below zero instead of saturating"
    );

    metrics::queue_push(); // and this would overflow and panic
    assert_eq!(metrics::EVENTS_QUEUED.load(Relaxed), 1);
    metrics::queue_pop();
    assert_eq!(metrics::EVENTS_QUEUED.load(Relaxed), 0);

    metrics::reset();
}
