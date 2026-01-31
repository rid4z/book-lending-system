use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePool},
    Executor,
};

pub async fn get_db_pool() -> SqlitePool {
    let options = SqliteConnectOptions::new()
        .filename("test.db")
        .create_if_missing(true);

    let pool = SqlitePool::connect_with(options)
        .await
        .expect("Failed to connect to DB");

    // Enable foreign keys (off by default in SQLite)
    pool.execute("PRAGMA foreign_keys = ON")
        .await
        .unwrap();

    // Users
    pool.execute(
        "
        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT UNIQUE NOT NULL,
            password TEXT NOT NULL,
            role TEXT NOT NULL
        )
        ",
    )
    .await
    .unwrap();

    // Books
    pool.execute(
        "
        CREATE TABLE IF NOT EXISTS books (
            bookid INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            author TEXT NOT NULL,
            isbn TEXT UNIQUE NOT NULL,
            year_of_pub INTEGER,
            genre TEXT,
            total_copies INTEGER NOT NULL DEFAULT 0,
            available_copies INTEGER NOT NULL DEFAULT 0
        )
        ",
    )
    .await
    .unwrap();

    // Loans
    pool.execute(
        "
        CREATE TABLE IF NOT EXISTS loans (
            loanid INTEGER PRIMARY KEY AUTOINCREMENT,
            loaned_to_user_id INTEGER NOT NULL,
            loaned_bookid INTEGER NOT NULL,
            checkout_date TEXT NOT NULL,
            due_date TEXT NOT NULL,
            return_date TEXT,
            FOREIGN KEY(loaned_to_user_id) REFERENCES users(id),
            FOREIGN KEY(loaned_bookid) REFERENCES books(bookid)
        )
        ",
    )
    .await
    .unwrap();

    // Sessions
    pool.execute(
        "
        CREATE TABLE IF NOT EXISTS sessions (
            token TEXT PRIMARY KEY,
            username TEXT NOT NULL,
            role TEXT NOT NULL,
            expires_at TEXT NOT NULL
        )
        ",
    )
    .await
    .unwrap();

    pool
}