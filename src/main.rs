/// rmesg - a rust-based dmesg implementation.
/// This CLI builds on top of the eponymous crate and provides a command-line utility.
///
use clap::{Arg, Command};
use futures_util::stream::StreamExt;
use std::process::ExitCode;

#[derive(Debug)]
struct Options {
    follow: bool,
    clear: bool,
    raw: bool,
    backend: rmesg::Backend,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let opts = parse_args();

    if !opts.follow {
        return nofollow(opts);
    }

    let mut entries = match rmesg::logs_stream(opts.backend, opts.clear, opts.raw).await {
        Ok(entries) => entries,
        Err(e) => return report_error("Unable to get logs stream", e),
    };

    while let Some(result) = entries.next().await {
        match result {
            Ok(entry) => println!("{}", entry),
            Err(e) => return report_error("Unable to get logs stream", e),
        }
    }

    ExitCode::SUCCESS
}

fn nofollow(opts: Options) -> ExitCode {
    if opts.raw {
        match rmesg::logs_raw(opts.backend, opts.clear) {
            Ok(raw) => print!("{}", raw),
            Err(e) => return report_error("Unable to get raw logs", e),
        }
    } else {
        match rmesg::log_entries(opts.backend, opts.clear) {
            Ok(entries) => {
                for entry in entries {
                    println!("{}", entry)
                }
            }
            Err(e) => return report_error("Unable to get log entries", e),
        }
    }

    ExitCode::SUCCESS
}

/// Prints the error (with a privilege hint where relevant) and returns a failing exit code.
fn report_error(context: &str, e: rmesg::error::RMesgError) -> ExitCode {
    eprintln!("{}: {}", context, e);

    if let rmesg::error::RMesgError::OperationNotPermitted(_) = e {
        eprintln!("\nHint: Try using 'sudo' or run the program as root/superuser.");
    }

    ExitCode::FAILURE
}

fn build_command() -> Command {
    Command::new("rmesg: A 'dmesg' port onto Rust")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Archis Gore <me@archisgore.com>")
        .about(
            "Reads (and prints) the kernel log buffer. Does not support all dmesg options (yet).",
        )
        .arg(
            Arg::new("follow")
                .short('f')
                .num_args(0)
                .action(clap::ArgAction::SetTrue)
                .help("When specified, follows logs (like tail -f)"),
        )
        .arg(
            Arg::new("clear")
                .short('c')
                .num_args(0)
                .action(clap::ArgAction::SetTrue)
                .help("Clear ring buffer after printing"),
        )
        .arg(
            Arg::new("raw")
                .short('r')
                .num_args(0)
                .action(clap::ArgAction::SetTrue)
                .help("Print raw data as it came from the source backend."),
        )
        .arg(
            Arg::new("backend")
                .short('b')
                .num_args(1)
                .value_parser(["klogctl", "devkmsg"])
                .help("Select backend from where to read the logs. klogctl is the syslog/klogctl system call through libc. devkmsg is the /dev/kmsg file."),
        )
}

fn options_from(matches: &clap::ArgMatches) -> Options {
    let backend = match matches.get_one::<String>("backend").map(|s| s.as_str()) {
        None => rmesg::Backend::Default,
        Some("klogctl") => rmesg::Backend::KLogCtl,
        Some("devkmsg") => rmesg::Backend::DevKMsg,
        Some(v) => panic!("Something went wrong. Possible values for backend were not restricted by the CLI parser and this value slipped through somehow: {}", v),
    };

    Options {
        follow: matches.get_flag("follow"),
        clear: matches.get_flag("clear"),
        raw: matches.get_flag("raw"),
        backend,
    }
}

fn parse_args() -> Options {
    options_from(&build_command().get_matches())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_args_from(args: &[&str]) -> Options {
        options_from(&build_command().try_get_matches_from(args).unwrap())
    }

    #[test]
    fn test_default_options() {
        let opts = parse_args_from(&["rmesg"]);
        assert!(!opts.follow);
        assert!(!opts.clear);
        assert!(!opts.raw);
        assert!(matches!(opts.backend, rmesg::Backend::Default));
    }

    #[test]
    fn test_follow_flag() {
        let opts = parse_args_from(&["rmesg", "-f"]);
        assert!(opts.follow);
        assert!(!opts.clear);
        assert!(!opts.raw);
        assert!(matches!(opts.backend, rmesg::Backend::Default));
    }

    #[test]
    fn test_clear_flag() {
        let opts = parse_args_from(&["rmesg", "-c"]);
        assert!(!opts.follow);
        assert!(opts.clear);
        assert!(!opts.raw);
        assert!(matches!(opts.backend, rmesg::Backend::Default));
    }

    #[test]
    fn test_raw_flag() {
        let opts = parse_args_from(&["rmesg", "-r"]);
        assert!(!opts.follow);
        assert!(!opts.clear);
        assert!(opts.raw);
        assert!(matches!(opts.backend, rmesg::Backend::Default));
    }

    #[test]
    fn test_klogctl_backend() {
        let opts = parse_args_from(&["rmesg", "-b", "klogctl"]);
        assert!(!opts.follow);
        assert!(!opts.clear);
        assert!(!opts.raw);
        assert!(matches!(opts.backend, rmesg::Backend::KLogCtl));
    }

    #[test]
    fn test_devkmsg_backend() {
        let opts = parse_args_from(&["rmesg", "-b", "devkmsg"]);
        assert!(!opts.follow);
        assert!(!opts.clear);
        assert!(!opts.raw);
        assert!(matches!(opts.backend, rmesg::Backend::DevKMsg));
    }

    #[test]
    fn test_all_flags_together() {
        let opts = parse_args_from(&["rmesg", "-f", "-c", "-r", "-b", "klogctl"]);
        assert!(opts.follow);
        assert!(opts.clear);
        assert!(opts.raw);
        assert!(matches!(opts.backend, rmesg::Backend::KLogCtl));
    }

    #[test]
    #[should_panic]
    fn test_invalid_backend() {
        parse_args_from(&["rmesg", "-b", "invalid"]);
    }

    #[test]
    fn test_version_matches_crate_version() {
        assert_eq!(
            build_command().get_version(),
            Some(env!("CARGO_PKG_VERSION"))
        );
    }

    #[test]
    fn test_backend_help_names_real_values() {
        let help = build_command().render_help().to_string();
        assert!(help.contains("klogctl is the syslog/klogctl system call"));
        assert!(help.contains("devkmsg is the /dev/kmsg file"));
    }
}
