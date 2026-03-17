use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedCommand {
    pub program: String,
    pub args: Vec<String>,
}

pub fn prepare_command(
    program: &str,
    args: &[String],
    is_windows: bool,
    resolved_path: Option<PathBuf>,
    comspec: Option<&str>,
) -> PreparedCommand {
    let resolved_path = resolved_path.unwrap_or_else(|| PathBuf::from(program));

    if !is_windows {
        return PreparedCommand {
            program: display_program(&resolved_path, program),
            args: args.to_vec(),
        };
    }

    let extension = resolved_path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase());

    if matches!(extension.as_deref(), Some("cmd" | "bat")) {
        let _ = comspec;
        return PreparedCommand {
            program: display_program(&resolved_path, program),
            args: args.to_vec(),
        };
    }

    PreparedCommand {
        program: display_program(&resolved_path, program),
        args: args.to_vec(),
    }
}

pub fn prepare_command_for_current_platform(program: &str, args: &[String]) -> PreparedCommand {
    prepare_command(
        program,
        args,
        cfg!(windows),
        which::which(program).ok(),
        env::var("COMSPEC").ok().as_deref(),
    )
}

pub fn new_command(program: &str, args: &[String]) -> Command {
    let prepared = prepare_command_for_current_platform(program, args);
    let mut command = Command::new(&prepared.program);
    command.args(&prepared.args);
    command
}

fn display_program(path: &Path, fallback: &str) -> String {
    path.to_str()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| fallback.to_string())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{prepare_command, PreparedCommand};

    #[test]
    fn windows_batch_wrappers_spawn_directly() {
        let prepared = prepare_command(
            "openclaw",
            &["gateway".into(), "status".into(), "--json".into()],
            true,
            Some(PathBuf::from(
                r"C:\Users\goya\AppData\Roaming\npm\openclaw.cmd",
            )),
            Some(r"C:\Windows\System32\cmd.exe"),
        );

        assert_eq!(
            prepared,
            PreparedCommand {
                program: r"C:\Users\goya\AppData\Roaming\npm\openclaw.cmd".into(),
                args: vec!["gateway".into(), "status".into(), "--json".into()],
            }
        );
    }

    #[test]
    fn windows_native_binaries_still_spawn_directly() {
        let prepared = prepare_command(
            "ssh",
            &["-V".into()],
            true,
            Some(PathBuf::from(r"C:\Windows\System32\OpenSSH\ssh.exe")),
            Some(r"C:\Windows\System32\cmd.exe"),
        );

        assert_eq!(
            prepared,
            PreparedCommand {
                program: r"C:\Windows\System32\OpenSSH\ssh.exe".into(),
                args: vec!["-V".into()],
            }
        );
    }

    #[test]
    fn non_windows_commands_are_left_unchanged() {
        let prepared = prepare_command(
            "openclaw",
            &["--version".into()],
            false,
            Some(PathBuf::from("/usr/local/bin/openclaw")),
            None,
        );

        assert_eq!(
            prepared,
            PreparedCommand {
                program: "/usr/local/bin/openclaw".into(),
                args: vec!["--version".into()],
            }
        );
    }
}
