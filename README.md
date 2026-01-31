# Book Lending System

A lightweight **library management system** built in **Rust** using raw TCP sockets, `sqlx`, and SQLite.
The application supports **role-based access** (`admin`, `lender`), session-based authentication, and a simple HTML frontend. The link of this application demo can be found [here](https://drive.google.com/drive/folders/1YUH7DLVj9_5zJlwSGShOW2m3QjYGx-6R?usp=drive_link).

---

## Features

The idea is to have 2 types of users: admins and lenders. The admins can access their dashboard where CRUD operations are utilised, as well as they are given privileges such as to:
1. View all books in the system
2. View all users registered in the system
3. View currently borrowed books and by whom
4. View overdue books

The lenders have their own dashboard, where they can search for books, checkout books, return books, as well as view overdue books.

Some notable features aside from the core functionalities described are:
1. User authentication with secure session cookies
2. Secure password storage via salted hashing
3. Handling due dates and overdue dates calculations

---

## Tech Stack

* **Language:** Rust
* **Database:** SQLite
* **Async Runtime:** Tokio
* **ORM:** SQLx
* **Frontend:** Static HTML
* **Auth:** Cookie-based sessions stored in DB

---


##  Code Structure

```
Cargo.toml
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
---

## Setup Instructions 

### Prerequisites

* Rust 2021
* Cargo
* SQLite

### Clone the Repository

```bash
git clone <repository-url>
cd book-lending-system
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

* SQLite database is initialized automatically on first run (`test.db`)
* Tables: `users`, `books`, `loans`, `sessions`

---

## Database Schema

### Users
*    id       INTEGER PRIMARY KEY AUTOINCREMENT
*   username TEXT UNIQUE NOT NULL
*   password TEXT NOT NULL
*   role     TEXT NOT NULL

### Books
*   bookid           INTEGER PRIMARY KEY AUTOINCREMENT
*   title            TEXT NOT NULL
*   author           TEXT NOT NULL
*   isbn             TEXT UNIQUE NOT NULL
*   year_of_pub      INTEGER
*   genre            TEXT
*   total_copies     INTEGER NOT NULL DEFAULT 0
*   available_copies INTEGER NOT NULL DEFAULT 0

### Loans
*   loanid            INTEGER PRIMARY KEY AUTOINCREMENT
*   loaned_to_user_id INTEGER NOT NULL
*   loaned_bookid     INTEGER NOT NULL
*   checkout_date     TEXT NOT NULL
*   due_date          TEXT NOT NULL
*   return_date       TEXT
*   FOREIGN KEY (loaned_to_user_id) REFERENCES users(id)
*   FOREIGN KEY (loaned_bookid)     REFERENCES books(bookid)

### Sessions
*   token      TEXT PRIMARY KEY
*   username   TEXT NOT NULL
*   role       TEXT NOT NULL
*   expires_at TEXT NOT NULL



---

## API Documentation

All endpoints run on `http://127.0.0.1:8080`. Protected routes require a valid `session` cookie (set automatically on login/register). The cookie is `HttpOnly` — it cannot be read or modified by JavaScript.

---

### Public Routes

---

#### `GET /`

Serves the login page.

**Auth:** None

**Response:** `200` — `static/index.html`

---

#### `GET /register.html`

Serves the registration page.

**Auth:** None

**Response:** `200` — `static/register.html`

---

#### `POST /login`

Authenticates a user, creates a session, and redirects to the appropriate dashboard.

**Auth:** None

**Content-Type:** `application/x-www-form-urlencoded`

**Form fields:**

| Field      | Type   | Required |
|------------|--------|----------|
| `username` | string | Yes      |
| `password` | string | Yes      |

**Responses:**

| Status | Condition                        | Result                                         |
|--------|----------------------------------|------------------------------------------------|
| 302    | Valid credentials, role = admin  | Redirect → `/admin.html` + session cookie set  |
| 302    | Valid credentials, role = lender | Redirect → `/lender.html` + session cookie set |
| 302    | Username not found               | Redirect → `/register.html`                    |
| 200    | Wrong password                   | `<h1>Invalid password</h1>` + back link        |

---

#### `POST /register`

Creates a new user, creates a session, and redirects to the appropriate dashboard.

**Auth:** None

**Content-Type:** `application/x-www-form-urlencoded`

**Form fields:**

| Field      | Type   | Required | Notes                        |
|------------|--------|----------|------------------------------|
| `username` | string | Yes      | Must be unique               |
| `password` | string | Yes      | Stored as bcrypt hash        |
| `role`     | string | Yes      | `admin` or `lender`          |

**Responses:**

| Status | Condition                        | Result                                         |
|--------|----------------------------------|------------------------------------------------|
| 302    | Success, role = admin            | Redirect → `/admin.html` + session cookie set  |
| 302    | Success, role = lender           | Redirect → `/lender.html` + session cookie set |
| 200    | Username already exists          | `<h1>User already exists</h1>` + back link     |
| 200    | Invalid role value               | `<h1>Unknown role</h1>`                        |

---

#### `GET /logout`

Destroys the session in the database and clears the cookie.

**Auth:** None (safe to call without an active session)

**Response:** `302` Redirect → `/` with `Set-Cookie: session=; Max-Age=0`

---

### Protected Page Routes

These serve the dashboard HTML pages. A valid session with the matching role is required; otherwise the server redirects back to `/`.

| Route             | Required Role | Serves                  |
|-------------------|---------------|-------------------------|
| `GET /admin.html` | `admin`       | `static/admin.html`     |
| `GET /lender.html`| `lender`      | `static/lender.html`    |

---

### Admin API

All endpoints below require a valid session with `role = admin`. Unauthorized requests return `{"error":"unauthorized"}`.

---

#### `GET /admin/api/users`

Returns all registered users.

**Response:** `200 JSON`

```json
[
  { "id": 1, "username": "alice", "role": "admin" },
  { "id": 2, "username": "bob",   "role": "lender" }
]
```

| Field      | Type   | Description                    |
|------------|--------|--------------------------------|
| `id`       | number | User primary key               |
| `username` | string | Username                       |
| `role`     | string | `admin` or `lender`            |

---

#### `GET /admin/api/books`

Returns all books with a computed status string.

**Response:** `200 JSON`

```json
[
  {
    "bookid": 1,
    "title": "The Rust Programming Language",
    "author": "Steve Klabnik",
    "isbn": "9781593278281",
    "year_of_pub": 2019,
    "genre": "Programming",
    "total_copies": 5,
    "available_copies": 3,
    "status": "3 available, 2 checked out"
  }
]
```

| Field              | Type         | Description                                  |
|--------------------|--------------|----------------------------------------------|
| `bookid`           | number       | Book primary key                             |
| `title`            | string       | Title                                        |
| `author`           | string       | Author                                       |
| `isbn`             | string       | ISBN (unique)                                |
| `year_of_pub`      | number/null  | Publication year                             |
| `genre`            | string/null  | Genre                                        |
| `total_copies`     | number       | Total copies in the library                  |
| `available_copies` | number       | Copies not currently checked out             |
| `status`           | string       | Computed: `"X available, Y checked out"`     |

---

#### `POST /admin/api/books`

Adds a new book. If a book with the same ISBN already exists, its copy count is increased instead.

**Content-Type:** `application/json`

**Request body:**

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

| Field         | Type         | Required | Notes                        |
|---------------|--------------|----------|------------------------------|
| `title`       | string       | Yes      | Must not be empty            |
| `author`      | string       | Yes      | Must not be empty            |
| `isbn`        | string       | Yes      | Must not be empty            |
| `year_of_pub` | number/null  | No       |                              |
| `genre`       | string/null  | No       |                              |
| `copies`      | number       | Yes      | Must be > 0                  |

**Responses:**

| Status | Condition                     | Body                                  |
|--------|-------------------------------|---------------------------------------|
| 200    | New book created              | `Book added successfully`             |
| 200    | ISBN exists, copies increased | `Book exists - copies increased`      |
| 200    | Validation failed             | `Invalid book data`                   |

---

#### `PUT /admin/api/books?bookid=<id>`

Updates an existing book's metadata and total copy count. `available_copies` is recalculated automatically as `new_total - currently_checked_out`, so active loans are never lost.

**Query parameter:**

| Param    | Type   | Required |
|----------|--------|----------|
| `bookid` | number | Yes      |

**Content-Type:** `application/json`

**Request body:** Same structure as `POST /admin/api/books`.

**Responses:**

| Status | Condition                                          | Body                                                            |
|--------|----------------------------------------------------|-----------------------------------------------------------------|
| 200    | Success                                            | `Book updated successfully`                                     |
| 200    | `copies` < number currently checked out            | `Cannot reduce total copies below number currently checked out` |
| 200    | `bookid` param missing or not a valid number       | `Missing bookid`                                                |

---

#### `DELETE /admin/api/books?bookid=<id>`

Deletes a book. Blocked if any active (unreturned) loans exist for that book.

**Query parameter:**

| Param    | Type   | Required |
|----------|--------|----------|
| `bookid` | number | Yes      |

**Responses:**

| Status | Condition                              | Body                                          |
|--------|----------------------------------------|-----------------------------------------------|
| 200    | Success                                | `Book deleted`                                |
| 200    | Book has unreturned loans              | `Cannot delete: book is currently loaned`     |
| 200    | `bookid` param missing or invalid      | `Missing bookid`                              |

---

#### `GET /admin/api/loans`

Returns all loans (active and returned) with a computed status field.

**Response:** `200 JSON`

```json
[
  {
    "loanid": 1,
    "username": "bob",
    "title": "The Rust Programming Language",
    "checkout_date": "2026-01-30",
    "due_date": "2026-02-13",
    "status": "Borrowed"
  }
]
```

| Field           | Type   | Description                                            |
|-----------------|--------|--------------------------------------------------------|
| `loanid`        | number | Loan primary key                                       |
| `username`      | string | Borrower's username                                    |
| `title`         | string | Book title                                             |
| `checkout_date` | string | `YYYY-MM-DD`                                           |
| `due_date`      | string | `YYYY-MM-DD`                                           |
| `status`        | string | `Borrowed`, `Overdue`, or `Returned` (see below)       |

**Status logic:**
- `return_date` is set → `Returned`
- `due_date < today` and no return → `Overdue`
- Otherwise → `Borrowed`

---

#### `GET /admin/api/overdue`

Returns all currently overdue loans (unreturned and past due date).

**Response:** `200 JSON`

```json
[
  {
    "username": "bob",
    "title": "The Rust Programming Language",
    "due_date": "2026-01-15",
    "days_overdue": 16
  }
]
```

| Field            | Type   | Description                          |
|------------------|--------|--------------------------------------|
| `username`       | string | Borrower's username                  |
| `title`          | string | Book title                           |
| `due_date`       | string | `YYYY-MM-DD`                         |
| `days_overdue`   | number | Number of days past the due date     |

---

### Lender API

All endpoints below require a valid session with `role = lender`. Unauthorized requests return an empty array `[]` or an HTML error message.

---

#### `GET /lender/api/books`

Returns all books that have at least one available copy.

**Response:** `200 JSON`

```json
[
  {
    "bookid": 1,
    "title": "The Rust Programming Language",
    "author": "Steve Klabnik",
    "genre": "Programming",
    "available_copies": 3
  }
]
```

| Field              | Type   | Description                    |
|--------------------|--------|--------------------------------|
| `bookid`           | number | Book primary key               |
| `title`            | string | Title                          |
| `author`           | string | Author                         |
| `genre`            | string | Genre                          |
| `available_copies` | number | Copies currently available     |

---

#### `GET /lender/api/search?q=<term>`

Searches available books. Matches against `title`, `author`, `isbn`, or `genre` using a case-insensitive partial match (`LIKE %term%`).

**Query parameter:**

| Param | Type   | Required | Notes                              |
|-------|--------|----------|------------------------------------|
| `q`   | string | Yes      | Partial match, any of the 4 fields |

**Response:** `200 JSON` — same shape as `GET /lender/api/books`. Returns `[]` if nothing matches or if `q` is missing.

---

#### `GET /lender/api/myloans`

Returns the logged-in user's active loans that are **not yet overdue** (`due_date >= today` and `return_date IS NULL`). Overdue loans appear under `/lender/api/overdue` instead.

**Response:** `200 JSON`

```json
[
  {
    "loanid": 1,
    "title": "The Rust Programming Language",
    "checkout_date": "2026-01-30",
    "due_date": "2026-02-13",
    "status": "Borrowed"
  }
]
```

| Field           | Type   | Description                                    |
|-----------------|--------|------------------------------------------------|
| `loanid`        | number | Loan primary key                               |
| `title`         | string | Book title                                     |
| `checkout_date` | string | `YYYY-MM-DD`                                   |
| `due_date`      | string | `YYYY-MM-DD`                                   |
| `status`        | string | Always `Borrowed` for this endpoint            |

---

#### `POST /lender/api/checkout?bookid=<id>`

Checks out a book for the logged-in user. The entire operation (loan insert + copy decrement) runs inside a single database transaction. Loan duration is fixed at **14 days**.

**Query parameter:**

| Param    | Type   | Required |
|----------|--------|----------|
| `bookid` | number | Yes      |

**Responses:**

| Status | Condition                                          | Body                                              |
|--------|----------------------------------------------------|---------------------------------------------------|
| 200    | Success                                            | `<h1>Checkout successful</h1>`                    |
| 200    | User already has an active loan for this book      | `<h1>You already borrowed this book</h1>`         |
| 200    | No copies available                                | `<h1>Book not available</h1>`                     |
| 200    | `bookid` does not exist in `books`                 | `<h1>Book not found</h1>`                         |
| 200    | Session user not found in `users`                  | `<h1>User not found</h1>`                         |
| 200    | No valid session                                   | `<h1>Not logged in</h1>`                          |
| 200    | `bookid` param missing or invalid                  | `<h1>Invalid checkout request</h1>`               |

---

#### `POST /lender/api/return?loanid=<id>`

Returns a previously checked-out book. The entire operation (set `return_date` + increment copy count) runs inside a single database transaction.

**Query parameter:**

| Param    | Type   | Required |
|----------|--------|----------|
| `loanid` | number | Yes      |

**Responses:**

| Status | Condition                         | Body                                        |
|--------|-----------------------------------|---------------------------------------------|
| 200    | Success                           | `<h1>Return successful</h1>`                |
| 200    | No valid session                  | `<h1>Not logged in</h1>`                    |
| 200    | `loanid` param missing or invalid | `<h1>Invalid return request</h1>`           |

---

#### `GET /lender/api/overdue`

Returns the logged-in user's overdue loans (unreturned and past due date).

**Response:** `200 JSON`

```json
[
  {
    "title": "The Rust Programming Language",
    "due_date": "2026-01-15",
    "days_overdue": 16
  }
]
```

| Field            | Type   | Description                          |
|------------------|--------|--------------------------------------|
| `title`          | string | Book title                           |
| `due_date`       | string | `YYYY-MM-DD`                         |
| `days_overdue`   | number | Number of days past the due date     |

