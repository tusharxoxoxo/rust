//! rustbuild, the Rust build system
//!
//! This is the entry point for the build system used to compile the `rustc`
//! compiler. Lots of documentation can be found in the `README.md` file in the
//! parent directory, and otherwise documentation can be found throughout the `build`
//! directory in each respective module.

use std::io::Write;
#[cfg(all(any(unix, windows), not(target_os = "solaris")))]
use std::process;
use std::{
    env,
    fs::{self, OpenOptions},
    io::{self, BufRead, BufReader, IsTerminal},
};

use bootstrap::{
    find_recent_config_change_ids, t, Build, Config, Subcommand, CONFIG_CHANGE_HISTORY,
};

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let config = Config::parse(&args);

    #[cfg(all(any(unix, windows), not(target_os = "solaris")))]
    let mut build_lock;
    #[cfg(all(any(unix, windows), not(target_os = "solaris")))]
    let _build_lock_guard;

    if !config.bypass_bootstrap_lock {
        // Display PID of process holding the lock
        // PID will be stored in a lock file
        #[cfg(all(any(unix, windows), not(target_os = "solaris")))]
        {
            let path = config.out.join("lock");
            let pid = match fs::read_to_string(&path) {
                Ok(contents) => contents,
                Err(_) => String::new(),
            };

            build_lock = fd_lock::RwLock::new(t!(fs::OpenOptions::new()
                .write(true)
                .create(true)
                .open(&path)));
            _build_lock_guard = match build_lock.try_write() {
                Ok(mut lock) => {
                    t!(lock.write(&process::id().to_string().as_ref()));
                    lock
                }
                err => {
                    drop(err);
                    println!("WARNING: build directory locked by process {pid}, waiting for lock");
                    let mut lock = t!(build_lock.write());
                    t!(lock.write(&process::id().to_string().as_ref()));
                    lock
                }
            };
        }

        #[cfg(any(not(any(unix, windows)), target_os = "solaris"))]
        println!("WARNING: file locking not supported for target, not locking build directory");
    }

    // check_version warnings are not printed during setup
    let changelog_suggestion =
        if matches!(config.cmd, Subcommand::Setup { .. }) { None } else { check_version(&config) };

    // NOTE: Since `./configure` generates a `config.toml`, distro maintainers will see the
    // changelog warning, not the `x.py setup` message.
    let suggest_setup = config.config.is_none() && !matches!(config.cmd, Subcommand::Setup { .. });
    if suggest_setup {
        println!("WARNING: you have not made a `config.toml`");
        println!(
            "HELP: consider running `./x.py setup` or copying `config.example.toml` by running \
            `cp config.example.toml config.toml`"
        );
    } else if let Some(suggestion) = &changelog_suggestion {
        println!("{suggestion}");
    }

    let pre_commit = config.src.join(".git").join("hooks").join("pre-commit");
    let dump_bootstrap_shims = config.dump_bootstrap_shims;
    let out_dir = config.out.clone();

    Build::new(config).build();

    if suggest_setup {
        println!("WARNING: you have not made a `config.toml`");
        println!(
            "HELP: consider running `./x.py setup` or copying `config.example.toml` by running \
            `cp config.example.toml config.toml`"
        );
    } else if let Some(suggestion) = &changelog_suggestion {
        println!("{suggestion}");
    }

    // Give a warning if the pre-commit script is in pre-commit and not pre-push.
    // HACK: Since the commit script uses hard links, we can't actually tell if it was installed by x.py setup or not.
    // We could see if it's identical to src/etc/pre-push.sh, but pre-push may have been modified in the meantime.
    // Instead, look for this comment, which is almost certainly not in any custom hook.
    if fs::read_to_string(pre_commit).map_or(false, |contents| {
        contents.contains("https://github.com/rust-lang/rust/issues/77620#issuecomment-705144570")
    }) {
        println!(
            "WARNING: You have the pre-push script installed to .git/hooks/pre-commit. \
                  Consider moving it to .git/hooks/pre-push instead, which runs less often."
        );
    }

    if suggest_setup || changelog_suggestion.is_some() {
        println!("NOTE: this message was printed twice to make it more likely to be seen");
    }

    if dump_bootstrap_shims {
        let dump_dir = out_dir.join("bootstrap-shims-dump");
        assert!(dump_dir.exists());

        for entry in walkdir::WalkDir::new(&dump_dir) {
            let entry = t!(entry);

            if !entry.file_type().is_file() {
                continue;
            }

            let file = t!(fs::File::open(&entry.path()));

            // To ensure deterministic results we must sort the dump lines.
            // This is necessary because the order of rustc invocations different
            // almost all the time.
            let mut lines: Vec<String> = t!(BufReader::new(&file).lines().collect());
            lines.sort_by_key(|t| t.to_lowercase());
            let mut file = t!(OpenOptions::new().write(true).truncate(true).open(&entry.path()));
            t!(file.write_all(lines.join("\n").as_bytes()));
        }
    }
}

fn check_version(config: &Config) -> Option<String> {
    let mut msg = String::new();

    if config.changelog_seen.is_some() {
        msg.push_str("WARNING: The use of `changelog-seen` is deprecated. Please refer to `change-id` option in `config.example.toml` instead.\n");
    }

    let latest_change_id = CONFIG_CHANGE_HISTORY.last().unwrap().change_id;
    let warned_id_path = config.out.join("bootstrap").join(".last-warned-change-id");

    if let Some(id) = config.change_id {
        if id == latest_change_id {
            return None;
        }

        if let Ok(last_warned_id) = fs::read_to_string(&warned_id_path) {
            if latest_change_id.to_string() == last_warned_id {
                return None;
            }
        }

        let changes = find_recent_config_change_ids(id);

        if !changes.is_empty() {
            msg.push_str("There have been changes to x.py since you last updated:\n");

            for change in changes {
                msg.push_str(&format!("  [{}] {}\n", change.severity.to_string(), change.summary));
                msg.push_str(&format!(
                    "    - PR Link https://github.com/rust-lang/rust/pull/{}\n",
                    change.change_id
                ));
            }

            msg.push_str("NOTE: to silence this warning, ");
            msg.push_str(&format!(
                "update `config.toml` to use `change-id = {latest_change_id}` instead"
            ));

            if io::stdout().is_terminal() {
                t!(fs::write(warned_id_path, latest_change_id.to_string()));
            }
        }
    } else {
        msg.push_str("WARNING: The `change-id` is missing in the `config.toml`. This means that you will not be able to track the major changes made to the bootstrap configurations.\n");
        msg.push_str("NOTE: to silence this warning, ");
        msg.push_str(&format!("add `change-id = {latest_change_id}` at the top of `config.toml`"));
    };

    Some(msg)
}
