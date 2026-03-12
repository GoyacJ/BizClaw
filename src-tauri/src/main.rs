#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::var_os("BIZCLAW_SSH_ASKPASS").is_some() {
        match std::env::var("BIZCLAW_SSH_PASSWORD") {
            Ok(password) => {
                use std::io::Write;

                print!("{password}");
                let _ = std::io::stdout().flush();
                return;
            }
            Err(_) => std::process::exit(1),
        }
    }

    bizclaw_lib::run();
}
