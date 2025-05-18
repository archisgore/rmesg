/// rmesg - a rust-based dmesg implementation.
/// This CLI builds on top of the eponymous crate and provides a command-line utility.
///
use clap::{Arg, Command};
use futures_util::stream::StreamExt;
use std::error::Error;

#[derive(Debug)]
struct Options {
    follow: bool,
    clear: bool,
    raw: bool,
    backend: rmesg::Backend,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let opts = parse_args();

    if !opts.follow {
        nofollow(opts);
    } else {
        let mut entries = match rmesg::logs_stream(opts.backend, opts.clear, opts.raw).await {
            Ok(entries) => entries,
            Err(e) => {
                eprintln!("Unable to get logs stream: {}", e);

                if let rmesg::error::RMesgError::OperationNotPermitted(_) = e {
                    eprintln!("\nHint: Try using 'sudo' or run the program as root/superuser.");
                }

                return Ok(());
            }
        };

        while let Some(result) = entries.next().await {
            match result {
                Ok(entry) => println!("{}", entry),
                Err(e) => {
                    eprintln!("Unable to get logs stream: {}", e);

                    if let rmesg::error::RMesgError::OperationNotPermitted(_) = e {
                        eprintln!("\nHint: Try using 'sudo' or run the program as root/superuser.");
                    }

                    return Ok(());
                }
            }
        }
    }

    Ok(())
}

fn nofollow(opts: Options) {
    if opts.raw {
        match rmesg::logs_raw(opts.backend, opts.clear) {
            Ok(raw) => {
                print!("{}", raw)
            }
            Err(e) => {
                eprintln!("Unable to get raw logs: {}", e);

                if let rmesg::error::RMesgError::OperationNotPermitted(_) = e {
                    eprintln!("\nHint: Try using 'sudo' or run the program as root/superuser.");
                }
            }
        }
    } else {
        match rmesg::log_entries(opts.backend, opts.clear) {
            Ok(entries) => {
                for entry in entries {
                    println!("{}", entry)
                }
            }
            Err(e) => {
                eprintln!("Unable to get log entries: {}", e);

                if let rmesg::error::RMesgError::OperationNotPermitted(_) = e {
                    eprintln!("\nHint: Try using 'sudo' or run the program as root/superuser.");
                }
            }
        }
    }
}

fn parse_args() -> Options {
    let matches = Command::new("rmesg: A 'dmesg' port onto Rust")
        .version("0.2.0")
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
                .help("Select backend from where to read the logs. klog is the syslog/klogctl system call through libc. kmsg is the /dev/kmsg file."),
        )
        .get_matches();

    let follow = matches.get_flag("follow");
    let clear = matches.get_flag("clear");
    let raw = matches.get_flag("raw");
    let backend = match matches.get_one::<String>("backend").map(|s| s.as_str()) {
        None => rmesg::Backend::Default,
        Some("klogctl") => rmesg::Backend::KLogCtl,
        Some("devkmsg") => rmesg::Backend::DevKMsg,
        Some(v) => panic!("Something went wrong. Possible values for backend were not restricted by the CLI parser and this value slipped through somehow: {}", v),
    };

    Options {
        follow,
        clear,
        raw,
        backend,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_args_from(args: &[&str]) -> Options {
        let cmd = Command::new("rmesg")
            .version("0.2.0")
            .author("Archis Gore <me@archisgore.com>")
            .about("Reads (and prints) the kernel log buffer. Does not support all dmesg options (yet).")
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
                    .help("Select backend from where to read the logs. klog is the syslog/klogctl system call through libc. kmsg is the /dev/kmsg file."),
            );

        let matches = cmd.try_get_matches_from(args).unwrap();

        let follow = matches.get_flag("follow");
        let clear = matches.get_flag("clear");
        let raw = matches.get_flag("raw");
        let backend = match matches.get_one::<String>("backend").map(|s| s.as_str()) {
            None => rmesg::Backend::Default,
            Some("klogctl") => rmesg::Backend::KLogCtl,
            Some("devkmsg") => rmesg::Backend::DevKMsg,
            Some(v) => panic!("Invalid backend value: {}", v),
        };

        Options {
            follow,
            clear,
            raw,
            backend,
        }
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
}
