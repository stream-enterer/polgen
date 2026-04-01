// Port of C++ emMain (IPC server + window factory).

use emcore::emMiniIpc::emMiniIpcClient;

/// Compute the IPC server name for single-instance coordination.
///
/// Port of C++ `emMain::CalcServerName`.
/// Derives a unique name from the hostname and DISPLAY environment variable.
pub fn CalcServerName() -> String {
    let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string());
    let hostname = get_hostname();
    format!(
        "eaglemode_on_{}_{}",
        hostname,
        display.replace([':', '.'], "_")
    )
}

/// Read the system hostname via /etc/hostname, falling back to "localhost".
fn get_hostname() -> String {
    std::fs::read_to_string("/etc/hostname")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "localhost".to_string())
}

/// Try to send a command to an already-running instance via IPC.
///
/// Port of C++ single-instance coordination using emMiniIpc::TrySend.
pub fn try_ipc_client(server_name: &str, visit: Option<&str>) -> bool {
    let mut args: Vec<&str> = vec!["NewWindow"];
    if let Some(v) = visit {
        args.push("-visit");
        args.push(v);
    }
    match emMiniIpcClient::TrySend(server_name, &args) {
        Ok(()) => {
            log::info!("IPC: sent NewWindow to existing instance");
            true
        }
        Err(e) => {
            log::debug!("IPC: no existing instance ({e})");
            false
        }
    }
}

/// IPC server + window factory engine.
///
/// Port of C++ `emMain`.
pub struct emMain {
    server_name: String,
}

impl emMain {
    pub fn new(serve: bool) -> Self {
        let server_name = CalcServerName();
        if serve {
            log::info!("IPC server name: {server_name}");
        }
        Self { server_name }
    }

    pub fn on_reception(&self, args: &[String]) {
        if args.is_empty() {
            log::warn!("emMain: empty IPC message");
            return;
        }
        match args[0].as_str() {
            "NewWindow" => {
                log::info!("emMain: received NewWindow command");
                // Full wiring in Phase 3 when startup engine is ported.
            }
            "ReloadFiles" => {
                log::info!("emMain: received ReloadFiles command");
            }
            _ => {
                let joined: String = args.join(" ");
                log::warn!("emMain: illegal MiniIpc request: {joined}");
            }
        }
    }

    pub fn server_name(&self) -> &str {
        &self.server_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_server_name() {
        let name = CalcServerName();
        assert!(name.starts_with("eaglemode_on_"));
        // Should contain no colons or dots (they're replaced)
        let suffix = &name["eaglemode_on_".len()..];
        assert!(!suffix.contains(':'));
        assert!(!suffix.contains('.'));
    }

    #[test]
    fn test_get_hostname_non_empty() {
        let h = get_hostname();
        assert!(!h.is_empty());
    }

    #[test]
    fn test_try_ipc_client_no_server() {
        // Should return false when no server is running
        assert!(!try_ipc_client("nonexistent_test_server_12345", None));
        assert!(!try_ipc_client("nonexistent_test_server_12345", Some("/home")));
    }

    #[test]
    fn test_emMain_new() {
        let em = emMain::new(false);
        assert!(em.server_name().starts_with("eaglemode_on_"));
    }

    #[test]
    fn test_emMain_on_reception_new_window() {
        let em = emMain::new(false);
        // Should not panic
        em.on_reception(&["NewWindow".to_string()]);
        em.on_reception(&[
            "NewWindow".to_string(),
            "-visit".to_string(),
            "/home".to_string(),
        ]);
    }

    #[test]
    fn test_emMain_on_reception_empty() {
        let em = emMain::new(false);
        // Should not panic on empty args
        em.on_reception(&[]);
    }

    #[test]
    fn test_emMain_on_reception_reload_files() {
        let em = emMain::new(false);
        em.on_reception(&["ReloadFiles".to_string()]);
    }

    #[test]
    fn test_emMain_on_reception_unknown() {
        let em = emMain::new(false);
        em.on_reception(&["UnknownCommand".to_string(), "arg1".to_string()]);
    }
}
