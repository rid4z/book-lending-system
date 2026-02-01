// mods and packages
mod db;

mod models;
mod auth;
use db::get_db_pool;
use sqlx::SqlitePool;

use std::collections::HashMap;
use std::fs;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use auth::{hash_password, verify_password, generate_session_token};
use models::*;


//----------------------------------------------------------------------------------------------------------
// creating server to run on


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Starting async server...");

    let pool = get_db_pool().await;
    
    println!("Database ready.");

    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("Listening on http://127.0.0.1:8080");

    loop {
        let (stream, _) = listener.accept().await?;
        let pool = pool.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, pool).await {
                eprintln!("Connection error: {:?}", e);
            }
        });
    }
}

//----------------------------------------------------------------------------------------------------------
// get/post from webpages sent back to server




async fn handle_connection(
    mut stream: TcpStream,
    pool: SqlitePool,
) -> anyhow::Result<()> {
    let mut buffer = [0u8; 16384];
    let bytes_read = stream.read(&mut buffer).await?;
    if bytes_read == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    println!("==== RAW REQUEST ====");
    println!("{}", request);

    let (method, path) = parse_request_line(&request);
    let body = extract_body(&request);

    // Resolve session from cookie (DB-backed)
    let session = resolve_session(&request, &pool).await;


    match (method.as_str(), path.as_str()) {
        // serving login/register pages
        ("GET", "/") => serve_file(&mut stream, "static/index.html").await?,
        ("GET", "/register.html") => serve_file(&mut stream, "static/register.html").await?,

        // forms
        ("POST", "/login") => handle_login(&mut stream, pool, body).await?,
        ("POST", "/register") => handle_register(&mut stream, pool, body).await?,

        // logout — destroys session then redirects
        ("GET", "/logout") | ("POST", "/logout") => {
            destroy_session(&pool, &request).await;
            send_logout_redirect(&mut stream).await?;
        }

        // protected pages — require valid session with correct role
        ("GET", "/admin.html") => {
            match &session {
                Some((_, role)) if role == "admin" => {
                    serve_file(&mut stream, "static/admin.html").await?
                }
                _ => send_redirect(&mut stream, "/").await?,
            }
        }
        ("GET", "/lender.html") => {
            match &session {
                Some((_, role)) if role == "lender" => {
                    serve_file(&mut stream, "static/lender.html").await?
                }
                _ => send_redirect(&mut stream, "/").await?,
            }
        }

        // admin dashboard api — all gated on admin session
        ("GET", "/admin/api/users") => {
            match &session {
                Some((_, role)) if role == "admin" => {
                    handle_admin_users(&mut stream, pool).await?
                }
                _ => send_json(&mut stream, b"{\"error\":\"unauthorized\"}").await?,
            }
        }
        ("GET", "/admin/api/books") => {
            match &session {
                Some((_, role)) if role == "admin" => {
                    handle_admin_books(&mut stream, pool).await?
                }
                _ => send_json(&mut stream, b"{\"error\":\"unauthorized\"}").await?,
            }
        }
        ("GET", "/admin/api/loans") => {
            match &session {
                Some((_, role)) if role == "admin" => {
                    handle_admin_loans(&mut stream, pool).await?
                }
                _ => send_json(&mut stream, b"{\"error\":\"unauthorized\"}").await?,
            }
        }
        ("GET", "/admin/api/overdue") => {
            match &session {
                Some((_, role)) if role == "admin" => {
                    handle_admin_overdue(&mut stream, pool).await?
                }
                _ => send_json(&mut stream, b"{\"error\":\"unauthorized\"}").await?,
            }
        }
        ("POST", "/admin/api/books") => {
            match &session {
                Some((_, role)) if role == "admin" => {
                    handle_admin_add_book(&mut stream, pool, body).await?
                }
                _ => send_json(&mut stream, b"{\"error\":\"unauthorized\"}").await?,
            }
        }
        ("PUT", path) if path.starts_with("/admin/api/books") => {
            match &session {
                Some((_, role)) if role == "admin" => {
                    if let Some(bookid) = parse_query_param(path, "bookid")
                        .and_then(|v| v.parse::<i64>().ok())
                    {
                        handle_admin_update_book(&mut stream, pool, bookid, body).await?;
                    } else {
                        send_html(&mut stream, b"Missing bookid").await?;
                    }
                }
                _ => { send_json(&mut stream, b"{\"error\":\"unauthorized\"}").await?; }
            }
        }
        ("DELETE", path) if path.starts_with("/admin/api/books") => {
            match &session {
                Some((_, role)) if role == "admin" => {
                    if let Some(bookid) = parse_query_param(path, "bookid")
                        .and_then(|v| v.parse::<i64>().ok())
                    {
                        handle_admin_delete_book(&mut stream, pool, bookid).await?;
                    } else {
                        send_html(&mut stream, b"Missing bookid").await?;
                    }
                }
                _ => { send_json(&mut stream, b"{\"error\":\"unauthorized\"}").await?; }
            }
        }

        // lender dashboard api — gated on lender session
        ("GET", "/lender/api/books") => {
            match &session {
                Some((_, role)) if role == "lender" => {
                    handle_lender_books(&mut stream, pool).await?
                }
                _ => send_json(&mut stream, b"[]").await?,
            }
        }
        ("GET", path) if path.starts_with("/lender/api/search") => {
            match &session {
                Some((_, role)) if role == "lender" => {
                    if let Some(q) = parse_query_param(path, "q") {
                        handle_lender_search(&mut stream, pool, &q).await?;
                    } else {
                        send_json(&mut stream, b"[]").await?;
                    }
                }
                _ => { send_json(&mut stream, b"[]").await?; }
            }
        }
        ("GET", path) if path.starts_with("/lender/api/myloans") => {
            match &session {
                Some((username, role)) if role == "lender" => {
                    handle_lender_myloans(&mut stream, pool, username).await?;
                }
                _ => { send_json(&mut stream, b"[]").await?; }
            }
        }
        ("POST", path) if path.starts_with("/lender/api/checkout") => {
            match &session {
                Some((username, role)) if role == "lender" => {
                    if let Some(bookid) = parse_query_param(path, "bookid").and_then(|v| v.parse().ok()) {
                        handle_lender_checkout(&mut stream, pool, username, bookid).await?;
                    } else {
                        send_html(&mut stream, b"<h1>Invalid checkout request</h1>").await?;
                    }
                }
                _ => { send_html(&mut stream, b"<h1>Not logged in</h1>").await?; }
            }
        }
        ("POST", path) if path.starts_with("/lender/api/return") => {
            match &session {
                Some((_, role)) if role == "lender" => {
                    if let Some(loanid) = parse_query_param(path, "loanid").and_then(|v| v.parse().ok()) {
                        handle_lender_return(&mut stream, pool, loanid).await?;
                    } else {
                        send_html(&mut stream, b"<h1>Invalid return request</h1>").await?;
                    }
                }
                _ => { send_html(&mut stream, b"<h1>Not logged in</h1>").await?; }
            }
        }
        ("GET", path) if path.starts_with("/lender/api/overdue") => {
            match &session {
                Some((username, role)) if role == "lender" => {
                    handle_lender_overdue(&mut stream, pool, username).await?;
                }
                _ => { send_json(&mut stream, b"[]").await?; }
            }
        }

        _ => send_404(&mut stream).await?,
    }

    Ok(())
}


//----------------------------------------------------------------------------------------------------------
// handler functions


//register
async fn handle_register(
    stream: &mut TcpStream,
    pool: SqlitePool,
    body: &str,
) -> anyhow::Result<()> {
    let form = parse_form_urlencoded(body);

    let username = form
        .get("username")
        .map(|s| s.as_str())
        .unwrap_or("");

    let password = form
        .get("password")
        .map(|s| s.as_str())
        .unwrap_or("");

    let role = form
        .get("role")
        .map(|s| s.as_str())
        .unwrap_or("");

    println!("REGISTER: {} {} {}", username, password, role);

    // if user already exists
    let exists: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(&pool)
            .await?;

    if exists.is_some() {
        let html = b"<h1>User already exists</h1><a href=\"/register.html\">Back</a>";
        send_html(stream, html).await?;
        return Ok(());
    }
    let hashed = hash_password(password)?;

    // insert/register new user
    sqlx::query(
        "INSERT INTO users (username, password, role) VALUES (?, ?, ?)",
    )
    .bind(username)
    .bind(&hashed)
    .bind(role)
    .execute(&pool)
    .await?;

     println!("Registration successful. Redirecting to dashboard.");

    // redirect to correct dashboard based on registered role
    match role {
        "admin" => {
            let token = create_session(&pool, username, "admin").await?;
            send_redirect_with_session_cookie(stream, "/admin.html", &token).await?;
        }
        "lender" => {
            let token = create_session(&pool, username, "lender").await?;
            send_redirect_with_session_cookie(stream, "/lender.html", &token).await?;
        }
        _ => {
            let html = b"<h1>Unknown role</h1>";
            send_html(stream, html).await?;
        }
    }

    Ok(())
}

//handle login
async fn handle_login(
    stream: &mut TcpStream,
    pool: SqlitePool,
    body: &str,
) -> anyhow::Result<()> {
    let form = parse_form_urlencoded(body);

    let username = form
        .get("username")
        .map(|s| s.as_str())
        .unwrap_or("");

    let password = form
        .get("password")
        .map(|s| s.as_str())
        .unwrap_or("");

    println!("LOGIN: {} {}", username, password);

    // Get password + role
    let user: Option<(String, String)> =
        sqlx::query_as(
            "SELECT password, role FROM users WHERE username = ?",
        )
        .bind(username)
        .fetch_optional(&pool)
        .await?;

    match user {
    None => {
        send_redirect(stream, "/register.html").await?;
    }
    Some((db_password, role)) => {
        let valid = verify_password(password, &db_password)?;

        if !valid {
            let html = b"<h1>Invalid password</h1><a href=\"/\">Back</a>";
            send_html(stream, html).await?;
            return Ok(());
        }

        println!("Login success. Role = {}", role);

        match role.as_str() {
            "admin" => {
                let token = create_session(&pool, username, "admin").await?;
                send_redirect_with_session_cookie(stream, "/admin.html", &token).await?;
            }
            "lender" => {
                let token = create_session(&pool, username, "lender").await?;
                send_redirect_with_session_cookie(stream, "/lender.html", &token).await?;
            }
            _ => {
                let html = b"<h1>Unknown role</h1>";
                send_html(stream, html).await?;
            }
        }
    }
}


    Ok(())
}


//----------------------------------------------------------------------------------------------------------
// admin dashboard fns

async fn handle_admin_users(
    stream: &mut TcpStream,
    pool: SqlitePool,
) -> anyhow::Result<()> {
    let users = sqlx::query_as::<_, AdminUser>(
    "
    SELECT id, username, role
    FROM users
    "
)
.fetch_all(&pool)
.await?;


    let json = serde_json::to_vec(&users)?;
    send_json(stream, &json).await
}

async fn handle_admin_books(
    stream: &mut TcpStream,
    pool: SqlitePool,
) -> anyhow::Result<()> {

    // First, sync all book availability counts with actual active loans
    sync_book_availability(&pool).await?;

    let rows = sqlx::query_as::<_, (
        i64,            // bookid
        String,         // title
        String,         // author
        String,         // isbn
        Option<i64>,    // year_of_pub
        Option<String>, // genre
        i64,            // total_copies
        i64             // available_copies
    )>(
        "
        SELECT
            bookid,
            title,
            author,
            isbn,
            year_of_pub,
            genre,
            total_copies,
            available_copies
        FROM books
        "
    )
    .fetch_all(&pool)
    .await?;

    let books: Vec<AdminBook> = rows
        .into_iter()
        .map(|(bookid, title, author, isbn, year_of_pub, genre, total, available)| {
            let checked_out = total - available;

            let status = format!(
                "{} available, {} checked out",
                available, checked_out
            );

            AdminBook {
                bookid,
                title,
                author,
                isbn,
                year_of_pub,
                genre,
                total_copies: total,
                available_copies: available,
                status,
            }
        })
        .collect();

    let json = serde_json::to_vec(&books)?;
    send_json(stream, &json).await
}

/// Syncs available_copies for all books based on actual active loans
async fn sync_book_availability(pool: &SqlitePool) -> anyhow::Result<()> {
    // For each book, recalculate available_copies = total_copies - active_loans
    sqlx::query(
        "
        UPDATE books
        SET available_copies = total_copies - (
            SELECT COUNT(*)
            FROM loans
            WHERE loans.loaned_bookid = books.bookid
            AND loans.return_date IS NULL
        )
        "
    )
    .execute(pool)
    .await?;

    Ok(())
}


async fn handle_admin_loans(
    stream: &mut TcpStream,
    pool: SqlitePool,
) -> anyhow::Result<()> {
    let rows = sqlx::query_as::<_, (i64, String, String, String, String, Option<String>)>(
        "
        SELECT
            l.loanid,
            u.username,
            b.title,
            l.checkout_date,
            l.due_date,
            l.return_date
        FROM loans l
        JOIN users u ON u.id = l.loaned_to_user_id
        JOIN books b ON b.bookid = l.loaned_bookid
        "
    )
    .fetch_all(&pool)
    .await?;

    let result: Vec<AdminLoan> = rows
        .into_iter()
        .map(|(loanid, username, title, checkout_date, due_date, return_date)| {
            let status = calculate_loan_status(&due_date, return_date.as_deref());

            AdminLoan {
                loanid,
                username,
                title,
                checkout_date,
                due_date,
                status,
            }
        })
        .collect();

    let json = serde_json::to_vec(&result)?;
    send_json(stream, &json).await
}

//admin crud OPS
async fn handle_admin_add_book(
    stream: &mut TcpStream,
    pool: SqlitePool,
    body: &str,
) -> anyhow::Result<()> {

    let input: AdminBookInput = serde_json::from_str(body)?;
    if input.title.trim().is_empty()
    || input.author.trim().is_empty()
    || input.isbn.trim().is_empty()
    || input.copies <= 0
{
    send_html(stream, b"Invalid book data").await?;
    return Ok(());
}

    // check if book already exists by ISBN
    let existing: Option<(i64, i64, i64)> =
        sqlx::query_as(
            "SELECT bookid, total_copies, available_copies FROM books WHERE isbn = ?"
        )
        .bind(&input.isbn)
        .fetch_optional(&pool)
        .await?;

    match existing {
        Some((bookid, _, _)) => {
            // increase copies
            sqlx::query(
                "
                UPDATE books
                SET total_copies = total_copies + ?,
                    available_copies = available_copies + ?
                WHERE bookid = ?
                "
            )
            .bind(input.copies)
            .bind(input.copies)
            .bind(bookid)
            .execute(&pool)
            .await?;

            send_html(stream, b"Book exists - copies increased").await
        }

        None => {
            // insert new book
            sqlx::query(
                "
                INSERT INTO books
                (title, author, isbn, year_of_pub, genre, total_copies, available_copies)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                "
            )
            .bind(&input.title)
            .bind(&input.author)
            .bind(&input.isbn)
            .bind(input.year_of_pub)
            .bind(&input.genre)
            .bind(input.copies)
            .bind(input.copies)
            .execute(&pool)
            .await?;

            send_html(stream, b"Book added successfully").await
        }
    }
}

async fn handle_admin_update_book(
    stream: &mut TcpStream,
    pool: SqlitePool,
    bookid: i64,
    body: &str,
) -> anyhow::Result<()> {

    let input: AdminBookInput = serde_json::from_str(body)?;

    // fetch current counts
    let (total, available): (i64, i64) = sqlx::query_as(
        "SELECT total_copies, available_copies FROM books WHERE bookid = ?"
    )
    .bind(bookid)
    .fetch_one(&pool)
    .await?;

    let checked_out = total - available;

    if input.copies < checked_out {
        send_html(
            stream,
            b"Cannot reduce total copies below number currently checked out",
        ).await?;
        return Ok(());
    }

    let new_available = input.copies - checked_out;

    sqlx::query(
        "
        UPDATE books
        SET title = ?, author = ?, isbn = ?, year_of_pub = ?, genre = ?,
            total_copies = ?, available_copies = ?
        WHERE bookid = ?
        "
    )
    .bind(&input.title)
    .bind(&input.author)
    .bind(&input.isbn)
    .bind(input.year_of_pub)
    .bind(&input.genre)
    .bind(input.copies)
    .bind(new_available)
    .bind(bookid)
    .execute(&pool)
    .await?;

    send_html(stream, b"Book updated successfully").await
}


async fn handle_admin_delete_book(
    stream: &mut TcpStream,
    pool: SqlitePool,
    bookid: i64,
) -> anyhow::Result<()> {

    // Check active loans
    let active_loans: i64 = sqlx::query_scalar(
        "
        SELECT COUNT(*)
        FROM loans
        WHERE loaned_bookid = ?
        AND return_date IS NULL
        "
    )
    .bind(bookid)
    .fetch_one(&pool)
    .await?;

    if active_loans > 0 {
        let response = serde_json::json!({
            "success": false,
            "message": format!(
                "Cannot delete book {}: {} active loan(s) exist",
                bookid, active_loans
            )
        });

        let json = serde_json::to_vec(&response)?;
        send_json(stream, &json).await?;
        return Ok(());
    }

    // 🔥 Use transaction for safe multi-step delete
    let mut tx = pool.begin().await?;

    // 1️⃣ Delete historical loans
    sqlx::query(
        "
        DELETE FROM loans
        WHERE loaned_bookid = ?
        "
    )
    .bind(bookid)
    .execute(&mut *tx)
    .await?;

    // 2️⃣ Delete book
    sqlx::query(
        "
        DELETE FROM books
        WHERE bookid = ?
        "
    )
    .bind(bookid)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    let response = serde_json::json!({
        "success": true,
        "message": "Book and related loan history deleted"
    });

    let json = serde_json::to_vec(&response)?;
    send_json(stream, &json).await
}


async fn handle_admin_overdue(
    stream: &mut TcpStream,
    pool: SqlitePool,
) -> anyhow::Result<()> {

    let rows = sqlx::query_as::<_, (String, String, String)>(
        "
        SELECT
            u.username,
            b.title,
            l.due_date
        FROM loans l
        JOIN users u ON u.id = l.loaned_to_user_id
        JOIN books b ON b.bookid = l.loaned_bookid
        WHERE l.return_date IS NULL
          AND date(l.due_date) < date('now')
        "
    )
    .fetch_all(&pool)
    .await?;

    let today = chrono::Utc::now().date_naive();

    let result: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|(username, title, due_date)| {
            let due = chrono::NaiveDate::parse_from_str(&due_date, "%Y-%m-%d").unwrap();
            let days = (today - due).num_days();

            serde_json::json!({
                "username": username,
                "title": title,
                "due_date": due_date,
                "days_overdue": days
            })
        })
        .collect();

    let json = serde_json::to_vec(&result)?;
    send_json(stream, &json).await
}

//----------------------------------------------------------------------------------------------------------
// lender dashboard fns

async fn handle_lender_books(
    stream: &mut TcpStream,
    pool: SqlitePool,
) -> anyhow::Result<()> {
    // Sync availability before showing books to lenders
    sync_book_availability(&pool).await?;

    let books = sqlx::query_as::<_, LenderBook>(
        "
        SELECT 
            bookid,
            title,
            author,
            genre,
            available_copies
        FROM books
        WHERE available_copies > 0
        "
    )
    .fetch_all(&pool)
    .await?;

    let json = serde_json::to_vec(&books)?;
    send_json(stream, &json).await
}

async fn handle_lender_search(
    stream: &mut TcpStream,
    pool: SqlitePool,
    query: &str,
) -> anyhow::Result<()> {
    // Sync availability before searching
    sync_book_availability(&pool).await?;

    let q = format!("%{}%", query);

    let books = sqlx::query_as::<_, LenderBook>(
        "
        SELECT
            bookid,
            title,
            author,
            isbn,
            genre,
            available_copies
        FROM books
        WHERE available_copies > 0
          AND (
            title LIKE ?
            OR author LIKE ?
            OR isbn LIKE ?
            OR genre LIKE ?
          )
        "
    )
    .bind(&q)
    .bind(&q)
    .bind(&q)
    .bind(&q)
    .fetch_all(&pool)
    .await?;

    let json = serde_json::to_vec(&books)?;
    send_json(stream, &json).await
}


async fn handle_lender_myloans(
    stream: &mut TcpStream,
    pool: SqlitePool,
    username: &str,
) -> anyhow::Result<()> {

    let loans = sqlx::query_as::<_, (i64, String, String, String)>(
        "
        SELECT 
            l.loanid,
            b.title,
            l.checkout_date,
            l.due_date
        FROM loans l
        JOIN users u ON u.id = l.loaned_to_user_id
        JOIN books b ON b.bookid = l.loaned_bookid
        WHERE u.username = ?
          AND l.return_date IS NULL
          AND date(l.due_date) >= date('now')
        "
    )
    .bind(username)
    .fetch_all(&pool)
    .await?;

    let result: Vec<LenderLoan> = loans
        .into_iter()
        .map(|(loanid, title, checkout_date, due_date)| {
            LenderLoan {
                loanid,
                title,
                checkout_date,
                due_date,
                status: "Borrowed".to_string(),
            }
        })
        .collect();

    let json = serde_json::to_vec(&result)?;
    send_json(stream, &json).await
}


async fn handle_lender_checkout(
    stream: &mut TcpStream,
    pool: SqlitePool,
    username: &str,
    bookid: i64,
) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;

    // Get user id
    let user = sqlx::query_as::<_, (i64,)>(
        "SELECT id FROM users WHERE username = ?"
    )
    .bind(username)
    .fetch_optional(&mut *tx)
    .await?;

    let user_id = match user {
        Some((id,)) => id,
        None => {
            send_html(stream, b"<h1>User not found</h1>").await?;
            return Ok(());
        }
    };


    // Check availability
    let row = sqlx::query_as::<_, (i64,)>(
        "SELECT available_copies FROM books WHERE bookid = ?"
    )
    .bind(bookid)
    .fetch_optional(&mut *tx)
    .await?;

    let available = match row {
        Some((v,)) => v,
        None => {
            send_html(stream, b"<h1>Book not found</h1>").await?;
            return Ok(());
        }
    };


    if available <= 0 {
        send_html(stream, b"<h1>Book not available</h1>").await?;
        return Ok(());
    }

    let already_borrowed = sqlx::query_scalar::<_, i64>(
        "
        SELECT COUNT(*)
        FROM loans
        WHERE loaned_to_user_id = ?
        AND loaned_bookid = ?
        AND return_date IS NULL
        "
    )
    .bind(user_id)
    .bind(bookid)
    .fetch_one(&mut *tx)
    .await?;

    if already_borrowed > 0 {
        send_html(stream, b"<h1>You already borrowed this book</h1>").await?;
        return Ok(());
    }

    // Insert loan
    sqlx::query(
        "
        INSERT INTO loans (loaned_to_user_id, loaned_bookid, checkout_date, due_date, return_date)
        VALUES (?, ?, date('now'), date('now', '+14 days'), NULL)
        "
    )
    .bind(user_id)
    .bind(bookid)
    .execute(&mut *tx)
    .await?;

    // Decrement copies
    sqlx::query(
        "
        UPDATE books
        SET available_copies = available_copies - 1
        WHERE bookid = ?
        "
    )
    .bind(bookid)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    send_html(stream, b"<h1>Checkout successful</h1>").await
}

async fn handle_lender_return(
    stream: &mut TcpStream,
    pool: SqlitePool,
    loanid: i64,
) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;

    // Get bookid
    let (bookid,): (i64,) = sqlx::query_as(
        "SELECT loaned_bookid FROM loans WHERE loanid = ?"
    )
    .bind(loanid)
    .fetch_one(&mut *tx)
    .await?;

    // Update loan
    sqlx::query(
        "UPDATE loans SET return_date = date('now') WHERE loanid = ?"
    )
    .bind(loanid)
    .execute(&mut *tx)
    .await?;

    // Increment copies
    sqlx::query(
        "
        UPDATE books
        SET available_copies = available_copies + 1
        WHERE bookid = ?
        "
    )
    .bind(bookid)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    send_html(stream, b"<h1>Return successful</h1>").await
}

async fn handle_lender_overdue(
    stream: &mut TcpStream,
    pool: SqlitePool,
    username: &str,
) -> anyhow::Result<()> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "
        SELECT 
            b.title,
            l.due_date
        FROM loans l
        JOIN users u ON u.id = l.loaned_to_user_id
        JOIN books b ON b.bookid = l.loaned_bookid
        WHERE u.username = ?
          AND l.return_date IS NULL
          AND date(l.due_date) < date('now')
        "
    )
    .bind(username)
    .fetch_all(&pool)
    .await?;

    let today = chrono::Utc::now().date_naive();

    let result: Vec<OverdueLoan> = rows
        .into_iter()
        .map(|(title, due_date)| {
            let due = chrono::NaiveDate::parse_from_str(&due_date, "%Y-%m-%d").unwrap();
            let days_overdue = (today - due).num_days();

            OverdueLoan {
                title,
                due_date,
                days_overdue,
            }
        })
        .collect();

    let json = serde_json::to_vec(&result)?;
    send_json(stream, &json).await
}


//----------------------------------------------------------------------------------------------------------
// parsing fns

async fn send_redirect(
    stream: &mut TcpStream,
    location: &str,
) -> anyhow::Result<()> {
    let response = format!(
        "HTTP/1.1 302 Found\r\n\
         Location: {}\r\n\
         Content-Length: 0\r\n\
         Connection: close\r\n\
         \r\n",
        location
    );

    stream.write_all(response.as_bytes()).await?;
    Ok(())
}


async fn send_redirect_with_session_cookie(
    stream: &mut TcpStream,
    location: &str,
    token: &str,
) -> anyhow::Result<()> {
    let response = format!(
        "HTTP/1.1 302 Found\r\n\
         Location: {}\r\n\
         Set-Cookie: session={}; Path=/; HttpOnly\r\n\
         Content-Length: 0\r\n\
         Connection: close\r\n\
         \r\n",
        location, token
    );

    stream.write_all(response.as_bytes()).await?;
    Ok(())
}

/// Clears the session cookie and redirects to login
async fn send_logout_redirect(stream: &mut TcpStream) -> anyhow::Result<()> {
    let response = format!(
        "HTTP/1.1 302 Found\r\n\
         Location: /\r\n\
         Set-Cookie: session=; Path=/; HttpOnly; Max-Age=0\r\n\
         Content-Length: 0\r\n\
         Connection: close\r\n\
         \r\n"
    );

    stream.write_all(response.as_bytes()).await?;
    Ok(())
}


/// Looks up the session cookie token in the DB.
/// Returns Some((username, role)) if a valid non-expired session exists, else None.
async fn resolve_session(request: &str, pool: &SqlitePool) -> Option<(String, String)> {
    let token = get_cookie_value(request, "session")?;

    let row: Option<(String, String, String)> = sqlx::query_as(
        "SELECT username, role, expires_at FROM sessions WHERE token = ?"
    )
    .bind(&token)
    .fetch_optional(pool)
    .await
    .ok()?;

    match row {
        Some((username, role, expires_at)) => {
            let expiry = chrono::NaiveDateTime::parse_from_str(&expires_at, "%Y-%m-%d %H:%M:%S")
                .ok()?;
            let now = chrono::Utc::now().naive_utc();

            if now > expiry {
                // Session expired — delete it silently
                let _ = sqlx::query("DELETE FROM sessions WHERE token = ?")
                    .bind(&token)
                    .execute(pool)
                    .await;
                None
            } else {
                Some((username, role))
            }
        }
        None => None,
    }
}

/// Extracts a specific cookie value from the raw request string
fn get_cookie_value(request: &str, name: &str) -> Option<String> {
    let prefix = format!("{}=", name);
    for line in request.lines() {
        if line.starts_with("Cookie:") {
            let cookies = line.strip_prefix("Cookie: ").unwrap_or(line);
            for cookie in cookies.split(';') {
                let cookie = cookie.trim();
                if let Some(value) = cookie.strip_prefix(&prefix) {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

/// Creates a session row in the DB and returns the token
async fn create_session(pool: &SqlitePool, username: &str, role: &str) -> anyhow::Result<String> {
    let token = generate_session_token();
    // Session lasts 24 hours
    let expires_at = (chrono::Utc::now().naive_utc() + chrono::Duration::hours(24))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();

    sqlx::query(
        "INSERT INTO sessions (token, username, role, expires_at) VALUES (?, ?, ?, ?)"
    )
    .bind(&token)
    .bind(username)
    .bind(role)
    .bind(&expires_at)
    .execute(pool)
    .await?;

    Ok(token)
}

/// Deletes a session from the DB by token
async fn destroy_session(pool: &SqlitePool, request: &str) {
    if let Some(token) = get_cookie_value(request, "session") {
        let _ = sqlx::query("DELETE FROM sessions WHERE token = ?")
            .bind(&token)
            .execute(pool)
            .await;
    }
}


async fn serve_file(stream: &mut TcpStream, path: &str) -> anyhow::Result<()> {
    let contents = fs::read(path)?;
    send_response(stream, 200, "text/html", &contents).await
}

async fn send_404(stream: &mut TcpStream) -> anyhow::Result<()> {
    send_response(stream, 404, "text/html", b"<h1>404 Not Found</h1>").await
}

async fn send_html(stream: &mut TcpStream, body: &[u8]) -> anyhow::Result<()> {
    send_response(stream, 200, "text/html", body).await
}


fn parse_request_line(request: &str) -> (String, String) {
    let first = request.lines().next().unwrap_or("");
    let parts: Vec<&str> = first.split_whitespace().collect();
    if parts.len() >= 2 {
        (parts[0].to_string(), parts[1].to_string())
    } else {
        ("".into(), "".into())
    }
}

fn extract_body(request: &str) -> &str {
    if let Some(pos) = request.find("\r\n\r\n") {
        &request[pos + 4..]
    } else {
        ""
    }
}

fn parse_form_urlencoded(body: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for pair in body.split('&') {
        let mut parts = pair.splitn(2, '=');
        if let (Some(k), Some(v)) = (parts.next(), parts.next()) {
            map.insert(url_decode(k), url_decode(v));
        }
    }
    map
}

fn url_decode(input: &str) -> String {
    input.replace('+', " ")
}


async fn send_response(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> anyhow::Result<()> {
    let status_text = match status {
        200 => "OK",
        302 => "Found",
        404 => "Not Found",
        _ => "OK",
    };

    let header = format!(
        "HTTP/1.1 {} {}\r\n\
         Content-Type: {}\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n",
        status,
        status_text,
        content_type,
        body.len()
    );

    stream.write_all(header.as_bytes()).await?;
    stream.write_all(body).await?;
    Ok(())
}

async fn send_json(
    stream: &mut TcpStream,
    body: &[u8],
) -> anyhow::Result<()> {
    let header = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n",
        body.len()
    );

    stream.write_all(header.as_bytes()).await?;
    stream.write_all(body).await?;
    Ok(())
}


fn parse_query_param(path: &str, key: &str) -> Option<String> {
    let parts: Vec<&str> = path.split('?').collect();
    if parts.len() < 2 {
        return None;
    }

    for pair in parts[1].split('&') {
        let mut kv = pair.splitn(2, '=');
        if let (Some(k), Some(v)) = (kv.next(), kv.next()) {
            if k == key {
                return Some(url_decode(v));
            }
        }
    }
    None
}

fn calculate_loan_status(
    due_date: &str,
    return_date: Option<&str>,
) -> String {
    if return_date.is_some() {
        return "Returned".to_string();
    }

    let today = chrono::Utc::now().date_naive();
    let due = chrono::NaiveDate::parse_from_str(due_date, "%Y-%m-%d")
        .unwrap();

    if due < today {
        "Overdue".to_string()
    } else {
        "Borrowed".to_string()
    }
}