use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

mod flags;

static RELOAD_FLAG: AtomicBool = AtomicBool::new(false);
static SHUTDOWN_FLAG: AtomicBool = AtomicBool::new(false);

include!("main_loop.rs");

#[cfg(unix)]
extern "C" fn handle_sighup(_: libc::c_int) {
    RELOAD_FLAG.store(true, Ordering::SeqCst);
}

#[cfg(unix)]
extern "C" fn handle_sigterm(_: libc::c_int) {
    SHUTDOWN_FLAG.store(true, Ordering::SeqCst);
}

#[cfg(unix)]
fn install_signals() {
    unsafe {
        libc::signal(
            libc::SIGHUP,
            handle_sighup as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGTERM,
            handle_sigterm as *const () as libc::sighandler_t,
        );
        libc::signal(libc::SIGPIPE, libc::SIG_IGN);
    }
}

#[cfg(not(unix))]
fn install_signals() {}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: thrawld <MODDIR>");
        std::process::exit(2);
    }
    let moddir = PathBuf::from(&args[1]);
    install_signals();
    let _ = flags::protect_from_oom();
    if let Err(e) = flags::check_and_write_pid(&moddir) {
        eprintln!("[thrawl] pid lock: {}", e);
        std::process::exit(0);
    }
    let cfg_path = PathBuf::from("/data/adb/thrawl/config.conf");
    let effective_path = moddir.join("data/config.effective");
    if let Err(e) = run_daemon(&moddir, &cfg_path, &effective_path) {
        eprintln!("[thrawl] fatal: {}", e);
    }
    flags::remove_pid(&moddir);
}
