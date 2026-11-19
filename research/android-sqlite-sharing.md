# Sharing one SQLite outbox between Rust (rusqlite) and Kotlin on Android

## Question

Orbit is a Tauri 2 Android app. Rust (rusqlite, likely `bundled`) and Kotlin (android.database.sqlite or Room) both need to write one SQLite outbox table. Kotlin code runs for a home-screen widget, a share target, and a WorkManager worker, possibly with the UI closed, in the same or a separate process.

1. Can both safely open the same db file concurrently (WAL, locks, same vs separate process)?
2. Do Tauri `app_data_dir` and `Context.getDatabasePath()` reach the same place?
3. Which SQLite versions does each side ship, and are there WAL/format concerns?
4. If unsafe, what is the simplest alternative?

## Answer

Recommendation: do not let Kotlin open the SQLite file. Rust is the only SQLite writer. Kotlin writes one JSON file per outbox record (temp file, fsync, atomic rename) into a directory under the app sandbox. Rust lists that directory on startup and on resume, inserts the records in one transaction with a record-id unique key, then deletes the files.

Summary of findings:
- Same process, two SQLite copies on one file: unsafe (sqlite.org). Widget, share target, and worker run in the app process by default, and rusqlite is a second copy, so the unsafe case applies.
- Tauri `app_data_dir` on Android is `Context.getDataDir()` (`activity.dataDir.absolutePath`, API 24+), with no bundle-id suffix. Kotlin can reach it.
- Android ships SQLite 3.50 at API 37, older on older releases. rusqlite bundles 3.54.1 (master header). Versions differ, so one WAL file should not be shared across both.

## 1. Concurrent open

- Threads in one process sharing one SQLite library and one connection per thread: safe. "it is perfectly safe for two or more threads to access the same SQLite database file using the SQLite library." (https://www.sqlite.org/howtocorrupt.html, section 2.2)
- POSIX advisory locks: "the close() system call will cancel all POSIX advisory locks on the same file for all threads and all file descriptors in the process." Any raw open/close of the db file by any code in the process can drop SQLite's locks. (https://www.sqlite.org/howtocorrupt.html, section 2.2)
- Two SQLite copies in one process: "if multiple copies of SQLite are linked into the same application, then there will be multiple instances of this global list... A close() operation on one connection might unknowingly clear the locks on a different database connection, leading to database corruption." Sqlite.org cites a real commercial product that shipped this bug. (https://www.sqlite.org/howtocorrupt.html, section 2.3)
- 3.51.0 mitigation: "Beginning with SQLite version 3.51.0 (2025-11-04), SQLite implements additional defenses... These new defenses help when the database is in WAL mode and is being accessed from multiple processes. But they are not a cure-all." The same page says developers should "never use close() on an SQLite database file while one or more database connections are open." (https://www.sqlite.org/howtocorrupt.html, section 2.2)
- Android: `android.database.sqlite` is the framework SQLite. rusqlite `bundled` is statically linked into the Rust library. In one process that is two copies. Whether the framework copy is loaded into every app process: UNVERIFIED (see Unverified).
- Cross-process on one device: WAL "requires all processes to share a small amount of memory", and works only when all processes are on the same host with the `-shm` file. "The only way we have found to guarantee that all processes accessing the same database file use the same shared memory is to create the shared memory by mmapping a file in the same directory as the database itself." (https://www.sqlite.org/wal.html, sections 1 and 7). A separate process is therefore workable in principle, but it still means two SQLite builds on one file. Not recommended.
- Android process model: "By default, all of an application's components run in the same process, and most applications don't change this." Widget receivers and share activities run in the app process unless `android:process` is set. (https://developer.android.com/guide/components/processes-and-threads)
- Locking semantics of Android's build vs rusqlite's: both use SQLite's unix VFS in principle. Android build flags: UNVERIFIED.

Conclusion: Kotlin and Rust must not both open this file. A single writer avoids all of the above.

## 2. Path reachability

- Tauri 2 docs: `appDataDir()` "Resolves to `${dataDir}/${bundleIdentifier}`". This is the generic description, not the Android behavior. (https://v2.tauri.app/reference/javascript/api/namespacepath/)
- Tauri source, Android (`crates/tauri/src/path/android.rs`): `app_data_dir` is `self.app_dir(AppDirectory::Data, || self.call_resolve("getDataDir"))`, and `call_resolve` runs the mobile plugin command `getDataDir`. (https://raw.githubusercontent.com/tauri-apps/tauri/dev/crates/tauri/src/path/android.rs)
- Tauri source, `crates/tauri/src/path/mod.rs`: `app_dir` returns the config override if set, otherwise the default closure; no bundle-id join happens here. The Android default is the closure above, so on Android the bundle-id suffix is not applied. (https://raw.githubusercontent.com/tauri-apps/tauri/dev/crates/tauri/src/path/mod.rs). Inference from source, not from docs; pin to the Orbit Tauri version when it exists.
- Tauri Kotlin `PathPlugin.getDataDir`: on API 24+ returns `activity.dataDir.absolutePath`, else `activity.applicationInfo.dataDir`. It does not use `filesDir`. (https://raw.githubusercontent.com/tauri-apps/tauri/dev/crates/tauri/mobile/android/src/main/java/app/tauri/PathPlugin.kt)
- So Tauri `app_data_dir` = Android `Context.getDataDir()` root. Kotlin can reach it via `context.dataDir` (API 24+). `Context.getDataDir()` doc text: UNVERIFIED (fetch truncated before the method).
- `filesDir` and `databases/` are children of that root. The app-specific storage doc says "dataDir is always an ancestor directory of this new directory." (https://developer.android.com/training/data-storage/app-specific). So a Kotlin `filesDir` file is inside the Tauri data dir but not at its root.
- `Context.getDatabasePath(name)`: returns a path under a `databases` location, not the root. Exact path: UNVERIFIED. Irrelevant if Kotlin never opens the db.
- Other apps cannot access internal storage: "Other apps cannot access files stored within internal storage." (https://developer.android.com/training/data-storage/app-specific)

## 3. Versions and format

- Android SQLite by API level: "The version of SQLite depends on the version of Android." API 37 and 36.1: 3.50; API 35: 3.44; API 34: 3.42 / 3.39; API 33/32/31: 3.32; API 30: 3.28; API 28: 3.22. "Some device manufacturers include different versions." (https://developer.android.com/reference/android/database/sqlite/package-summary)
- rusqlite: crate `rusqlite` 0.43.0 depends on `libsqlite3-sys` `^0.40.0` (https://docs.rs/rusqlite/latest/rusqlite/). `libsqlite3-sys` master `sqlite3/sqlite3.h`: `#define SQLITE_VERSION "3.54.1"` (https://raw.githubusercontent.com/rusqlite/rusqlite/master/libsqlite3-sys/sqlite3/sqlite3.h). The 0.40.0 tag fetch returned 404, so the exact bundled version of the released 0.40.0 crate is UNVERIFIED (assumed 3.54.1 from master).
- Gap: rusqlite 3.54.1 vs Android 3.50 or older. Android lacks the 3.51.0 close() defenses (version comparison with the sqlite.org text above; inference).
- Format: "File format write version. 1 for legacy; 2 for WAL." Read and write version bytes 18/19. Both sides use format 2 today. A future write version >2 makes the file read-only for current readers; read version >2 makes it unusable. (https://www.sqlite.org/fileformat2.html, section 1.3.3)
- Recovery across versions: "older versions of SQLite will not know how to recover a crashed SQLite database that was operating in WAL mode... it will report an error along the lines of 'file is encrypted or is not a database'." (https://www.sqlite.org/wal.html, section 10). Mixed-version access to one WAL file is a real failure mode, not only a theoretical one.

## 4. Options

(a) Kotlin-only outbox file, drained by Rust. Same as (c) below; picked as the concrete form.

(b) Kotlin calls Rust via JNI or a Tauri mobile plugin. Tauri plugin Kotlin classes extend `app.tauri.plugin.Plugin` and are called from Rust via `run_mobile_plugin` (https://v2.tauri.app/develop/plugins/develop-mobile/). Rejected: the widget and worker would need the Rust library loaded, and the example loads it with `System.loadLibrary` in the activity-side plugin code. Whether a cold WorkManager process has it loaded: UNVERIFIED. Adds coupling for no gain over (c).

(c) Rust sole writer; Kotlin writes records as files. Picked.
- No SQLite on the Kotlin side: no shared lock, no two copies, no version split.
- No fcntl-style locks shared across the two sides. POSIX advisory locks are per process and any `close()` on the file drops them (https://www.sqlite.org/howtocorrupt.html, section 2.2). Avoid cross-language file locks.
- One file per record, written with temp + atomic rename, so Rust never reads a partial record. Rename atomicity on Android internal storage: UNVERIFIED.
- Rust dedupes on a record id (`INSERT OR IGNORE` on a unique key) so a crash between import and delete does not duplicate.

Trade-off: records written while the app is closed wait until Rust drains them. If the worker must send over the network with the UI closed, that needs option (b) or a Kotlin HTTP path. Decision for the user.

## Unverified

1. The Android framework SQLite library is loaded into every app process (so rusqlite and it share one process). Not found in a primary source.
2. `android.database.sqlite` and rusqlite use the same POSIX advisory lock semantics on Android (Android build flags not checked).
3. `Context.getDataDir()` method description. The Context reference fetch truncated before this method.
4. `Context.getDatabasePath(name)` exact directory. Same cause.
5. Exact bundled SQLite version of released `libsqlite3-sys` 0.40.0 (only master header read; tag fetch 404).
6. WorkManager work runs in the app process by default (WorkManager doc is silent on process).
7. A cold WorkManager process has the Tauri native library loaded (option b feasibility).
8. POSIX `rename()` atomicity on Android internal storage (option c correctness).

Also not checked: the Orbit repo has no `Cargo.toml` or Gradle files on `main` (`git ls-tree` found none), so the rusqlite and Tauri versions are the stated stack, not measured versions. Next: pin versions once `Cargo.lock` and `build.gradle` exist.

## Sources

- https://www.sqlite.org/howtocorrupt.html (sections 2.2, 2.3)
- https://www.sqlite.org/wal.html (sections 1, 4, 7, 10)
- https://www.sqlite.org/lockingv3.html (fetched; the close() rule is not on this page)
- https://www.sqlite.org/threadsafe.html
- https://www.sqlite.org/atomiccommit.html
- https://www.sqlite.org/fileformat2.html (section 1.3.3, header bytes 18/19)
- https://developer.android.com/reference/android/database/sqlite/package-summary
- https://developer.android.com/reference/android/database/sqlite/SQLiteDatabase#enableWriteAheadLogging()
- https://developer.android.com/training/data-storage/sqlite
- https://developer.android.com/training/data-storage/app-specific
- https://developer.android.com/guide/components/processes-and-threads
- https://developer.android.com/develop/background-work/background-tasks/persistent/getting-started
- https://v2.tauri.app/reference/javascript/api/namespacepath/
- https://v2.tauri.app/develop/plugins/develop-mobile/
- https://raw.githubusercontent.com/tauri-apps/tauri/dev/crates/tauri/src/path/android.rs
- https://raw.githubusercontent.com/tauri-apps/tauri/dev/crates/tauri/src/path/mod.rs
- https://raw.githubusercontent.com/tauri-apps/tauri/dev/crates/tauri/mobile/android/src/main/java/app/tauri/PathPlugin.kt
- https://docs.rs/rusqlite/latest/rusqlite/
- https://raw.githubusercontent.com/rusqlite/rusqlite/master/libsqlite3-sys/Cargo.toml
- https://raw.githubusercontent.com/rusqlite/rusqlite/master/libsqlite3-sys/sqlite3/sqlite3.h
