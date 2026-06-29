use std::{env, fmt, fs, path::PathBuf};

#[derive(Clone)]
pub struct RiotLockfile {
    pub name: String,
    pub pid: u32,
    pub port: u16,
    pub password: String,
    pub protocol: String,
    pub path: PathBuf,
}

impl fmt::Debug for RiotLockfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RiotLockfile")
            .field("name", &self.name)
            .field("pid", &self.pid)
            .field("port", &self.port)
            .field("password", &"<redacted>")
            .field("protocol", &self.protocol)
            .field("path", &self.path)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct LockfilePaths {
    pub riot_installs_path: Option<PathBuf>,
    pub lockfile_path: Option<PathBuf>,
}

pub fn default_paths() -> LockfilePaths {
    let riot_installs_path = env::var_os("PROGRAMDATA").map(|root| {
        PathBuf::from(root)
            .join("Riot Games")
            .join("RiotClientInstalls.json")
    });

    let lockfile_path = env::var_os("LOCALAPPDATA").map(|root| {
        PathBuf::from(root)
            .join("Riot Games")
            .join("Riot Client")
            .join("Config")
            .join("lockfile")
    });

    LockfilePaths {
        riot_installs_path,
        lockfile_path,
    }
}

impl RiotLockfile {
    pub fn read_from_path(path: PathBuf) -> Result<Self, String> {
        let raw = fs::read_to_string(&path)
            .map_err(|err| format!("failed to read Riot lockfile: {err}"))?;
        Self::parse(path, raw.trim())
    }

    pub fn parse(path: PathBuf, raw: &str) -> Result<Self, String> {
        let parts: Vec<&str> = raw.split(':').collect();
        if parts.len() != 5 {
            return Err("Riot lockfile has an unexpected format".to_string());
        }

        let pid = parts[1]
            .parse::<u32>()
            .map_err(|_| "Riot lockfile PID is invalid".to_string())?;
        if pid == 0 {
            return Err("Riot lockfile PID is zero".to_string());
        }

        let port = parts[2]
            .parse::<u16>()
            .map_err(|_| "Riot lockfile port is invalid".to_string())?;
        if port == 0 {
            return Err("Riot lockfile port is zero".to_string());
        }

        let protocol = parts[4].to_string();
        if protocol != "http" && protocol != "https" {
            return Err("Riot lockfile protocol is invalid".to_string());
        }

        Ok(Self {
            name: parts[0].to_string(),
            pid,
            port,
            password: parts[3].to_string(),
            protocol,
            path,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::RiotLockfile;
    use std::path::PathBuf;

    #[test]
    fn parses_valid_lockfile() {
        let parsed = RiotLockfile::parse(
            PathBuf::from("lockfile"),
            "Riot Client:1234:5678:secret:https",
        )
        .expect("lockfile should parse");

        assert_eq!(parsed.name, "Riot Client");
        assert_eq!(parsed.pid, 1234);
        assert_eq!(parsed.port, 5678);
        assert_eq!(parsed.protocol, "https");
        assert_eq!(parsed.password, "secret");
    }

    #[test]
    fn rejects_bad_format() {
        let parsed = RiotLockfile::parse(PathBuf::from("lockfile"), "Riot Client:1234");
        assert!(parsed.is_err());
    }

    #[test]
    fn debug_redacts_password() {
        let parsed = RiotLockfile::parse(
            PathBuf::from("lockfile"),
            "Riot Client:1234:5678:secret:https",
        )
        .expect("lockfile should parse");

        let debug = format!("{parsed:?}");
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("secret"));
    }
}
