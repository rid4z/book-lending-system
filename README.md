
# Book Lending System

A lightweight **library management system** built in **Rust** using raw TCP sockets, `sqlx`, and SQLite.
The application supports **role-based access** (`admin`, `lender`), session-based authentication, and a simple HTML frontend.

---

## Features

* User authentication with secure session cookies
* Role-based access control (Admin / Lender)
* Book inventory management (CRUD)
* Loan checkout, return, and overdue tracking
* SQLite database with transactional integrity
* Zero external web frameworks (manual HTTP parsing)

---

## Tech Stack

* **Language:** Rust
* **Database:** SQLite
* **Async Runtime:** Tokio
* **ORM:** SQLx
* **Frontend:** Static HTML
* **Auth:** Cookie-based sessions stored in DB

---

## Setup Instructions 

### Prerequisites

* Rust (latest stable)
* Cargo
* SQLite

### Clone the Repository

```bash
git clone https://github.com/yourusername/library-management-system.git
cd library-management-system
```

### Install Dependencies

```bash
cargo build
```

### Run the Application

```bash
cargo run
```

The server starts on:

```
http://127.0.0.1:8080
```

### Database

* SQLite database is initialized automatically
* Tables include: `users`, `books`, `loans`, `sessions`
* All schema creation lives in `db.rs`


---

## API Documentation 

### Authentication

#### `POST /login`

Authenticates a user and creates a session.

**Request (form-urlencoded):**

```
username=alice&password=secret
```

**Response:**

* `302 Redirect` to role dashboard
* Sets `session` cookie

---

### Admin Endpoints

#### `GET /admin/books`

Returns all books (including unavailable).

**Response (JSON):**

```json
[
  {
    "bookid": 1,
    "title": "The Rust Book",
    "author": "Steve Klabnik",
    "isbn": "123456",
    "total_copies": 5,
    "available_copies": 3,
    "status": "Available"
  }
]
```

---

#### `POST /admin/books`

Adds a new book or increases copies if ISBN exists.

**Request (JSON):**

```json
{
  "title": "Clean Code",
  "author": "Robert C. Martin",
  "isbn": "9780132350884",
  "year_of_pub": 2008,
  "genre": "Programming",
  "copies": 3
}
```

---

#### `PUT /admin/books/{bookid}`

Updates book metadata and copy counts.

**Validation:**

* Cannot reduce total copies below active loans

---

#### `DELETE /admin/books/{bookid}`

Deletes a book **only if no active loans exist**.

---

#### `GET /admin/loans`

Returns all loans with computed status:

* Borrowed
* Overdue
* Returned

---

#### `GET /admin/overdue`

Returns all overdue loans with days overdue.

---

### Lender Endpoints

#### `GET /lender/books`

Returns all available books.

---

#### `GET /lender/search?q=term`

Search by title, author, ISBN, or genre.

---

#### `POST /lender/checkout/{bookid}`

Checks out a book:

* Uses DB transaction
* Prevents duplicate active loans
* Decrements availability atomically

---

#### `POST /lender/return/{loanid}`

Returns a book:

* Sets `return_date`
* Increments available copies

---

#### `GET /lender/myloans`

Returns active (non-overdue) loans for the user.

---

#### `GET /lender/overdue`

Returns overdue loans with days overdue.

---

##  Code Structure

```
src/
├── main.rs        # TCP server, routing, HTTP parsing
├── auth.rs        # Authentication & password validation
├── db.rs          # Database connection & schema
├── models.rs     # Shared data structs
```

```
static/
├── index.html
├── register.html
├── admin.html
├── lender.html
```
