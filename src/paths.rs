use std::path::PathBuf;

use directories::ProjectDirs;

fn project_dirs() -> ProjectDirs {
    ProjectDirs::from("dev", "local-ticker-wallet", "local-ticker-wallet")
        .expect("cannot resolve home directory")
}

pub fn data_db() -> PathBuf {
    let dir = project_dirs().data_dir().to_path_buf();
    std::fs::create_dir_all(&dir).ok();
    dir.join("wallet.duckdb")
}

pub fn config_file() -> PathBuf {
    let dir = project_dirs().config_dir().to_path_buf();
    std::fs::create_dir_all(&dir).ok();
    dir.join("config.json")
}

pub fn socket_path() -> PathBuf {
    if let Ok(rt) = std::env::var("XDG_RUNTIME_DIR") {
        return PathBuf::from(rt).join("local-ticker-wallet.sock");
    }
    std::env::temp_dir().join("local-ticker-wallet.sock")
}
