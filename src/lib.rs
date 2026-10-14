/// Print a line to stdout without panicking when the reader has gone away.
///
/// Rust ignores SIGPIPE, so the standard `println!` panics (exit 101) as soon as
/// a consumer closes the pipe early — `piccolo-gws describe --json | head -1`, or
/// a pager that quits. Whether the output can be delivered is the caller's
/// business; the command's own result is not, so a write failure is ignored here
/// rather than turning a successful run into a panic.
///
/// Piccolo always drains stdout, so this changes nothing for orchestrated runs.
#[macro_export]
macro_rules! println_ok {
    ($($arg:tt)*) => {{
        use ::std::io::Write as _;
        let _ = writeln!(::std::io::stdout(), $($arg)*);
    }};
}

pub const DEFAULT_CONFIG_DIR: &str = "/root/.piccolo/integrations/gws";
pub const DEFAULT_PICOCLAW_CONFIG: &str = "/root/.picoclaw/config.json";

pub mod actions;
pub mod backend;
pub mod config;
pub mod descriptor;
pub mod mcp;
pub mod setup;
pub mod status;
