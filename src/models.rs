use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Serialize, FromRow)]
pub struct AdminUser {
    pub id: i64,
    pub username: String,
    pub role: String,
}

#[derive(Serialize, FromRow)]
pub struct AdminBook {
    pub bookid: i64,
    pub title: String,
    pub author: String,
    pub isbn: String,
    pub year_of_pub: Option<i64>,
    pub genre: Option<String>,
    pub total_copies: i64,
    pub available_copies: i64,
    pub status: String,
}

#[derive(Serialize)]
pub struct AdminLoan {
    pub loanid: i64,
    pub username: String,
    pub title: String,
    pub checkout_date: String,
    pub due_date: String,
    pub status: String,
}

#[derive(Deserialize)]
pub struct AdminBookInput {
    pub title: String,
    pub author: String,
    pub isbn: String,
    pub year_of_pub: Option<i64>,
    pub genre: Option<String>,
    pub copies: i64,
}

#[derive(Serialize, FromRow)]
pub struct LenderBook {
    pub bookid: i64,
    pub title: String,
    pub author: String,
    pub genre: String,
    pub available_copies: i64,
}

#[derive(Serialize, FromRow)]
pub struct LenderLoan {
    pub loanid: i64,
    pub title: String,
    pub checkout_date: String,
    pub due_date: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct OverdueLoan {
    pub loanid: i64,
    pub title: String,
    pub due_date: String,
    pub days_overdue: i64,
}