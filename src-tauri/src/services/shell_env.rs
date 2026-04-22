use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

static LOGIN_SHELL_PATH: OnceLock<Option<OsString>> = OnceLock::new();

const PATH_START_MARKER: &str = "__CCSWITCH_PATH_START__";
const PATH_END_MARKER: &str = "__CCSWITCH_PATH_END__";

pub fn apply_resolved_path(command: &mut Command) {
    if let Some(path) = resolved_path_env() {
        command.env("PATH", path);
    }
}

pub fn resolved_path_env() -> Option<OsString> {
    let merged = build_resolved_path_entries(
        env::var_os("PATH"),
        login_shell_path(),
        default_search_paths(),
    );

    if merged.is_empty() {
        None
    } else {
        env::join_paths(merged).ok()
    }
}

pub fn build_resolved_path_entries(
    current_path: Option<OsString>,
    login_shell_path: Option<OsString>,
    extra_paths: Vec<PathBuf>,
) -> Vec<PathBuf> {
    let mut merged = Vec::new();

    extend_unique_paths(&mut merged, current_path);
    extend_unique_paths(&mut merged, login_shell_path);

    for path in extra_paths {
        push_unique_path(&mut merged, path);
    }

    merged
}

fn login_shell_path() -> Option<OsString> {
    LOGIN_SHELL_PATH.get_or_init(read_login_shell_path).clone()
}

fn read_login_shell_path() -> Option<OsString> {
    #[cfg(target_os = "windows")]
    {
        None
    }

    #[cfg(not(target_os = "windows"))]
    {
        let shell = preferred_shell();
        for flag in login_shell_flags(&shell) {
            let output = Command::new(&shell)
                .arg(flag)
                .arg(format!(
                    "printf '{PATH_START_MARKER}%s{PATH_END_MARKER}' \"$PATH\""
                ))
                .output();

            if let Ok(output) = output {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Some(path) = extract_marked_path(&stdout) {
                    return Some(OsString::from(path));
                }
            }
        }

        None
    }
}

#[cfg(not(target_os = "windows"))]
fn preferred_shell() -> String {
    let env_shell = env::var("SHELL").ok().filter(|shell| {
        let path = Path::new(shell);
        path.is_absolute() && path.exists()
    });

    if let Some(shell) = env_shell {
        shell
    } else if cfg!(target_os = "macos") {
        "/bin/zsh".to_string()
    } else if Path::new("/bin/bash").exists() {
        "/bin/bash".to_string()
    } else {
        "/bin/sh".to_string()
    }
}

#[cfg(not(target_os = "windows"))]
fn login_shell_flags(shell: &str) -> &'static [&'static str] {
    match Path::new(shell)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("sh")
    {
        "fish" => &["-lc", "-c"],
        "sh" | "dash" => &["-lc", "-c"],
        _ => &["-lic", "-lc", "-c"],
    }
}

fn default_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let home = dirs::home_dir().unwrap_or_default();

    if !home.as_os_str().is_empty() {
        for suffix in [
            ".local/bin",
            ".npm-global/bin",
            "n/bin",
            ".volta/bin",
            ".bun/bin",
            ".opencode/bin",
            "go/bin",
            "bin",
        ] {
            push_unique_path(&mut paths, home.join(suffix));
        }

        extend_node_version_dirs(&mut paths, &home.join(".nvm/versions/node"));
        extend_node_version_dirs(&mut paths, &home.join(".local/state/fnm_multishells"));
    }

    #[cfg(target_os = "macos")]
    for candidate in ["/opt/homebrew/bin", "/usr/local/bin"] {
        push_unique_path(&mut paths, PathBuf::from(candidate));
    }

    #[cfg(target_os = "linux")]
    for candidate in ["/usr/local/bin", "/usr/bin"] {
        push_unique_path(&mut paths, PathBuf::from(candidate));
    }

    paths
}

fn extend_node_version_dirs(paths: &mut Vec<PathBuf>, base: &Path) {
    if !base.exists() {
        return;
    }

    if let Ok(entries) = std::fs::read_dir(base) {
        for entry in entries.flatten() {
            let bin_path = entry.path().join("bin");
            if bin_path.exists() {
                push_unique_path(paths, bin_path);
            }
        }
    }
}

fn push_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if path.as_os_str().is_empty() {
        return;
    }

    if !paths.iter().any(|existing| existing == &path) {
        paths.push(path);
    }
}

fn extend_unique_paths(paths: &mut Vec<PathBuf>, value: Option<OsString>) {
    if let Some(value) = value {
        for path in env::split_paths(&value) {
            push_unique_path(paths, path);
        }
    }
}

fn extract_marked_path(output: &str) -> Option<String> {
    let start = output.find(PATH_START_MARKER)?;
    let tail = &output[start + PATH_START_MARKER.len()..];
    let end = tail.find(PATH_END_MARKER)?;
    let value = tail[..end].trim();

    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;

    use super::{build_resolved_path_entries, extract_marked_path};

    #[test]
    fn merges_current_login_and_extra_paths_without_duplicates() {
        let current = Some(OsString::from("/usr/bin:/bin:/opt/homebrew/bin"));
        let login = Some(OsString::from(
            "/opt/homebrew/bin:/Users/tester/.nvm/versions/node/v22.0.0/bin:/usr/bin",
        ));
        let extra = vec![
            PathBuf::from("/Users/tester/.local/bin"),
            PathBuf::from("/opt/homebrew/bin"),
        ];

        let merged = build_resolved_path_entries(current, login, extra);

        assert_eq!(merged[0], PathBuf::from("/usr/bin"));
        assert_eq!(merged[1], PathBuf::from("/bin"));
        assert_eq!(merged[2], PathBuf::from("/opt/homebrew/bin"));
        assert!(merged.contains(&PathBuf::from(
            "/Users/tester/.nvm/versions/node/v22.0.0/bin"
        )));
        assert!(merged.contains(&PathBuf::from("/Users/tester/.local/bin")));

        let brew_count = merged
            .iter()
            .filter(|path| **path == PathBuf::from("/opt/homebrew/bin"))
            .count();
        assert_eq!(brew_count, 1);
    }

    #[test]
    fn extracts_marked_path_even_when_shell_prints_extra_output() {
        let output = "welcome\n__CCSWITCH_PATH_START__/opt/homebrew/bin:/usr/bin__CCSWITCH_PATH_END__\n";

        assert_eq!(
            extract_marked_path(output).as_deref(),
            Some("/opt/homebrew/bin:/usr/bin")
        );
    }
}
