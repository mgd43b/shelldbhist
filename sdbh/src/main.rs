mod cleanup;
mod cli;
mod config;
mod db;
mod domain;
mod template;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    reset_sigpipe()?;
    let cli = cli::Cli::parse();
    cli::run(cli)
}

/// Restore the default `SIGPIPE` disposition (`SIG_DFL`).
///
/// Rust installs `SIG_IGN` for `SIGPIPE` at startup, which turns a closed
/// downstream pipe (e.g. `sdbh export | head`, or `sdbha | grep …` where grep
/// exits early) into an `EPIPE` error that the `println!` machinery unwraps —
/// panicking with "failed printing to stdout: Broken pipe". Restoring `SIG_DFL`
/// makes the process terminate quietly on `SIGPIPE`, matching standard Unix
/// filters like `grep`/`cat`.
#[cfg(unix)]
fn reset_sigpipe() -> Result<()> {
    // SAFETY: called once at the very start of `main`, before any other threads
    // exist; `signal(2)` with `SIG_DFL` is async-signal-safe.
    let prev = unsafe { libc::signal(libc::SIGPIPE, libc::SIG_DFL) };
    sigpipe_result(prev)
}

/// Map the return value of `signal(2)` to a `Result`, erroring on `SIG_ERR`.
///
/// Split out from [`reset_sigpipe`] so the (practically unreachable) failure
/// path can be unit-tested without having to make a real `signal` call fail.
#[cfg(unix)]
fn sigpipe_result(prev: libc::sighandler_t) -> Result<()> {
    if prev == libc::SIG_ERR {
        // Surface it loudly rather than silently continuing with the
        // panic-prone SIG_IGN default that Rust installs.
        return Err(anyhow::anyhow!(
            "failed to restore default SIGPIPE handler: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn reset_sigpipe() -> Result<()> {
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::sigpipe_result;

    #[test]
    fn sigpipe_result_ok_for_valid_previous_handler() {
        // SIG_DFL is a normal previous-handler value, not an error sentinel.
        assert!(sigpipe_result(libc::SIG_DFL).is_ok());
    }

    #[test]
    fn sigpipe_result_errors_on_sig_err() {
        assert!(sigpipe_result(libc::SIG_ERR).is_err());
    }
}
