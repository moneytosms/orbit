use std::{env, io::BufRead, net::SocketAddr, path::PathBuf};

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
    let addr = env::var("ORBIT_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".into());
    let (app, setup) = orbit_server::app(&data).expect("open data dir");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("bind ORBIT_ADDR");
    eprintln!("orbit-server on {addr}, data in {}", data.display());
    if let Some(code) = setup {
        eprintln!("First run: open the app and enter setup code {code}");
    }
    let app = app.into_make_service_with_connect_info::<SocketAddr>();
    axum::serve(listener, app).await.unwrap();
}
