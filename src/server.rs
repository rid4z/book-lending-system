use std::net::TcpListener;
use std::io::{Read, Write};
use std::sync::Arc;

use sqlx::SqlitePool;

pub async fn run_server(pool: Arc<SqlitePool>) {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    println!("Server running on http://127.0.0.1:8080");

    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        let pool = pool.clone();

        let mut buffer = [0; 4096];
        let bytes_read = stream.read(&mut buffer).unwrap();
        let request = String::from_utf8_lossy(&buffer[..bytes_read]);

        println!("--- RAW REQUEST ---");
        println!("{}", request);

        let mut lines = request.lines();
        let request_line = lines.next().unwrap_or("");
        let parts: Vec<&str> = request_line.split_whitespace().collect();

        if parts.len() < 2 {
            continue;
        }

        let method = parts[0];
        let path = parts[1];
        let body = request.split("\r\n\r\n").nth(1).unwrap_or("");

        if method == "POST" && path == "/register" {
            handle_register(body, &pool).await;

            let response = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nRegistered\n";
            stream.write_all(response.as_bytes()).unwrap();
        } else {
            let response = "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\n\r\nNot Found\n";
            stream.write_all(response.as_bytes()).unwrap();
        }
    }
}

async fn handle_register(body: &str, pool: &SqlitePool) {
    println!("--- FORM BODY ---");
    println!("{}", body);

    let mut username = "";
    let mut password = "";
    let mut role = "";

    for pair in body.split('&') {
        let mut kv = pair.splitn(2, '=');
        let key = kv.next().unwrap_or("");
        let value = kv.next().unwrap_or("");

        match key {
            "username" => username = value,
            "password" => password = value,
            "role" => role = value,
            _ => {}
        }
    }

    println!("Parsed:");
    println!("username = {}", username);
    println!("password = {}", password);
    println!("role     = {}", role);

    sqlx::query("INSERT INTO users (username, password, role) VALUES (?1, ?2, ?3)")
        .bind(username)
        .bind(password)
        .bind(role)
        .execute(pool)
        .await
        .unwrap();

    println!("User inserted.");

    let rows = sqlx::query!("SELECT id, username, role FROM users")
        .fetch_all(pool)
        .await
        .unwrap();

    println!("--- USERS TABLE ---");
    for r in rows {
        println!("id={} username={} role={}", r.id, r.username, r.role);
    }
}
