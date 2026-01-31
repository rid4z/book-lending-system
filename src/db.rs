use sqlx::{
    prelude::FromRow,
    sqlite::{SqliteConnectOptions, SqlitePool},
    Executor,
};
use chrono::{Utc, Duration};

pub async fn get_db_pool() -> SqlitePool {
    let options = SqliteConnectOptions::new()
        .filename("test.db")
        .create_if_missing(true);

    let pool = SqlitePool::connect_with(options)
        .await
        .expect("Failed to connect to DB");

    // Users
    pool.execute(
        "
        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT UNIQUE,
            password TEXT,
            role TEXT
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
            title TEXT, 
            author TEXT,
            isbn TEXT,
            year_of_pub INTEGER,
            genre TEXT,
            total_copies INTEGER,
            available_copies INTEGER,
            status TEXT
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
            loaned_to_user_id INTEGER, 
            loaned_bookid INTEGER,
            checkout_date TEXT,
            due_date TEXT,
            return_date TEXT,
            status TEXT,
            FOREIGN KEY(loaned_to_user_id) REFERENCES users(id),
            FOREIGN KEY(loaned_bookid) REFERENCES books(bookid)
        )
        ",
    )
    .await
    .unwrap();

    pool
}



//     sqlx::query("INSERT INTO users (username, password, role) VALUES (?1, ?2, ?3)")
//     .bind("lender1")
//     .bind("lenderpass")
//     .bind("lender")
//     .execute(&connection)
//     .await
//     .unwrap();

//     sqlx::query("INSERT INTO books (title, author, isbn, year_of_pub, genre, total_copies, available_copies, status) 
// VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)")
//     .bind("The Rust Programming Language")
//     .bind("Steve Klabnik")
//     .bind("9781593278281")
//     .bind(2019)
//     .bind("Programming")
//     .bind(5)
//     .bind(5)
//     .bind("available")
//     .execute(&connection)
//     .await
//     .unwrap();

//     sqlx::query("INSERT INTO loans (loaned_to_user_id, loaned_bookid, checkout_date, due_date, return_date, status) 
// VALUES (?1, ?2, ?3, ?4, ?5, ?6)")
//     .bind(2)
//     .bind(1)
//     .bind("2026-01-30")
//     .bind("2026-02-13")
//     .bind("")
//     .bind("active")
//     .execute(&connection)
//     .await
//     .unwrap();



//     let users:Vec<User> = sqlx::query_as("SELECT * FROM users").fetch_all(&connection).await.unwrap();

//     for u in users {
//         println!("{:?}", u);
//     }


//     let books:Vec<Book> = sqlx::query_as("SELECT * FROM books").fetch_all(&connection).await.unwrap();

//     for b in books {
//         println!("{:?}", b);
//     }


//     let loans:Vec<Loan> = sqlx::query_as("SELECT * FROM loans").fetch_all(&connection).await.unwrap();

//     for l in loans {
//         println!("{:?}", l);
//     }
// }

