use orbit_server::backup;
use std::{env, io::BufRead, net::SocketAddr, path::PathBuf, time::Duration};

const KEEP_SNAPSHOTS: usize = 7;

#[tokio::main]
async fn main() {
    let data = PathBuf::from(env::var("ORBIT_DATA").unwrap_or_else(|_| "data".into()));
    if env::args().nth(1).as_deref() == Some("reset-password") {
        eprintln!("New password (one line on stdin):");
        let mut pw = String::new();
        std::io::stdin()
            .lock()
            .read_line(&mut pw)
            .expect("read stdin");
        match orbit_server::reset_password(&data, pw.trim_end_matches(['\r', '\n'])) {
            Ok(()) => eprintln!("Password reset, TOTP off, every Device signed out."),
            Err(e) => {
                eprintln!("reset-password: {e}");
                std::process::exit(1);
            }
        }
        return;
    }
    if env::args().nth(1).as_deref() == Some("backup") {
        let Some(dir) = env::args().nth(2) else {
            eprintln!("usage: orbit-server backup <dir>");
            std::process::exit(2);
        };
        if !data.join("orbit.db").is_file() {
            eprintln!("backup: no database at {}", data.join("orbit.db").display());
            std::process::exit(1);
        }
        match backup::snapshot(&data, &PathBuf::from(&dir), backup::now_ms()) {
            Ok(path) => eprintln!("backup: wrote {}", path.display()),
            Err(e) => {
                eprintln!("backup: {e}");
                std::process::exit(1);
            }
        }
        return;
    }
    if env::args().nth(1).as_deref() == Some("restore") {
        let Some(snapshot) = env::args().nth(2) else {
            eprintln!("usage: orbit-server restore <snapshot>");
            std::process::exit(2);
        };
        eprintln!("restore: stop orbit-server first; it must not be running.");
        if let Err(e) = backup::restore(&data, &PathBuf::from(&snapshot)) {
            eprintln!("restore: {e}");
            std::process::exit(1);
        }
        eprintln!(
            "restore: {snapshot} is now {}",
            data.join("orbit.db").display()
        );
        return;
    }
    let addr = env::var("ORBIT_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".into());
    let (mut app, setup) = orbit_server::app(&data).expect("open data dir");
    if let Ok(web) = env::var("ORBIT_WEB") {
        app = orbit_server::with_web(app, &PathBuf::from(web));
    }
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("bind ORBIT_ADDR");
    eprintln!("orbit-server on {addr}, data in {}", data.display());
    if let Some(code) = setup {
        eprintln!("First run: open the app and enter setup code {code}");
    }
    // Hourly check; a snapshot is taken only when the newest one is over a day old.
    // Blocking loop on the blocking pool: the tokio `time` feature is not enabled.
    let backups = data.join("backups");
    let snapshots_data = data.clone();
    tokio::task::spawn_blocking(move || {
        loop {
            let now = backup::now_ms();
            if backup::due(&backups, now) {
                match backup::snapshot(&snapshots_data, &backups, now) {
                    Ok(path) => eprintln!("backup: wrote {}", path.display()),
                    Err(e) => eprintln!("backup: {e}"),
                }
                if let Err(e) = backup::prune(&backups, KEEP_SNAPSHOTS) {
                    eprintln!("backup prune: {e}");
                }
            }
            std::thread::sleep(Duration::from_secs(60 * 60));
        }
    });
    let app = app.into_make_service_with_connect_info::<SocketAddr>();
    axum::serve(listener, app).await.unwrap();
}
