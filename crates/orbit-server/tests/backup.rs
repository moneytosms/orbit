use std::{fs, process::Command};

use rusqlite::Connection;

fn run(data: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_orbit-server"))
        .env("ORBIT_DATA", data)
        .args(args)
        .output()
        .unwrap()
}

fn value(data: &std::path::Path) -> String {
    let conn = Connection::open(data.join("orbit.db")).unwrap();
    conn.query_row("SELECT v FROM t", [], |r| r.get(0)).unwrap()
}

#[test]
fn restore_brings_back_the_snapshotted_row() {
    let tmp = tempfile::tempdir().unwrap();
    let data = tmp.path().join("data");
    let snaps = tmp.path().join("snaps");
    fs::create_dir_all(&data).unwrap();

    let conn = Connection::open(data.join("orbit.db")).unwrap();
    conn.execute_batch("CREATE TABLE t (v TEXT); INSERT INTO t VALUES ('original');")
        .unwrap();
    drop(conn);

    let out = run(&data, &["backup", snaps.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let files: Vec<_> = fs::read_dir(&snaps)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(files.len(), 1, "backup writes one snapshot file");

    let conn = Connection::open(data.join("orbit.db")).unwrap();
    conn.execute_batch("UPDATE t SET v = 'changed';").unwrap();
    drop(conn);
    assert_eq!(value(&data), "changed");

    let out = run(&data, &["restore", files[0].to_str().unwrap()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(value(&data), "original");
}

#[test]
fn backup_without_dir_prints_usage_and_exits_2() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["backup"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("usage: orbit-server backup"));
}
