# Учебный разбор backend Vivec: от Hello World до сайта

Этот гайд объясняет backend проекта строка за строкой. Он рассчитан на человека,
который знает только примерно такой Rust:

```rust
fn main() {
    println!("Hello, world!");
}
```

Не нужно знать заранее axum, SQL, async, cookie или HTTP. Новые понятия сначала
объясняются обычными словами, затем показывается место в коде Vivec.

Перед чтением открой в редакторе `TODO.md` и файл, который разбирается. Лучше
читать по одному разделу и после каждого запускать `cargo check`.

---

## 1. Что именно делает backend

Backend Vivec — это программа `vivec`, которая:

1. принимает HTTP-запрос от браузера;
2. понимает URL и метод запроса;
3. иногда проверяет cookie;
4. читает или меняет SQLite;
5. собирает HTML либо JSON;
6. возвращает браузеру ответ и HTTP-код.

Публичная газета получает готовый HTML от Rust. Vue-админка получает JSON от
Rust и уже сама меняет экран.

```text
Публичная страница:
браузер → GET / → Rust handler → SQLite → HTML

Админка:
браузер → GET /admin/ → готовый Vue JS/CSS → JSON API → Rust → SQLite
```

На сервере Node.js не работает. Vite собирает Vue локально в статические файлы;
Rust раздаёт результаты как обычные файлы.

---

## 2. Карта backend-файлов

| Файл | Работа |
|---|---|
| `Cargo.toml` | какие библиотеки подключены |
| `src/main.rs` | вход, настройка и таблица HTTP-маршрутов |
| `src/db.rs` | открытие SQLite и создание таблиц |
| `src/articles.rs` | чтение статей и публичный поиск |
| `src/markdown.rs` | Markdown → безопасный HTML |
| `src/slug.rs` | русский заголовок → латинский URL |
| `src/error.rs` | превращение Rust-ошибки в HTTP-ответ |
| `src/auth.rs` | пароль, сессии, cookie, вход |
| `src/api.rs` | JSON CRUD для админки |
| `src/uploads.rs` | загрузка изображений |
| `src/bin/hash-password.rs` | отдельная утилита для хеша пароля |
| `migrations/*.sql` | структура таблиц и демо-данные |
| `templates/*.jinja` | HTML-шаблоны публичной части |

Правильный порядок первого чтения:

```text
Cargo.toml
→ migrations
→ markdown.rs
→ slug.rs
→ db.rs
→ error.rs
→ articles.rs
→ main.rs
→ auth.rs
→ api.rs
→ uploads.rs
→ hash-password.rs
```

---

## 3. Rust-минимум на примерах Vivec

### 3.1 `fn main()` — точка входа

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // запуск приложения
    Ok(())
}
```

Обычная программа начинается с `main`. Vivec — сервер, поэтому `main` не
завершается сразу, а ждёт запросы.

- `fn` — объявление функции.
- `main` — имя, которое ищет Rust.
- `async` — функция может ждать, не блокируя весь сервер.
- `-> anyhow::Result<()>` — функция вернёт либо успех `Ok(())`, либо ошибку.
- `#[tokio::main]` — служебная аннотация tokio, которая создаёт async runtime.

`()` читается как unit. Это пустое значение: «успех без полезного результата».

### 3.2 `use` — сокращение пути

```rust
use sqlx::sqlite::SqlitePool;
```

Без `use` пришлось бы каждый раз писать полный путь
`sqlx::sqlite::SqlitePool`.

В `main.rs` строки `use axum::{...}` импортируют несколько типов и функций
из одного ящика (crate).

### 3.3 `mod` — подключение своего файла

```rust
mod auth;
```

Если написать это в `src/main.rs`, Rust ищет `src/auth.rs`. Внутри `auth.rs`
функции вызываются как `auth::login`, `auth::logout`.

### 3.4 `struct` — набор связанных полей

```rust
struct App {
    pool: SqlitePool,
    today: String,
}
```

Это не объект с методами, а простая запись данных. У `App` четыре поля:
шаблоны, база, auth и текущая дата.

### 3.5 `Option<T>` — значение есть или отсутствует

```rust
Option<String>
```

Означает одно из двух:

```text
Some("текст")
None
```

Пример Vivec: у черновика `published_at = None`. Компилятор не даст случайно
использовать дату так, будто она точно есть.

### 3.6 `Result<T, E>` — успех или ошибка

```rust
Result<String, AppError>
```

Означает:

```text
Ok("готовый HTML")
Err(AppError::NotFound)
```

Хендлер HTTP почти всегда возвращает `Result`, потому что база, файл или шаблон
могут дать ошибку.

### 3.7 Оператор `?`

```rust
let pool = db::open().await?;
```

Читается так:

1. запустить `db::open().await`;
2. если результат `Ok(pool)` — продолжить;
3. если `Err(e)` — немедленно вернуть ошибку из текущей функции.

`?` не скрывает ошибку. Он передаёт её тому, кто вызвал функцию.

### 3.8 `match` — обработать все варианты

```rust
match result {
    Ok(article) => Json(article),
    Err(error) => return Err(error.into()),
}
```

Компилятор требует обработать каждый вариант. Нельзя случайно забыть ошибку.

### 3.9 Замыкание

```rust
list.retain(|article| article.published);
```

`|article| ...` — маленькая функция без имени. `retain` вызовет её для каждого
элемента и оставит только те, где результат `true`.

### 3.10 Итератор и `collect`

```rust
rows.into_iter()
    .map(to_article)
    .collect::<sqlx::Result<Vec<_>>>()
```

- `into_iter()` забирает строки по одной;
- `map(to_article)` превращает каждую SQL-строку в `Article`;
- `collect` собирает всё в `Vec`;
- тип `sqlx::Result` означает, что одна плохая строка делает весь результат ошибкой.

### 3.11 Владение

`String` владеет текстом в памяти. `&str` только смотрит на чужой текст.

```rust
fn trim_title(title: &str) -> String {
    title.trim().to_string()
}
```

Функция не забирает строку у вызывающего, а создаёт новую. В Vivec `&app.pool`
означает «функция использует пул, но не уничтожает его».

### 3.12 `Clone`

```rust
#[derive(Clone)]
struct App { /* ... */ }
```

Это автоматически создаёт `.clone()`. axum даёт каждому запросу копию `App`.
`SqlitePool` и `Arc` внутри клонируются дешёво: копируется ссылка на общие данные.

### 3.13 `async` и `await`

База и файл отвечают не мгновенно. `async` позволяет tokio во время ожидания
обслуживать другие запросы.

`await` не равен `sleep`: он отдаёт управление runtime до готовности операции.

### 3.14 Derive

```rust
#[derive(Serialize)]
struct UserResponse { username: String }
```

`derive(Serialize)` генерирует код, который превращает структуру в JSON.
Это стандартная возможность serde, а не ручной парсинг.

---

## 4. `Cargo.toml` строка за строкой

### Строки 1–4

```toml
[package]
name = "vivec"
version = "0.1.0"
edition = "2024"
```

- `[package]` начинает настройки приложения.
- `name` задаёт имя бинарника и пакета.
- `version` — версия проекта.
- `edition = "2024"` включает правила синтаксиса Rust 2024 года.

### Строки 6–8

```toml
[dependencies]
ammonia = "4.2.5"
anyhow = "1.0.104"
```

- `[dependencies]` начинает список библиотек.
- `ammonia` очищает HTML от опасных тегов.
- `anyhow` удобен для ошибок запуска в `main`; в HTTP-хендлерах он запрещён.

### Строки 9–11

```toml
argon2 = { version = "0.6.0", features = ["getrandom"] }
axum = { version = "0.8.9", features = ["multipart"] }
axum-extra = { version = "0.12.6", features = ["cookie"] }
```

- `argon2` хеширует и проверяет пароль.
- `features = ["getrandom"]` разрешает Argon2 брать случайную соль.
- `axum` принимает HTTP и вызывает хендлеры.
- `multipart` добавляет разбор загрузки файлов.
- `axum-extra` расширяет axum cookie-инструментами.
- `cookie` feature включает `CookieJar`.

### Строки 12–14

```toml
cookie = "0.18.2"
getrandom = "0.4.3"
infer = "0.22.0"
```

- `cookie` нужен для настройки `Max-Age` cookie.
- `getrandom` создаёт криптографически случайные токены и имена файлов.
- `infer` определяет тип файла по первым байтам, а не по расширению.

### Строки 15–17

```toml
minijinja = { version = "3.1.0", features = ["serde"] }
pulldown-cmark = { version = "0.13.6", default-features = false, features = ["html"] }
serde = { version = "1.0.229", features = ["derive"] }
```

- `minijinja` заполняет HTML-шаблоны.
- `serde` feature позволяет передавать Rust-структуры в шаблоны.
- `pulldown-cmark` превращает Markdown в HTML.
- `default-features = false` отключает лишнее.
- `features = ["html"]` обязательно включает HTML-вывод.
- `serde` превращает структуры в JSON и обратно.
- `derive` включает `#[derive(Serialize, Deserialize)]`.

### Строки 18–20

```toml
sha2 = "0.11.1"
sqlx = { version = "0.9.0", default-features = false, features = ["runtime-tokio", "sqlite"] }
tokio = { version = "1.54.1", features = ["rt-multi-thread", "macros", "net", "signal", "fs"] }
```

- `sha2` считает SHA-256 от токена сессии.
- `sqlx` работает с SQLite.
- `runtime-tokio` связывает sqlx с async runtime.
- `sqlite` включает SQLite-драйвер.
- `tokio` запускает async-код.
- `rt-multi-thread` создаёт многопоточный runtime.
- `macros` включает `#[tokio::main]`.
- `net` включает TCP-сервер.
- `signal` включает Ctrl+C и SIGTERM.
- `fs` включает асинхронную работу с файлами.

### Строки 21–23

```toml
tower-http = { version = "0.7.1", features = ["trace", "fs", "set-header"] }
tracing = "0.1.44"
tracing-subscriber = { version = "0.3.23", features = ["env-filter"] }
```

- `trace` пишет HTTP-запросы в лог.
- `fs` даёт `ServeDir` и `ServeFile`.
- `set-header` добавляет security headers.
- `tracing` создаёт события логов.
- `tracing-subscriber` выводит их в терминал.
- `env-filter` позволяет управлять логами через `RUST_LOG`.

---

## 5. SQL-схема: `migrations/0001_init.sql`

### Строки 1–5

```sql
-- Схема Vivec, версия 1. Применяется при старте: CREATE IF NOT EXISTS,
-- поэтому повторный запуск ничего не ломает.
--
-- Даты — TEXT 'ГГГГ-ММ-ДД': sqlx отдаёт их строкой, форматируем в Rust.
-- chrono в типах не держим (правило проекта).
```

Всё это SQL-комментарии. После `--` база игнорирует строку. Комментарии
объясняют решение: повторный запуск безопасен, дата хранится текстом.

### Строки 7–18: таблица статей

```sql
CREATE TABLE IF NOT EXISTS articles (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    slug         TEXT    NOT NULL UNIQUE,
    title        TEXT    NOT NULL,
    rubric       TEXT    NOT NULL DEFAULT 'Школа',
    author       TEXT    NOT NULL DEFAULT '',
    lead         TEXT    NOT NULL DEFAULT '',
    body_md      TEXT    NOT NULL DEFAULT '',
    published    INTEGER NOT NULL DEFAULT 0,
    published_at TEXT,
    created_at   TEXT    NOT NULL DEFAULT (datetime('now'))
);
```

- `CREATE TABLE IF NOT EXISTS` создаёт таблицу только при первом запуске.
- `articles` — имя таблицы.
- `id INTEGER PRIMARY KEY` делает уникальный числовой идентификатор.
- `AUTOINCREMENT` не переиспользует старые номера после удаления.
- `slug TEXT UNIQUE` не даёт двум статьям получить один URL.
- `title TEXT NOT NULL` запрещает заголовок `NULL`.
- `rubric` по умолчанию получает `Школа`.
- `author` по умолчанию пустой.
- `lead` по умолчанию пустой.
- `body_md` хранит исходный Markdown.
- `published=0` означает черновик.
- `published_at` может быть `NULL`, пока статья не опубликована.
- `created_at` автоматически получает текущую дату и время SQLite.

### Строки 20–22: индекс

```sql
CREATE INDEX IF NOT EXISTS idx_articles_published
    ON articles (published, published_at DESC);
```

- индекс ускоряет фильтр `WHERE published = 1`;
- затем SQLite быстро сортирует свежие опубликованные записи;
- `DESC` означает «сначала новые».

---

## 6. Сессии: `migrations/0002_admin.sql`

### Строки 1–8

```sql
CREATE TABLE IF NOT EXISTS sessions (
    token_hash TEXT PRIMARY KEY,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT NOT NULL
);
```

- `sessions` хранит входы, а не пароли.
- `token_hash` — SHA-256 случайного токена и одновременно первичный ключ.
- `created_at` записывает, когда вход создан.
- `expires_at` задаёт конец срока действия.
- В cookie едет исходный токен; в базе лежит только его хеш.

### Строки 10–11

```sql
CREATE INDEX IF NOT EXISTS idx_sessions_expires
    ON sessions (expires_at);
```

Индекс ускоряет удаление просроченных сессий и проверку срока.

---

## 7. Seed: `migrations/seed.sql`

### Строки 1–4

Комментарии объясняют:

- seed нужен для разработки;
- выдуманные статьи нельзя автоматически грузить на production;
- `INSERT OR IGNORE` не дублирует статьи при повторном запуске;
- `body_md` содержит настоящий Markdown.

### Строки 6–7

```sql
INSERT OR IGNORE INTO articles
    (slug, title, rubric, author, lead, body_md, published, published_at)
VALUES
```

- `INSERT` добавляет строки.
- `OR IGNORE` пропускает конфликт по уникальному slug.
- В скобках перечислены заполняемые колонки.
- `id` и `created_at` SQLite создаст сама.

### Строки 8–55

Каждый блок — одна демонстрационная статья:

- первый текст в скобках — slug;
- второй — заголовок;
- третий — рубрика;
- четвёртый — автор;
- пятый — лид;
- шестой — Markdown;
- `1` означает опубликована;
- последняя строка — дата публикации.

Символы `##`, `>`, `-` и `**` являются Markdown и будут превращены
`pulldown-cmark` в HTML.

---

## 8. `src/markdown.rs` — Markdown в безопасный HTML

### Строка 1

```rust
use pulldown_cmark::{Options, Parser, html};
```

- `Options` хранит включённые расширения Markdown.
- `Parser` читает Markdown.
- `html` содержит функцию, которая пишет HTML в строку.

### Строки 3–8

Это комментарий. Он объясняет главную угрозу: Markdown-парсер пропускает
сырой HTML, поэтому без очистки автор мог бы вставить `<script>`.

Ammonia — второй шаг защиты. Сначала Markdown превращается в HTML, затем
опасные элементы удаляются.

### Строки 9–10

```rust
pub fn to_html(md: &str) -> String {
```

- `pub` разрешает вызывать функцию из другого модуля.
- `fn to_html` объявляет функцию.
- `md: &str` принимает ссылку на Markdown без копирования.
- `-> String` возвращает новый HTML-текст.

### Строки 10–12

```rust
let mut opts = Options::empty();
opts.insert(Options::ENABLE_TABLES);
opts.insert(Options::ENABLE_STRIKETHROUGH);
```

- `Options::empty()` начинает пустой набор возможностей.
- `mut` разрешает менять набор.
- `ENABLE_TABLES` разрешает Markdown-таблицы.
- `ENABLE_STRIKETHROUGH` разрешает `~~зачёркнутый текст~~`.

### Строки 14–15

```rust
let mut html = String::new();
html::push_html(&mut html, Parser::new_ext(md, opts));
```

- `html` — изменяемая строка результата.
- `String::new()` создаёт пустую строку.
- `Parser::new_ext` создаёт парсер с выбранными опциями.
- `&mut html` разрешает функции дописать HTML внутрь строки.
- `push_html` выполняет преобразование.

### Строки 17–18

```rust
ammonia::clean(&html)
}
```

- `ammonia::clean` читает готовый HTML.
- возвращается новая безопасная строка;
- закрывающая скобка завершает функцию.

Без `ammonia` строка называлась бы `html`, но не была бы безопасной.

---

## 9. `src/slug.rs` — URL из русского текста

### Строки 1–3

Комментарий объясняет назначение:

```text
Школа → shkola
День самоуправления → den-samoupravleniya
```

Slug должен содержать только ASCII-буквы, цифры и дефисы.

### Строка 4

```rust
const MAP: [(char, &str); 33] = [
```

- `const` — значение фиксировано на этапе компиляции.
- тип — массив из 33 элементов.
- каждый элемент — пара `(char, &str)`.
- первый член пары: кириллический символ.
- второй член: его латинская замена.

### Строки 5–37: таблица замен

Каждая строка имеет один и тот же смысл:

| Русская буква | Замена |
|---|---|
| `а` | `a` |
| `б` | `b` |
| `в` | `v` |
| `г` | `g` |
| `д` | `d` |
| `е`, `ё` | `e` |
| `ж` | `zh` |
| `з` | `z` |
| `и` | `i` |
| `й` | `y` |
| `к` | `k` |
| `л` | `l` |
| `м` | `m` |
| `н` | `n` |
| `о` | `o` |
| `п` | `p` |
| `р` | `r` |
| `с` | `s` |
| `т` | `t` |
| `у` | `u` |
| `ф` | `f` |
| `х` | `h` |
| `ц` | `c` |
| `ч` | `ch` |
| `ш` | `sh` |
| `щ` | `sch` |
| `ъ`, `ь` | пустая строка |
| `ы` | `y` |
| `э` | `e` |
| `ю` | `yu` |
| `я` | `ya` |

Твёрдый и мягкий знаки удаляются, потому что в латинском URL им нет
простой однобуквенной замены.

### Строки 40–43

```rust
pub fn translit(text: &str) -> String {
```

Функция публичная, принимает текст по ссылке и возвращает новый slug.

### Строки 44–45

```rust
let mut out = String::with_capacity(text.len());
let mut dash = false;
```

- `out` будет результатом.
- `with_capacity` заранее выделяет память.
- `dash` помнит, был ли только что добавлен дефис.
- Флаг не даёт получить `---` на месте нескольких пробелов.

### Строка 47

```rust
for ch in text.to_lowercase().chars() {
```

- `to_lowercase()` делает `Ш` строчной.
- `chars()` идёт по Unicode-символам.
- `ch` — один символ на каждой итерации.

### Строки 48–50

```rust
if let Some((_, lat)) = MAP.iter().find(|(c, _)| *c == ch) {
    out.push_str(lat);
    dash = false;
}
```

- `MAP.iter()` перебирает пары.
- замыкание `|(c, _)| *c == ch` ищет русскую букву.
- `_` говорит, что латинское значение здесь пока не нужно.
- `Some((_, lat))` извлекает замену.
- `push_str` дописывает одну или несколько латинских букв.
- `dash = false` означает, что следующий знак снова может поставить дефис.

### Строки 51–53

```rust
else if ch.is_ascii_alphanumeric() {
    out.push(ch);
    dash = false;
}
```

Если символ уже латинская буква или цифра, он сохраняется как есть.

### Строки 54–59

```rust
else if !dash && !out.is_empty() {
    out.push('-');
    dash = true;
}
```

Любой другой символ считается разделителем.

- `!dash` запрещает второй дефис подряд.
- `!out.is_empty()` запрещает дефис в начале.
- после добавления флаг становится `true`.

### Строки 62–65

```rust
while out.ends_with('-') {
    out.pop();
}
out
```

- если строка кончается дефисом, `pop` удаляет последний символ;
- цикл повторяется, пока хвостовых дефисов нет;
- последнее выражение без `return` становится результатом функции.

### Строки 68–90: тесты

`#[cfg(test)]` означает, что модуль тестов компилируется только при
`cargo test`.

- `use super::translit` импортирует функцию из родительского файла.
- `#[test]` регистрирует тест.
- `assert_eq!` проверяет полное совпадение.
- первый тест закрепляет slug всех четырёх рубрик;
- второй проверяет длинный заголовок, кавычки, пробелы, `ё`, цифры и пустой результат.

---

## 10. `src/db.rs` — открыть базу

### Строки 1–3

```rust
use std::str::FromStr;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool};
```

- `FromStr` позволяет превратить строку URL в настройки SQLite.
- `SqliteConnectOptions` хранит путь и параметры подключения.
- `SqliteJournalMode` задаёт режим журнала.
- `SqlitePool` — пул переиспользуемых соединений.

### Строки 5–8

```rust
const SCHEMA: &str = include_str!("../migrations/0001_init.sql");
const ADMIN_SCHEMA: &str = include_str!("../migrations/0002_admin.sql");
```

- `include_str!` читает SQL-файл во время компиляции.
- содержимое попадает внутрь бинарника;
- `&str` означает, что это статическая строка;
- при деплое отдельный SQL-файл не может потеряться.

### Строка 12

```rust
pub async fn open() -> anyhow::Result<SqlitePool> {
```

`open` может ждать диск и сетевые операции, поэтому `async`. Результат —
пул либо ошибка запуска.

### Строки 13–14

```rust
let dir = data_dir();
std::fs::create_dir_all(&dir)?;
```

- `data_dir()` читает env или возвращает `data`;
- `create_dir_all` создаёт каталог и родителей;
- если каталог уже есть, ошибки нет;
- `?` возвращает ошибку в `main`.

### Строка 16

```rust
let url = format!("sqlite://{dir}/vivec.db");
```

`format!` подставляет путь в строку подключения. Для `/data` получится
`sqlite:///data/vivec.db`.

### Строки 17–20

```rust
let opts = SqliteConnectOptions::from_str(&url)?
    .create_if_missing(true)
    .journal_mode(SqliteJournalMode::Wal);
```

- строка URL разбирается в настройки;
- `create_if_missing(true)` создаёт файл базы;
- `Wal` включает write-ahead log;
- в WAL читатели и писатель меньше блокируют друг друга.

### Строки 22–25

```rust
let pool = SqlitePool::connect_with(opts).await?;
sqlx::raw_sql(SCHEMA).execute(&pool).await?;
sqlx::raw_sql(ADMIN_SCHEMA).execute(&pool).await?;
tracing::info!("база готова: {dir}/vivec.db");
```

- `connect_with` создаёт пул;
- первая схема создаёт `articles`;
- вторая создаёт `sessions`;
- `IF NOT EXISTS` позволяет выполнять их на каждом старте;
- `tracing::info` пишет путь в лог.

Перед логом вызывается `seed_demo_if_empty`:

- `VIVEC_SEED_DEMO=1` включает тестовый режим;
- функция считает статьи;
- демо-SQL выполняется только для пустой таблицы;
- обычный production запуск без env не меняет данные.

### Строки 26–31

```rust
Ok(pool)
}

pub fn data_dir() -> String {
    std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string())
}
```

- функция отдаёт готовый пул;
- `data_dir` читает переменную окружения;
- `unwrap_or_else` при отсутствии env создаёт `"data"`;
- публичность нужна `uploads.rs`, чтобы положить файлы рядом с базой.

Этот код не использует `sqlx::migrate!`: миграции простые и совместимы
с повторным запуском. При будущем изменении колонок `ALTER TABLE` нужно
перейти на полноценный migration runner.

---

## 11. `src/error.rs` — одна ошибка для всех хендлеров

### Строки 1–6

```rust
use axum::{Json, http::StatusCode, response::{IntoResponse, Response}};
use serde::Serialize;
```

- `Json` формирует JSON-ответ;
- `StatusCode` содержит HTTP-код;
- `IntoResponse` умеет превращать тип в ответ;
- `Response` — итоговый HTTP-ответ;
- `Serialize` разрешает превратить структуру ошибки в JSON.

### Строки 8–22: enum ошибок

```rust
#[derive(Debug)]
pub enum AppError {
    Template(minijinja::Error),
    Db(sqlx::Error),
    BadRequest(&'static str),
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict(&'static str),
    TooManyRequests,
    Unavailable(&'static str),
    Internal(String),
}
```

- `Debug` печатает ошибку в отладочном виде.
- `Template` хранит ошибку minijinja.
- `Db` хранит ошибку SQLx.
- `BadRequest` хранит постоянный текст причины.
- `Unauthorized` означает отсутствие входа.
- `Forbidden` означает запрещённое действие.
- `NotFound` означает отсутствие записи.
- `Conflict` описывает нарушение уникальности.
- `TooManyRequests` включает лимит входа.
- `Unavailable` означает, что функция не настроена.
- `Internal(String)` хранит внутреннюю диагностическую строку.

HTTP-код здесь ещё не записан. Enum хранит смысл ошибки.

### Строки 24–27

```rust
#[derive(Serialize)]
struct ErrorBody {
    error: &'static str,
}
```

Это форма JSON:

```json
{"error":"текст"}
```

Поле приватное, но serde сериализует его внутри текущего модуля.

### Строки 29–34

```rust
impl From<minijinja::Error> for AppError {
    fn from(e: minijinja::Error) -> Self {
        AppError::Template(e)
    }
}
```

`From` разрешает оператору `?` автоматически преобразовать ошибку шаблона
в `AppError`.

### Строки 36–41

```rust
impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Db(e)
    }
}
```

То же для ошибки базы. Без этого пришлось бы писать `.map_err(...)` у
каждого SQL-запроса.

### Строки 43–47

```rust
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (code, msg) = match self {
```

`IntoResponse` — контракт axum: «этот тип знает, как стать HTTP-ответом».

`self` здесь забирается по значению. `match` превращает вариант ошибки
в пару: HTTP-код и безопасное сообщение.

### Строки 48–55

```rust
AppError::Template(e) => {
    tracing::error!("template error: {e}");
    (StatusCode::INTERNAL_SERVER_ERROR, "500")
}
AppError::Db(e) => {
    tracing::error!("db error: {e}");
    (StatusCode::INTERNAL_SERVER_ERROR, "500")
}
```

Подробная техническая ошибка пишется в лог. Клиент получает только `500`.
Путь к базе и SQL не утекают наружу.

### Строки 56–66

Каждый вариант получает свой HTTP-код:

- `BadRequest` → `400`;
- `Unauthorized` → `401`;
- `Forbidden` → `403`;
- `NotFound` → `404`;
- `Conflict` → `409`;
- `TooManyRequests` → `429`;
- `Unavailable` → `503`;
- `Internal` → лог плюс `500`.

### Строки 68–72

```rust
if code.is_client_error() || code == StatusCode::SERVICE_UNAVAILABLE {
    (code, Json(ErrorBody { error: msg })).into_response()
} else {
    (code, msg).into_response()
}
```

- клиентские ошибки и `503` возвращаются JSON;
- `500` возвращается короткой строкой;
- кортеж `(StatusCode, тело)` axum умеет превращать в ответ.

Vue в `api.js` ожидает именно JSON с полем `error`.

---

## 12. `src/articles.rs` — чтение публичных статей

### Строки 1–4

```rust
use serde::Serialize;
use sqlx::{Row, sqlite::SqlitePool};
use crate::{markdown, slug};
```

- `Serialize` позволяет передать статью в minijinja.
- `Row` даёт метод `try_get` для чтения колонок.
- `SqlitePool` нужен каждой функции чтения.
- `markdown` превращает тело в безопасный HTML.
- `slug` строит латинский slug рубрики.

### Строки 6–14: список рубрик

```rust
pub const RUBRICS: [(&str, &str); 4] = [
    ("Школа", "shkola"),
    ("Спорт", "sport"),
    ("Культура", "kultura"),
    ("Мнения", "mneniya"),
];
```

- массив публичный, потому что его использует API-валидация;
- тип означает четыре пары строк;
- первая строка пары показывается человеку;
- вторая используется в URL;
- фиксированный список не даёт открыть `/rubric/что-угодно`.

### Строка 16

```rust
pub const SEARCH_LIMIT: usize = 20;
```

Ограничение выдачи. Даже если подходящих статей больше, API/шаблон получает
не более 20.

### Строки 18–24: `rubric_name`

```rust
pub fn rubric_name(rubric_slug: &str) -> Option<&'static str> {
    RUBRICS
        .iter()
        .find(|(_, s)| *s == rubric_slug)
        .map(|(name, _)| *name)
}
```

- вход — slug из URL;
- выход — русское имя или `None`;
- `iter` перебирает массив;
- `find` ищет пару по второму элементу;
- `*s` получает `&str` из ссылки на пару;
- `map` превращает найденную пару в имя;
- `'static` значит, что имя лежит в константе и живёт всю программу.

### Строки 26–40: `Article`

```rust
#[derive(Serialize)]
pub struct Article {
    pub slug: String,
    pub title: String,
    pub rubric: String,
    pub rubric_slug: String,
    pub author: String,
    pub lead: String,
    pub body_html: String,
    pub date: String,
}
```

- `slug` используется в `/article-slug`;
- `title` показывается как заголовок;
- `rubric` — русское имя;
- `rubric_slug` — латинская часть `/rubric/...`;
- `author` и `lead` выводятся в карточке;
- `body_html` уже безопасен;
- `date` уже отформатирована по-русски;
- все поля публичные, чтобы шаблон и другие модули могли их читать.

### Строки 42–55: `published`

```rust
pub async fn published(pool: &SqlitePool) -> sqlx::Result<Vec<Article>> {
    let rows = sqlx::query(
        "SELECT slug, title, rubric, author, lead, body_md, published_at
           FROM articles
          WHERE published = 1
          ORDER BY published_at DESC, id DESC",
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(to_article).collect()
}
```

- функция асинхронная, потому что ждёт SQLite;
- `pool` только заимствуется;
- `SELECT` перечисляет ровно нужные колонки;
- `published = 1` исключает черновики;
- `published_at DESC` ставит новые сверху;
- `id DESC` разрешает одинаковые даты;
- `fetch_all` получает все строки;
- `await?` ждёт и передаёт ошибку;
- `map(to_article)` превращает SQL-строки в структуры;
- `collect()` собирает итоговый `Vec`.

### Строки 57–69: `by_slug`

```rust
pub async fn by_slug(pool: &SqlitePool, slug: &str) -> sqlx::Result<Option<Article>> {
    let row = sqlx::query(
        "SELECT slug, title, rubric, author, lead, body_md, published_at
           FROM articles
          WHERE slug = ? AND published = 1",
    )
    .bind(slug)
    .fetch_optional(pool)
    .await?;

    row.map(to_article).transpose()
}
```

- вход: пул и slug;
- выход: `Some(Article)`, `None` или SQL-ошибка;
- `?` — место параметра; значение передаётся через `.bind`;
- `published = 1` не даёт открыть черновик;
- `fetch_optional` допускает отсутствие строки;
- `map` при наличии строки вызывает `to_article`;
- `transpose` превращает `Option<Result<Article>>` в `Result<Option<Article>>`.

Этот `transpose` удобен: вызывающий код получает одну обычную ошибку базы.

### Строки 71–78: `by_rubric`

```rust
pub async fn by_rubric(
    pool: &SqlitePool,
    rubric_slug: &str,
) -> sqlx::Result<Vec<Article>> {
    let mut list = published(pool).await?;
    list.retain(|a| a.rubric_slug == rubric_slug);
    Ok(list)
}
```

- сначала берутся все опубликованные статьи;
- `retain` оставляет совпавшую рубрику;
- `mut` нужен, потому что список меняется;
- для школьного объёма такая фильтрация проще отдельной колонки в БД.

### Строки 80–91: начало `search`

```rust
pub async fn search(pool: &SqlitePool, query: &str) -> sqlx::Result<Vec<Article>> {
    let needle = query.to_lowercase();
    let rows = sqlx::query(
        "SELECT slug, title, rubric, author, lead, body_md, published_at
           FROM articles
          WHERE published = 1
          ORDER BY published_at DESC, id DESC",
    )
    .fetch_all(pool)
    .await?;
```

- запрос пользователя приводится к нижнему регистру;
- `to_lowercase` работает с кириллицей, в отличие от SQLite `NOCASE`;
- из базы берутся только опубликованные статьи;
- порядок остаётся «свежие сверху».

### Строки 93–109: фильтрация поиска

```rust
let mut found = Vec::new();
for row in rows {
    let title: String = row.try_get("title")?;
    let author: String = row.try_get("author")?;
    let lead: String = row.try_get("lead")?;
    let body: String = row.try_get("body_md")?;
    let matches = [title, author, lead, body]
        .iter()
        .any(|value| value.to_lowercase().contains(&needle));
    if matches {
        found.push(to_article(row)?);
    }
    if found.len() == SEARCH_LIMIT {
        break;
    }
}
Ok(found)
```

- `found` — накопительный список результатов;
- каждая колонка читается в свою `String`;
- массив объединяет четыре поля;
- `any` возвращает `true`, если совпало хотя бы одно;
- `contains` ищет подстроку;
- совпавшая строка превращается в `Article`;
- после 20 результатов цикл прекращается;
- функция возвращает найденный список.

### Строки 112–129: `to_article`

```rust
fn to_article(row: sqlx::sqlite::SqliteRow) -> sqlx::Result<Article> {
    let body: String = row.try_get("body_md")?;
    let published_at: Option<String> = row.try_get("published_at")?;
    let rubric: String = row.try_get("rubric")?;

    Ok(Article {
        slug: row.try_get("slug")?,
        title: row.try_get("title")?,
        rubric_slug: slug::translit(&rubric),
        rubric,
        author: row.try_get("author")?,
        lead: row.try_get("lead")?,
        body_html: markdown::to_html(&body),
        date: ru_date(published_at.as_deref().unwrap_or("")),
    })
}
```

- SQL-строка забирается по значению;
- `try_get` проверяет имя и тип колонки;
- `published_at` может быть `None`;
- `rubric` нужен дважды: сохранить и превратить в slug;
- `translit` не забирает `rubric`, поэтому порядок полей безопасен;
- `body_md` проходит Markdown и ammonia;
- `as_deref()` превращает `Option<String>` в `Option<&str>`;
- `unwrap_or("")` даёт пустую дату для черновика/битой записи.

### Строки 131–144: месяцы

`MONTHS` — массив из 12 названий. Индекс `0` соответствует январю.
Массив нужен, потому что SQLite хранит `2026-11-12`, а сайт показывает
`12 ноября`.

### Строки 146–162: `ru_date`

```rust
fn ru_date(iso: &str) -> String {
    let mut parts = iso.split('-');
    let _year = parts.next();
    let month = parts.next().and_then(|m| m.parse::<usize>().ok());
    let day = parts.next().and_then(|d| d.parse::<u32>().ok());

    match (month, day) {
        (Some(m), Some(d)) => MONTHS
            .get(m.wrapping_sub(1))
            .map(|name| format!("{d} {name}"))
            .unwrap_or_default(),
        _ => String::new(),
    }
}
```

- `split('-')` разбивает ISO-дату;
- `_year` сознательно не используется, `_` убирает предупреждение;
- месяц и день пытаются стать числами;
- `and_then` продолжает цепочку только при успехе;
- `parse::<usize>().ok()` превращает `Result` в `Option`;
- `wrapping_sub(1)` переводит номер месяца в индекс;
- `.get()` защищает от месяца `0` или `13`;
- `format!` создаёт `12 ноября`;
- при неверной дате возвращается пустая строка.

---

## 13. `src/auth.rs` — вход и защита админских API

### Строки 1–5

```rust
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
```

- `HashMap` хранит счётчики попыток по username.
- `Arc` позволяет нескольким запросам иметь один общий объект.
- `Mutex` разрешает менять объект только одному потоку одновременно.
- `Duration` задаёт длительность блокировки.
- `Instant` измеряет прошедшее время без календарной даты.

### Строки 7–12

```rust
#[cfg(test)]
use argon2::password_hash::PasswordHasher;
use argon2::{
    Argon2,
    password_hash::{PasswordVerifier, phc::PasswordHash},
};
```

- `PasswordHasher` нужен только тестам и отдельной утилите.
- `#[cfg(test)]` исключает импорт из production-сборки.
- `Argon2` выполняет хеширование и проверку.
- `PasswordVerifier` даёт метод `verify_password`.
- `PasswordHash` разбирает строку `$argon2id$...`.

### Строки 13–25

Импортируются:

- `Json` для входа и ответа;
- `Request` и `State` для middleware;
- `Next`, чтобы передать запрос следующему обработчику;
- `IntoResponse` и `Response`;
- cookie-типы;
- serde для JSON;
- SHA-256 для хеша токена;
- `SqlitePool`;
- `App` и `AppError`.

### Строки 27–30: константы

```rust
const SESSION_COOKIE: &str = "vivec_session";
const SESSION_HOURS: i64 = 12;
const LOGIN_LIMIT: u8 = 5;
const LOGIN_WINDOW: Duration = Duration::from_secs(15 * 60);
```

- имя cookie одинаково при записи и чтении;
- сессия живёт 12 часов;
- разрешено пять неудачных попыток;
- после лимита username блокируется на 15 минут.

### Строки 32–38: `Auth`

```rust
#[derive(Clone)]
pub struct Auth {
    username: Option<String>,
    password_hash: Option<String>,
    secure_cookie: bool,
    attempts: Arc<Mutex<HashMap<String, LoginAttempt>>>,
}
```

- `Clone` нужен, потому что `Auth` входит в `App`.
- `Option` позволяет запустить публичный сайт без настроенной админки.
- `password_hash` хранит только Argon2-строку, не пароль.
- `secure_cookie` включает HTTPS-only cookie.
- `Arc` — общий указатель на данные.
- `Mutex` — блокировка доступа.
- `HashMap` сопоставляет username с попытками.

### Строки 40–44: попытка входа

```rust
#[derive(Clone, Copy)]
struct LoginAttempt {
    failures: u8,
    blocked_until: Option<Instant>,
}
```

- `Copy` разрешает копировать маленькую структуру.
- `failures` считает ошибки.
- `blocked_until = None` означает «не заблокирован».
- `Some(Instant)` означает конкретный момент разблокировки.

### Строки 46–50: `from_env`

```rust
pub fn from_env() -> Self {
    let username = std::env::var("ADMIN_USER").ok();
    let password_hash = std::env::var("ADMIN_PASSWORD_HASH").ok();
```

- функция создаёт настройки auth;
- `.ok()` превращает отсутствие переменной в `None`;
- настоящие значения приходят из `deploy/vivec.env`.

### Строки 51–53

Если хотя бы одной переменной нет, пишется предупреждение. Сайт продолжает
показывать статьи, но админка отвечает `503`.

### Строки 55–57

```rust
let secure_cookie = std::env::var("SESSION_COOKIE_SECURE")
    .map(|value| value == "true" || value == "1")
    .unwrap_or(false);
```

- `map` превращает строку env в `bool`;
- `true` или `1` включают Secure;
- отсутствие переменной означает `false` для локального HTTP.

### Строки 59–64

```rust
Self {
    username,
    password_hash,
    secure_cookie,
    attempts: Arc::new(Mutex::new(HashMap::new())),
}
```

- поля получают прочитанные значения;
- `HashMap::new()` создаёт пустую таблицу;
- `Mutex::new` оборачивает таблицу блокировкой;
- `Arc::new` делает владельца общим.

### Строки 67–74: `credentials`

```rust
fn credentials(&self) -> Result<(&str, &str), AppError> {
    match (&self.username, &self.password_hash) {
        (Some(username), Some(password_hash)) => {
            Ok((username.as_str(), password_hash.as_str()))
        }
        _ => Err(AppError::Unavailable("админка не настроена")),
    }
}
```

- метод только читает `self`;
- `match` одновременно проверяет оба `Option`;
- успех отдаёт две заимствованные строки;
- `as_str` превращает `String` в `&str`;
- любой неполный набор env даёт `503`.

### Строки 76–83: начало проверки лимита

```rust
fn check_rate_limit(&self, username: &str) -> Result<(), AppError> {
    let mut attempts = self.attempts.lock().map_err(|_| {
        AppError::Internal("блокировка лимита входа повреждена".to_string())
    })?;
    let Some(attempt) = attempts.get_mut(username) else {
        return Ok(());
    };
```

- `lock()` получает эксклюзивный доступ;
- `map_err` превращает отравленный Mutex в `500`;
- `get_mut` ищет изменяемую запись;
- `let Some` продолжает только при наличии записи;
- новый username пока не заблокирован.

### Строки 85–94

```rust
if let Some(until) = attempt.blocked_until {
    if Instant::now() < until {
        return Err(AppError::TooManyRequests);
    }
    *attempt = LoginAttempt {
        failures: 0,
        blocked_until: None,
    };
}
Ok(())
```

- если срок ещё не прошёл, возвращается `429`;
- если прошёл, запись сбрасывается;
- `*attempt = ...` заменяет содержимое по ссылке.

Блокировка отпускается, когда функция завершается. Ждать Argon2 с зажатым
Mutex нельзя.

### Строки 97–106: запись неудачи

```rust
fn register_failure(&self, username: &str) {
    let Ok(mut attempts) = self.attempts.lock() else {
        return;
    };
    let attempt = attempts
        .entry(username.to_string())
        .or_insert(LoginAttempt {
            failures: 0,
            blocked_until: None,
        });
```

- если Mutex сломан, вход просто не обновляет счётчик;
- `entry` ищет username;
- `or_insert` создаёт запись для нового имени;
- `to_string` делает собственный ключ карты.

### Строки 107–111

```rust
attempt.failures = attempt.failures.saturating_add(1);
if attempt.failures >= LOGIN_LIMIT {
    attempt.failures = 0;
    attempt.blocked_until = Some(Instant::now() + LOGIN_WINDOW);
}
```

- `saturating_add` не переполняет `u8`;
- на пятом провале ставятся 15 минут блокировки;
- счётчик сбрасывается, потому что блокировка уже активна.

### Строки 114–118

После успешного входа запись по username удаляется. Человек снова получает
пять попыток.

### Строки 121–135: JSON-структуры

```rust
#[derive(Deserialize)]
pub struct LoginInput {
    username: String,
    password: String,
}

#[derive(Serialize)]
pub struct UserResponse {
    username: String,
}

#[derive(Serialize)]
pub struct OkResponse {
    ok: bool,
}
```

- `LoginInput` разбирает JSON из браузера;
- `UserResponse` сообщает текущего пользователя;
- `OkResponse` подтверждает выход.

### Строки 137–141: параметры login

```rust
pub async fn login(
    State(app): State<App>,
    jar: CookieJar,
    Json(input): Json<LoginInput>,
) -> Result<impl IntoResponse, AppError> {
```

- `State` даёт пул и auth;
- `CookieJar` читает/добавляет cookie;
- `Json` разбирает тело;
- `impl IntoResponse` значит «вернётся один из подходящих HTTP-ответов».

### Строки 142–144

```rust
validate_login_input(&input)?;
app.auth.check_rate_limit(&input.username)?;
let (username, password_hash) = app.auth.credentials()?;
```

Сначала дешёвые проверки размеров, затем rate limit, затем env. Argon2
запускается только после них.

### Строки 146–150

```rust
let parsed_hash = PasswordHash::new(password_hash)
    .map_err(|_| AppError::Internal("ADMIN_PASSWORD_HASH повреждён".to_string()))?;
let password_ok = Argon2::default()
    .verify_password(input.password.as_bytes(), &parsed_hash)
    .is_ok();
```

- строка хеша разбирается;
- битая env-строка даёт `500`;
- Argon2 проверяет пароль;
- `.is_ok()` превращает результат в `bool`.

### Строки 152–155

Если username не совпал или пароль неверен, регистрируется провал и
возвращается `401`. Наружу не сообщается, что именно неверно.

### Строки 157–159

```rust
app.auth.register_success(&input.username);
let token = random_token()?;
let token_hash = hash_token(&token);
```

- успешная попытка очищает счётчик;
- токен — 32 случайных байта в hex;
- в базу пойдёт только SHA-256.

### Строки 161–171: сессия

```rust
sqlx::query("DELETE FROM sessions WHERE expires_at <= datetime('now')")
    .execute(&app.pool)
    .await?;
sqlx::query(
    "INSERT INTO sessions (token_hash, expires_at)
     VALUES (?, datetime('now', ?))",
)
.bind(token_hash)
.bind(format!("+{SESSION_HOURS} hours"))
.execute(&app.pool)
.await?;
```

- сначала удаляются истёкшие сессии;
- затем добавляется новая;
- SQLite вычисляет `expires_at`;
- `+12 hours` безопасно передаётся как параметр.

### Строки 173–179: cookie

```rust
let cookie = Cookie::build((SESSION_COOKIE, token))
    .path("/")
    .http_only(true)
    .same_site(SameSite::Lax)
    .secure(app.auth.secure_cookie)
    .max_age(cookie::time::Duration::hours(SESSION_HOURS))
    .build();
```

- имя и значение;
- `Path=/` делает cookie доступной API;
- `HttpOnly` запрещает чтение из JavaScript;
- `SameSite=Lax` ограничивает межсайтовые запросы;
- `Secure` в production требует HTTPS;
- `Max-Age` — 12 часов;
- `build` создаёт готовый cookie.

### Строки 180–184

JSON сообщает username. `jar.add(cookie)` добавляет `Set-Cookie`;
получившийся кортеж axum превращает в ответ.

### Строки 186–192: logout

```rust
if let Some(token) = jar.get(SESSION_COOKIE) {
    sqlx::query("DELETE FROM sessions WHERE token_hash = ?")
        .bind(hash_token(token.value()))
        .execute(&app.pool)
        .await?;
}
```

Если cookie есть, её токен хешируется и удаляется из базы. Выход отзывает
сессию на сервере, а не только стирает cookie в браузере.

### Строки 194–202

Новая cookie с пустым значением и `Max-Age=0` удаляет cookie из браузера.
JSON подтверждает успех.

### Строки 204–209: `me`

Middleware уже проверил сессию. `me` только возвращает username из env.
Подчёркивание в `let (username, _)` означает «хеш здесь не нужен».

### Строки 211–226: `require_session`

```rust
pub async fn require_session(
    State(app): State<App>,
    jar: CookieJar,
    request: Request,
    next: Next,
) -> Response {
    let Some(token) = jar.get(SESSION_COOKIE) else {
        return AppError::Unauthorized.into_response();
    };

    match session_exists(&app.pool, token.value()).await {
        Ok(true) => next.run(request).await,
        Ok(false) => AppError::Unauthorized.into_response(),
        Err(error) => AppError::Db(error).into_response(),
    }
}
```

- нет cookie → `401`;
- сессия есть → запрос идёт дальше;
- сессии нет → `401`;
- ошибка БД → `500`.

`Next` содержит следующий хендлер. Важно: `request` передаётся только
после проверки.

### Строки 228–243: `require_csrf`

```rust
if matches!(request.method().as_str(), "GET" | "HEAD" | "OPTIONS") {
    return next.run(request).await;
}
```

Чтение не меняет данные и не требует CSRF-заголовка.

```rust
let valid = request
    .headers()
    .get("x-vivec-csrf")
    .and_then(|value| value.to_str().ok())
    == Some("1");
```

- берётся заголовок;
- заголовок переводится в строку;
- значение должно быть `1`.

При неверном заголовке возвращается `403`. Обычная HTML-форма без CORS
не может произвольно добавить такой заголовок.

### Строки 245–254: проверка БД

```rust
let row = sqlx::query(
    "SELECT 1 FROM sessions
      WHERE token_hash = ? AND expires_at > datetime('now')",
)
.bind(hash_token(token))
.fetch_optional(pool)
.await?;
Ok(row.is_some())
```

- `SELECT 1` экономит передачу всех колонок;
- сравнение идёт по хешу;
- срок проверяет SQLite;
- `is_some` превращает результат в `bool`.

### Строки 256–264: валидация входа

Username не пустой и не длиннее 100 байт. Password не пустой и не длиннее
1024 байт. Это защита от absurd-размеров, а не проверка правильности.

### Строки 266–271: случайный токен

```rust
let mut bytes = [0_u8; 32];
getrandom::fill(&mut bytes)
    .map_err(|_| AppError::Internal("не удалось создать сессию".to_string()))?;
Ok(to_hex(&bytes))
```

- создаётся массив 32 байт;
- ОС заполняет его криптографической случайностью;
- ошибка ОС становится `500`;
- байты превращаются в 64 hex-символа.

### Строки 273–275: хеш токена

`Sha256::digest` считает 32-байтовый хеш, `to_hex` снова делает hex.

### Строки 277–285: hex

```rust
const HEX: &[u8; 16] = b"0123456789abcdef";
let mut out = String::with_capacity(bytes.len() * 2);
for byte in bytes {
    out.push(HEX[(byte >> 4) as usize] as char);
    out.push(HEX[(byte & 0x0f) as usize] as char);
}
```

- таблица HEX содержит 16 символов;
- каждый байт кодируется двумя символами;
- старший nibble получается `byte >> 4`;
- младший — `byte & 0x0f`;
- результат имеет длину `bytes.len() * 2`.

### Строки 287–320: тесты

`hash_password` компилируется только для тестов, потому что отдельный
бинарник уже содержит свою реализацию.

Первый тест:

1. хеширует пароль;
2. разбирает PHC-строку;
3. проверяет правильный пароль.

Второй тест:

- одинаковый токен даёт одинаковый хеш;
- другой токен даёт другой хеш;
- `000f10ff` проверяет hex-преобразование.

---

## 14. `src/bin/hash-password.rs` — отдельная программа

### Строки 1–3

```rust
use std::io::{self, Read};
use argon2::{Argon2, password_hash::PasswordHasher};
```

Это не HTTP-сервер, а отдельный маленький бинарник для генерации хеша.

### Строки 5–10

```rust
fn main() {
    let mut password = String::new();
    if io::stdin().read_to_string(&mut password).is_err() {
        eprintln!("не удалось прочитать пароль из stdin");
        std::process::exit(1);
    }
```

- создаётся пустая строка;
- stdin читается в неё;
- ошибка печатается в stderr;
- код выхода `1` сообщает оболочке о провале.

Пароль читается из stdin, чтобы он не попал в список процессов и историю
аргументов.

### Строки 12–16

```rust
let password = password.trim_end_matches(['\r', '\n']);
if password.is_empty() {
    eprintln!("пароль пустой");
    std::process::exit(1);
}
```

Убираются только переводы строк. Пробелы внутри пароля сохраняются.

### Строки 18–24

```rust
match Argon2::default().hash_password(password.as_bytes()) {
    Ok(hash) => println!("{hash}"),
    Err(error) => {
        eprintln!("не удалось создать хеш: {error}");
        std::process::exit(1);
    }
}
```

- `Argon2::default()` использует Argon2id;
- `hash_password` генерирует соль и PHC-строку;
- хеш идёт в stdout;
- ошибка идёт в stderr.

---

## 15. `src/api.rs` — JSON API админки

### Строки 1–6

```rust
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
```

- `Json<T>` разбирает входной JSON и сериализует выходной.
- `Path` читает `{id}` из URL.
- `Query` читает параметры после `?`.
- `State` даёт `App`.
- `StatusCode` позволяет вернуть `201` или `204`.
- `IntoResponse` нужен `create`, который возвращает разные формы ответа.

### Строки 7–10

- serde превращает структуры в JSON и обратно.
- `Row` читает SQL-колонки.
- `SqlitePool` нужен внутренним функциям.
- `RUBRICS` содержит допустимые рубрики.
- `AppError` — единая ошибка.
- `slug` нормализует URL.

### Строки 12–16: ограничения

```rust
const MAX_TITLE: usize = 200;
const MAX_SLUG: usize = 120;
const MAX_AUTHOR: usize = 100;
const MAX_LEAD: usize = 500;
const MAX_BODY: usize = 200_000;
```

- заголовок до 200 Unicode-символов;
- slug до 120;
- автор до 100;
- лид до 500;
- Markdown до 200 000 байт.

### Строки 18–28: `ArticleSummary`

Это краткая статья для таблицы.

- `id` — номер для URL API;
- `slug`, `title`, `rubric`, `author` — данные строки;
- `published` превращается в JSON `true/false`;
- `published_at` может быть `null`;
- `created_at` нужен, если даты публикации ещё нет;
- тело статьи здесь отсутствует специально.

### Строки 30–42: `AdminArticle`

Полная статья для редактора. Дополнительно содержит `lead` и `body_md`.
Опять нет Markdown-рендера: редактор должен получить исходный текст.

### Строки 44–58: входные типы

```rust
#[derive(Deserialize)]
pub struct ArticleInput {
    title: String,
    slug: String,
    rubric: String,
    author: String,
    lead: String,
    body_md: String,
}

#[derive(Deserialize)]
pub struct ListQuery {
    status: Option<String>,
    q: Option<String>,
}
```

- `ArticleInput` — JSON тела create/update;
- `ListQuery` разбирает `?status=draft&q=слово`;
- оба query-параметра необязательны.

### Строки 60–63

```rust
#[derive(Serialize)]
pub struct ArticleList {
    items: Vec<ArticleSummary>,
}
```

JSON всегда имеет форму:

```json
{"items":[...]}
```

Обёртка позволяет позже добавить `total` без смены корневого формы.

### Строки 65–72: начало `list`

```rust
pub async fn list(
    State(app): State<App>,
    Query(query): Query<ListQuery>,
) -> Result<Json<ArticleList>, AppError> {
    let status = query.status.as_deref().unwrap_or("all");
    if !matches!(status, "all" | "draft" | "published") {
        return Err(AppError::BadRequest("неизвестный статус статьи"));
    }
```

- axum передаёт состояние и query;
- `as_deref` даёт `&str` из `Option<String>`;
- отсутствие статуса означает `all`;
- `matches!` разрешает только три значения;
- неизвестный статус даёт `400`.

### Строки 74–85: SQL и преобразование

```rust
let rows = sqlx::query(
    "SELECT id, slug, title, rubric, author, published, published_at, created_at
       FROM articles
      ORDER BY created_at DESC, id DESC",
)
.fetch_all(&app.pool)
.await?;

let mut items = rows
    .into_iter()
    .map(to_summary)
    .collect::<sqlx::Result<Vec<_>>>()?;
```

Админка видит и черновики, поэтому `WHERE published` здесь нет.
`to_summary` преобразует каждую строку. `collect::<sqlx::Result<...>>`
останавливается на первой ошибке чтения.

### Строки 87–91: фильтр статуса

```rust
if status == "draft" {
    items.retain(|article| !article.published);
} else if status == "published" {
    items.retain(|article| article.published);
}
```

- `draft` оставляет только `published=false`;
- `published` оставляет только `true`;
- `all` ничего не фильтрует.

### Строки 93–100: поиск в админке

```rust
if let Some(needle) = query.q.as_deref().map(str::trim).filter(|q| !q.is_empty()) {
    let needle = needle.to_lowercase();
    items.retain(|article| {
        article.title.to_lowercase().contains(&needle)
            || article.slug.to_lowercase().contains(&needle)
            || article.author.to_lowercase().contains(&needle)
    });
}
```

- query обрезается по пробелам;
- пустая строка игнорируется;
- регистр Unicode складывается;
- сравниваются title, slug и author;
- `retain` оставляет совпавшие элементы.

### Строки 102–110: `get`

```rust
pub async fn get(
    State(app): State<App>,
    Path(id): Path<i64>,
) -> Result<Json<AdminArticle>, AppError> {
    Ok(Json(get_by_id(&app.pool, id).await?))
}
```

- `{id}` автоматически разбирается в `i64`;
- `get_by_id` делает SQL;
- отсутствие статьи будет `404`;
- результат сериализуется в JSON.

### Строки 112–116: начало `create`

```rust
pub async fn create(
    State(app): State<App>,
    Json(input): Json<ArticleInput>,
) -> Result<impl IntoResponse, AppError> {
    let input = validate(input)?;
```

`Json` уже разобрал тело. `validate` забирает input, нормализует строки
и возвращает исправленный input либо `400`.

### Строки 117–128: INSERT

```rust
let result = sqlx::query(
    "INSERT INTO articles (slug, title, rubric, author, lead, body_md)
     VALUES (?, ?, ?, ?, ?, ?)",
)
.bind(&input.slug)
.bind(&input.title)
.bind(&input.rubric)
.bind(&input.author)
.bind(&input.lead)
.bind(&input.body_md)
.execute(&app.pool)
.await;
```

- поля перечислены в порядке `VALUES`;
- каждый `?` получает значение через `bind`;
- значения не склеиваются в SQL;
- `published` получает default `0`, то есть создаётся черновик.

### Строки 130–136: результат INSERT

```rust
let id = match result {
    Ok(result) => result.last_insert_rowid(),
    Err(error) if is_unique_violation(&error) => {
        return Err(AppError::Conflict("такой slug уже существует"));
    }
    Err(error) => return Err(error.into()),
};
```

- успех даёт автосозданный id;
- уникальный конфликт превращается в `409`;
- остальные SQL-ошибки передаются в `AppError::Db`.

### Строки 138–140

Статья читается обратно из базы. Это гарантирует, что ответ содержит
также `published`, `published_at` и `created_at`. HTTP-код `201` означает
«создано».

### Строки 142–149: начало `update`

```rust
let input = validate(input)?;
get_by_id(&app.pool, id).await?;
```

Сначала проверяется вход, затем существование записи. Если id нет,
`get_by_id` вернёт `404` до UPDATE.

### Строки 150–163: UPDATE

```rust
UPDATE articles
   SET slug = ?, title = ?, rubric = ?, author = ?, lead = ?, body_md = ?
 WHERE id = ?
```

Обновляются все редактируемые поля. `published` и `published_at` не входят
в форму: их меняют отдельные publish/unpublish endpoints.

### Строки 165–171

Успех ничего не возвращает. Уникальный slug превращается в `409`,
остальные ошибки — в `AppError`.

### Строки 173–174

После UPDATE статья снова читается и возвращается клиенту. Так Vue сразу
получает серверную версию данных.

### Строки 176–192: publish

```rust
UPDATE articles
   SET published = 1, published_at = date('now')
 WHERE id = ?
```

- статья становится публичной;
- дата публикации обновляется;
- `rows_affected() == 0` означает «id не найден» → `404`;
- успешный полный объект возвращается JSON.

### Строки 194–206: unpublish

Меняется только `published = 0`. Текст и дата публикации сохраняются.
Отсутствующий id даёт `404`.

### Строки 208–217: delete

- DELETE физически удаляет строку;
- нулевое число удалённых строк → `404`;
- успех → `204 No Content`.

`204` не содержит JSON, поэтому `api.js` обрабатывает его отдельно.

### Строки 219–243: `get_by_id`

```rust
let row = sqlx::query(/* SELECT ... WHERE id = ? */)
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;
```

- `fetch_optional` допускает отсутствие;
- `ok_or` превращает `None` в `404`;
- затем каждое поле читается через `try_get`;
- SQLite хранит `published` числом, поэтому `== 1` превращает его в `bool`.

### Строки 245–256: `to_summary`

Функция похожа на `get_by_id`, но:

- принимает уже полученную строку;
- не читает `lead` и `body_md`;
- возвращает краткую структуру.

### Строки 258–263: очистка строк

```rust
input.title = input.title.trim().to_string();
input.author = input.author.trim().to_string();
input.lead = input.lead.trim().to_string();
input.rubric = input.rubric.trim().to_string();
```

`trim` убирает пробелы по краям. `to_string` создаёт новую нормализованную
строку. `body_md` не обрезается, потому что пустые строки в Markdown
являются частью форматирования.

### Строки 264–278: проверки

```rust
if input.title.is_empty() || input.title.chars().count() > MAX_TITLE {
    return Err(AppError::BadRequest("заголовок: от 1 до 200 символов"));
}
```

Заголовок проверяется по `chars().count()`, а не `len()`: русская буква
в UTF-8 занимает два байта. Автор и лид проверяются так же.

Тело проверяется через `len()`, потому что лимит задан в байтах и защищает
память HTTP-запроса.

```rust
if !RUBRICS.iter().any(|(name, _)| *name == input.rubric) {
    return Err(AppError::BadRequest("неизвестная рубрика"));
}
```

`any` разрешает только четыре известные рубрики.

### Строки 280–287: slug

```rust
input.slug = if input.slug.trim().is_empty() {
    slug::translit(&input.title)
} else {
    slug::translit(input.slug.trim())
};
```

- пустой slug генерируется из заголовка;
- введённый вручную тоже нормализуется;
- пустой результат отклоняется;
- длина проверяется после нормализации.

Изменение заголовка не меняет существующий slug автоматически: URL должен
оставаться стабильным.

### Строки 292–297: уникальность

```rust
error
    .as_database_error()
    .map(|error| error.is_unique_violation())
    .unwrap_or(false)
```

- проверяется, что ошибка пришла от базы;
- у неё спрашивают тип unique violation;
- не-SQL ошибка считается не-уникальной.

### Строки 299–327: тесты

`article()` создаёт эталонный `ArticleInput`.

Первый тест проверяет:

- пробелы вокруг заголовка удалены;
- пустой slug стал `novyy-material`.

Второй тест:

- меняет рубрику на `Сплетни`;
- ожидает ошибку валидации.

---

## 16. `src/uploads.rs` — загрузка изображений

### Строки 1–9

```rust
use std::path::PathBuf;
use axum::{Json, extract::{Multipart, State}};
use serde::Serialize;
use crate::{App, db, error::AppError};
```

- `PathBuf` — изменяемый путь к файлу.
- `Json` возвращает URL загруженного файла.
- `Multipart` разбирает `multipart/form-data`.
- `State` даёт приложение.
- `Serialize` превращает ответ в JSON.
- `db::data_dir()` даёт каталог с базой.

### Строка 11

```rust
pub const MAX_UPLOAD: usize = 8 * 1024 * 1024;
```

8 × 1024 × 1024 байт = 8 MiB. Константа используется и проверкой, и
маршрутом aixum.

### Строки 13–16

```rust
#[derive(Serialize)]
pub struct UploadResponse {
    url: String,
}
```

Vue получит:

```json
{"url":"/uploads/abc.png"}
```

### Строки 18–22: `init`

```rust
pub async fn init() -> std::io::Result<PathBuf> {
    let dir = PathBuf::from(db::data_dir()).join("uploads");
    tokio::fs::create_dir_all(&dir).await?;
    Ok(dir)
}
```

- `data` превращается в `PathBuf`;
- `join("uploads")` получает `data/uploads`;
- `create_dir_all` создаёт каталог;
- возвращается путь для `ServeDir`.

### Строки 24–27: сигнатура upload

```rust
pub async fn upload(
    State(_app): State<App>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, AppError> {
```

- `_app` не используется, но сохраняет единый тип хендлера с состоянием.
- `mut` нужен, потому что чтение полей меняет `multipart`.
- результат — JSON URL или `AppError`.

### Строки 28–35: перебор полей

```rust
while let Some(field) = multipart.next_field().await.map_err(...)? {
    if field.name() != Some("file") {
        continue;
    }
```

- `next_field` читает поля по одному;
- `await` ждёт очередную часть тела;
- ошибка разбора становится `400`;
- поле с другим именем пропускается;
- Vue отправляет именно поле `file`.

### Строки 37–43: байты и лимит

```rust
let bytes = field.bytes().await...;
if bytes.is_empty() || bytes.len() > MAX_UPLOAD {
    return Err(AppError::BadRequest("файл: от 1 байта до 8 МБ"));
}
```

`bytes` — буфер файла в памяти. Пустой и слишком большой файл отклоняются
до записи на диск.

### Строки 45–52: тип файла

```rust
let kind = infer::get(&bytes).ok_or(...)?;
let extension = match kind.mime_type() {
    "image/jpeg" => "jpg",
    "image/png" => "png",
    "image/webp" => "webp",
    "image/gif" => "gif",
    _ => return Err(...),
};
```

- `infer` смотрит сигнатуру байтов;
- имя файла от клиента не используется;
- разрешены только четыре изображения;
- SVG/HTML/EXE и другие типы отклоняются;
- расширение выбирает сервер.

### Строки 54–61: запись

```rust
let name = format!("{}.{}", random_name()?, extension);
let path = init().await...?.join(&name);
tokio::fs::write(&path, bytes).await...?;
```

- случайное имя исключает перезапись и path traversal;
- путь получается через контролируемый каталог;
- `write` асинхронно сохраняет байты.

### Строки 63–69

Успех возвращает `/uploads/<name>`. Если поля `file` вообще не было,
возвращается `400`.

### Строки 71–83: случайное имя

```rust
let mut bytes = [0_u8; 16];
getrandom::fill(&mut bytes)...?;
```

16 случайных байт дают 32 hex-символа. Дальше идёт тот же способ hex:
старший nibble и младший nibble на каждый байт. Имя не содержит пользовательских
данных.

---

## 17. `src/main.rs` — сборка всего приложения

### Строки 1–8: import axum

```rust
use axum::{
    Router,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{HeaderValue, StatusCode, header},
    middleware,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
```

- `Router` хранит таблицу маршрутов.
- `DefaultBodyLimit` ограничивает тело JSON/upload.
- `Path`, `Query`, `State` — extractors.
- `HeaderValue` и `header` настраивают HTTP-заголовки.
- `middleware` подключает auth/CSRF слои.
- `Html` возвращает HTML.
- `IntoResponse` и `Response` нужны `not_found`.
- `get` и `post` связывают функции с методами HTTP.

### Строки 9–13

```rust
use sqlx::sqlite::SqlitePool;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
```

- `SqlitePool` хранится в `App`.
- `ServeDir` раздаёт каталоги.
- `ServeFile` возвращает `index.html` для Vue router.
- `SetResponseHeaderLayer` добавляет security headers.
- `TraceLayer` логирует HTTP.
- tracing imports настраивают вывод логов.

### Строки 15–23: модули

```rust
mod api;
mod articles;
mod auth;
mod db;
mod error;
mod markdown;
mod slug;
mod uploads;
use error::AppError;
```

Каждая строка `mod` подключает файл. `AppError` импортируется коротким
именем для хендлеров.

### Строки 25–30: шаблоны

```rust
const LAYOUT: &str = include_str!("../templates/layout.html.jinja");
// ...
```

Каждый `include_str!` встраивает один HTML-шаблон в бинарник. После правки
HTML нужен новый build. Взамен файл не потеряется в контейнере.

### Строки 32–39: `App`

```rust
#[derive(Clone)]
struct App {
    tpl: minijinja::Environment<'static>,
    pool: SqlitePool,
    auth: auth::Auth,
    today: String,
}
```

- `tpl` хранит загруженные шаблоны.
- `'static` значит, что шаблоны созданы из встроенных строк и живут всю программу.
- `pool` — общий пул базы.
- `auth` — настройки входа и лимитер.
- `today` — дата, вычисленная при старте.
- `Clone` позволяет отдавать копию состояния каждому запросу.

### Строки 41–46: вход

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();
```

- tokio создаёт async runtime;
- `main` возвращает ошибки запуска;
- `EnvFilter` читает `RUST_LOG`;
- без env используется уровень `info`;
- `fmt::layer` печатает логи;
- `init` активирует subscriber.

### Строки 48–50: ресурсы

```rust
let pool = db::open().await?;
let today = today_ru(&pool).await?;
let uploads_dir = uploads::init().await?;
```

- открывается SQLite и применяются схемы;
- из SQLite берётся русская дата;
- создаётся `DATA_DIR/uploads`.

Любая ошибка `?` завершает запуск до открытия HTTP-порта.

### Строки 52–58: шаблоны

```rust
let mut tpl = minijinja::Environment::new();
tpl.add_template("layout.html.jinja", LAYOUT)?;
// ...
```

- создаётся окружение шаблонов;
- `mut` разрешает добавлять шаблоны;
- имя при добавлении должно совпадать с `extends`/`get_template`;
- все `?` ловят синтаксическую ошибку шаблона на старте.

### Строки 60–65: состояние

```rust
let app = App {
    tpl,
    pool,
    auth: auth::Auth::from_env(),
    today,
};
```

Ресурсы перемещаются в `App`. `Auth::from_env` читает переменные окружения.

### Строки 67–76: защищённые маршруты

```rust
let protected_admin = Router::new()
    .route("/me", get(auth::me))
    .route("/logout", post(auth::logout))
    .route("/articles", get(api::list).post(api::create))
    .route(
        "/articles/{id}",
        get(api::get).put(api::update).delete(api::delete),
    )
    .route("/articles/{id}/publish", post(api::publish))
    .route("/articles/{id}/unpublish", post(api::unpublish))
```

- `/me` и `/logout` относятся к auth;
- `/articles` отвечает GET и POST;
- один путь `{id}` имеет GET/PUT/DELETE;
- publish/unpublish — отдельные POST-операции.

Маршруты здесь относительные: позже роутер вложат в `/api/admin`.

### Строки 77–84: upload и session middleware

```rust
.route(
    "/uploads",
    post(uploads::upload).layer(DefaultBodyLimit::max(
        uploads::MAX_UPLOAD + 64 * 1024
    )),
)
.route_layer(middleware::from_fn_with_state(
    app.clone(),
    auth::require_session,
));
```

- upload получает лимит 8 MiB плюс запас на multipart;
- `route_layer` применяется только к protected_admin;
- `from_fn_with_state` передаёт `App` в middleware;
- без валидной сессии запрос не дойдёт до хендлера.

### Строки 85–88: публичный login и CSRF

```rust
let admin_api = Router::new()
    .route("/login", post(auth::login))
    .merge(protected_admin)
    .route_layer(middleware::from_fn(auth::require_csrf));
```

- login доступен без сессии;
- protected routes объединяются с login;
- CSRF-layer применяется ко всему `/api/admin`;
- GET/HEAD/OPTIONS проходят без заголовка;
- мутации требуют `X-Vivec-CSRF: 1`.

### Строки 90–91: Vue SPA

```rust
let admin_files =
    ServeDir::new("static/admin").fallback(ServeFile::new("static/admin/index.html"));
```

- реальный asset отдаётся из `static/admin`;
- если файла нет, отдаётся `index.html`;
- Vue router сам разбирает `/admin/articles/1/edit`;
- `.fallback` сохраняет статус `200`; `not_found_service` вернул бы `404`.

### Строки 93–99: публичные маршруты

```rust
let router = Router::new()
    .route("/", get(index))
    .route("/healthz", get(healthz))
    .route("/rubric/{rubric}", get(rubric))
    .route("/search", get(search))
    .route("/{slug}", get(article))
```

- `/` — главная;
- `/healthz` — проверка живости;
- `{rubric}` — параметр;
- `/search` — поиск;
- `{slug}` — статья;
- динамический `{slug}` идёт последним, чтобы не перехватывать статические пути.

### Строки 100–104: вложенные сервисы

```rust
.nest("/api/admin", admin_api)
.nest_service("/admin", admin_files)
.nest_service("/static", ServeDir::new("static"))
.nest_service("/uploads", ServeDir::new(uploads_dir))
.fallback(not_found)
```

- API получает префикс `/api/admin`;
- Vue — `/admin`;
- CSS — `/static`;
- изображения — `/uploads`;
- всё неизвестное показывает HTML 404.

### Строки 105–113: лимит и headers

```rust
.layer(DefaultBodyLimit::max(256 * 1024))
.layer(SetResponseHeaderLayer::overriding(
    header::X_CONTENT_TYPE_OPTIONS,
    HeaderValue::from_static("nosniff"),
))
.layer(SetResponseHeaderLayer::overriding(
    header::REFERRER_POLICY,
    HeaderValue::from_static("strict-origin-when-cross-origin"),
))
```

- обычный JSON ограничен 256 KiB;
- upload имеет свой больший лимит;
- `nosniff` не разрешает угадывать MIME вопреки серверу;
- Referrer Policy не отправляет полный URL на чужие сайты.

### Строки 114–115

`TraceLayer` логирует HTTP, `with_state(app)` вставляет состояние во все
extractors `State<App>`.

### Строки 117–121: порт

```rust
let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
let addr = format!("0.0.0.0:{port}");
let listener = tokio::net::TcpListener::bind(&addr).await?;
```

- env `PORT` переопределяет порт;
- по умолчанию 8080;
- `0.0.0.0` принимает соединения из контейнера/сети;
- `TcpListener` занимает порт.

### Строки 122–126

```rust
tracing::info!("listening on http://{addr}");
axum::serve(listener, router)
    .with_graceful_shutdown(shutdown_signal())
    .await?;
Ok(())
```

- лог сообщает адрес;
- axum принимает запросы;
- shutdown ждёт сигнал;
- `Ok(())` достигается после корректной остановки.

### Строки 129–137: Ctrl+C

```rust
let ctrl_c = async {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for ctrl-c");
};
```

`ctrl_c` — асинхронное будущее, готовое после сигнала. `expect` здесь
допустим: если ОС не даёт слушать Ctrl+C, сервер не может корректно работать.

### Строки 139–148: SIGTERM

```rust
#[cfg(unix)]
let terminate = async {
    tokio::signal::unix::signal(
        tokio::signal::unix::SignalKind::terminate()
    ).expect(...).recv().await;
};

#[cfg(not(unix))]
let terminate = std::future::pending::<()>();
```

- Linux использует SIGTERM;
- `recv` ждёт сигнал;
- на не-Unix платформе future никогда не завершится.

### Строки 150–155

```rust
tokio::select! {
    _ = ctrl_c => {},
    _ = terminate => {},
}
```

`select!` ждёт первый из двух сигналов. Затем пишется лог. axum заканчивает
текущие запросы и только после этого останавливается.

### Строки 158–161: `healthz`

Хендлер без параметров возвращает `"ok"`. Мониторинг и podman healthcheck
понимают, что процесс жив.

### Строки 163–170: `IndexCtx`

```rust
#[derive(serde::Serialize)]
struct IndexCtx {
    today: String,
    hero: Option<articles::Article>,
    cards: Vec<articles::Article>,
}
```

- `today` выводится в шапке;
- `hero` — первая статья или `None`;
- `cards` — остальные до трёх.

### Строки 172–189: `index`

```rust
let mut list = articles::published(&app.pool).await?;
let mut cards = list.split_off(list.len().min(1));
let hero = if list.is_empty() {
    None
} else {
    Some(list.remove(0))
};
cards.truncate(3);
```

- получить опубликованные статьи;
- `split_off` отделяет всё после первой;
- из первой части берётся hero;
- `remove(0)` забирает владение первой статьёй;
- карточки ограничены тремя.

```rust
let ctx = minijinja::Value::from(minijinja::value::Serde(IndexCtx {...}));
let html = app.tpl.get_template(...).render(ctx)?;
Ok(Html(html))
```

- структура оборачивается serde-адаптером;
- шаблон получает контекст;
- `render` создаёт HTML;
- `Html` выставляет content type.

### Строки 192–197: `ArticleCtx`

Содержит ссылки на дату и статью. Ссылки не копируют полный текст.

### Строки 199–212: `article`

```rust
match articles::by_slug(&app.pool, &slug).await? {
    Some(article) => {
        // render
    }
    None => Ok(not_found(State(app)).await),
}
```

- slug приходит из URL;
- база ищет только опубликованную статью;
- найденная рендерится;
- отсутствующая даёт общий HTML 404;
- черновик неотличим от несуществующей статьи для внешнего пользователя.

### Строки 214–221: `RubricCtx`

Содержит дату, русское имя, slug и статьи рубрики.

### Строки 223–232

```rust
let Some(name) = articles::rubric_name(&rubric_slug) else {
    return Ok(not_found(State(app)).await);
};
```

Это let-else: при `None` выполняется return. Неизвестная рубрика даёт 404,
а не пустую страницу `200`.

### Строки 233–242

Рубричные статьи получаются из БД, контекст сериализуется, шаблон
рендерится в HTML.

### Строки 244–254: поиск

```rust
#[derive(serde::Deserialize)]
struct SearchQuery {
    q: Option<String>,
}

#[derive(serde::Serialize)]
struct SearchCtx<'a> {
    today: &'a str,
    query: &'a str,
    articles: Vec<articles::Article>,
}
```

- `q` приходит из query string;
- `SearchCtx` содержит безопасно отформатированный запрос и результаты;
- `'a` говорит, что ссылки живут не дольше входной строки.

### Строки 256–274: `search`

```rust
let query = params.q.unwrap_or_default();
let query = query.trim();
let found = if query.is_empty() {
    Vec::new()
} else {
    articles::search(&app.pool, query).await?
};
```

- отсутствующий `q` становится пустой строкой;
- пробелы обрезаются;
- пустой запрос не делает SQL-поиск;
- иначе вызывается `articles::search`;
- найденное передаётся в шаблон.

### Строки 276–296: `not_found`

Локальная структура содержит дату шаблона.

```rust
app.tpl
    .get_template("404.html.jinja")
    .and_then(|t| t.render(ctx))
```

- `and_then` продолжает только при успешном поиске шаблона;
- `render` выполняет второй `Result`;
- успех возвращает `404` и HTML;
- ошибка логируется и даёт `500`.

### Строки 298–312: начало `today_ru`

```rust
let row = sqlx::query(
    "SELECT date('now') AS iso, strftime('%w', 'now') AS weekday"
)
.fetch_one(pool)
.await?;
```

- SQLite возвращает ISO-дату;
- `%w` возвращает номер дня недели 0–6;
- `fetch_one` ожидает ровно одну строку.

```rust
let iso: String = sqlx::Row::try_get(&row, "iso")?;
let weekday: String = sqlx::Row::try_get(&row, "weekday")?;
```

Обе колонки читаются как строки. Полный путь `sqlx::Row` используется,
потому что трейт не импортирован в `main.rs`.

### Строки 304–312: день недели

`match` превращает `0–6` в русское слово. `_` — суббота, потому что все
числа 0–5 уже обработаны.

### Строки 313–317: разбор даты

```rust
let mut parts = iso.split('-');
let year = parts.next().unwrap_or("");
let month = parts.next().and_then(|value| value.parse::<usize>().ok());
let day = parts.next().and_then(|value| value.parse::<u32>().ok());
```

- `split` даёт три части;
- year остаётся строкой;
- month и day превращаются в числа;
- ошибка parse превращается в `None`.

### Строки 318–338: русская дата

```rust
(Some(month), Some(day)) => {
    const MONTHS: [&str; 12] = [...];
    let name = MONTHS.get(month.wrapping_sub(1)).copied().unwrap_or("");
    format!("{weekday}, {day} {name} {year}")
}
_ => String::new(),
```

- при валидных month/day выбирается русское имя;
- `wrapping_sub` получает индекс;
- `get` защищает от выхода за массив;
- `format!` создаёт `среда, 18 ноября 2026`;
- `to_uppercase` делает `СРЕДА, 18 НОЯБРЯ 2026`.

---

## 18. SSR-шаблоны — часть backend

Публичные страницы собирает не Vue, а Rust через minijinja. Поэтому шаблоны
относятся к backend-потоку ответа.

### `templates/layout.html.jinja`

Строка 1 `<!doctype html>` сообщает браузеру, что это современный HTML5.

Строка 2 задаёт русский язык документа. Это важно screen reader и браузеру.

Строки 3–8:

- `head` содержит служебные данные;
- `charset=utf-8` включает кириллицу;
- `viewport` задаёт масштаб на телефоне;
- `title` — имя вкладки, дочерний шаблон может его заменить;
- `stylesheet` подключает `/static/style.css`.

Строки 9–10 открывают `body` и общий контейнер `.page`.

Строки 12–15 — верхняя строка:

- `today` пришла из Rust;
- номер выпуска пока константа `№1`.

Строки 17–21 — блок `masthead`:

- дочерний шаблон может заменить этот блок;
- обычная шапка содержит большой логотип и подпись газеты.

Строки 23–33 — навигация:

- ссылки ведут на главную и четыре рубрики;
- `GET`-форма отправляет `q` на `/search`;
- `name="q"` совпадает с `SearchQuery`;
- `value` сохраняет введённый запрос;
- фильтр `default('')` подставляет пустую строку на страницах без query.

Строка 34 закрывает блок masthead.

Строка 36 — место, куда дочерние шаблоны вставляют `content`.

Строки 38–43 — подвал и закрытие контейнеров.

Minijinja сам экранирует значения. `<script>` из базы не станет тегом,
если не применить `safe`.

### `templates/index.html.jinja`

- строка 1 наследует общий layout;
- строка 3 задаёт title;
- блок `content` начинается на строке 5;
- `if hero` проверяет наличие статьи;
- ссылка рубрики использует `rubric_slug`;
- ссылка статьи использует `slug`;
- поля статьи автоматически экранируются;
- `else` показывает пустое состояние;
- `if cards` проверяет остальные материалы;
- `for a in cards` перебирает карточки;
- каждая карточка повторяет ссылку, title, lead, author и date;
- `endfor`, `endif`, `endblock` закрывают Jinja-блоки.

### `templates/article.html.jinja`

- title берётся из статьи;
- `masthead` заменён компактной шапкой;
- `article-rubric` ведёт в рубрику;
- заголовок выводится как текст;
- лид показывается только при непустом значении;
- мета содержит автора и дату;
- `<hr>` визуально отделяет начало текста;
- `article.body_html | safe` вставляет уже очищенный ammonia HTML;
- комментарий рядом объясняет, почему `safe` здесь допустим.

### `templates/rubric.html.jinja`

- title содержит имя рубрики;
- используется компактная шапка;
- `rubric-count` проверяет длину массива;
- `if articles` различает непустую рубрику;
- цикл выводит заголовок, лид, автора и дату;
- `else` объясняет пустое состояние.

### `templates/search.html.jinja`

- title зависит от наличия query;
- `query` в заголовке экранируется minijinja;
- пустой поиск показывает инструкцию;
- непустой поиск показывает число результатов;
- цикл выводит найденные статьи;
- `elif query` показывает подсказку, если результатов нет.

Именно экранирование `query` проверено XSS-тестом: `<script>` выводится
как текст, а не исполняется.

### `templates/404.html.jinja`

- наследует layout;
- меняет title;
- выводит понятную страницу вместо белой строки;
- даёт ссылку на главную.

HTTP-код `404` выставляет Rust в `not_found`, а не шаблон.

---

## 19. Полная таблица HTTP-маршрутов

### Публичные GET

| Путь | Handler | Ответ |
|---|---|---|
| `/` | `index` | главная с 4 статьями максимум |
| `/healthz` | `healthz` | текст `ok` |
| `/rubric/{rubric}` | `rubric` | статьи рубрики |
| `/search?q=...` | `search` | результаты |
| `/{slug}` | `article` | статья или 404 |
| `/admin/{path}` | `ServeDir` | Vue files/index |
| `/static/{file}` | `ServeDir` | CSS |
| `/uploads/{file}` | `ServeDir` | изображение |

### Auth API

| Метод | Путь | Cookie | CSRF | Ответ |
|---|---|---|---|---|
| POST | `/api/admin/login` | создаёт | требует | user JSON |
| GET | `/api/admin/me` | требует | нет | user JSON |
| POST | `/api/admin/logout` | требует | требует | ok JSON |

### Articles API

| Метод | Путь | Смысл |
|---|---|---|
| GET | `/api/admin/articles` | список и фильтры |
| POST | `/api/admin/articles` | создать черновик |
| GET | `/api/admin/articles/{id}` | полный текст |
| PUT | `/api/admin/articles/{id}` | сохранить |
| POST | `.../{id}/publish` | опубликовать |
| POST | `.../{id}/unpublish` | снять |
| DELETE | `.../{id}` | удалить |

### Upload

```text
POST /api/admin/uploads
Content-Type: multipart/form-data
field: file
```

---

## 20. Формы JSON

### Создание и обновление

```json
{
  "title": "Заголовок",
  "slug": "",
  "rubric": "Школа",
  "author": "Автор",
  "lead": "Короткий лид",
  "body_md": "Текст **жирным**"
}
```

`published`, `published_at`, `created_at` клиент не присылает. Ими управляет
сервер.

### Полная статья

```json
{
  "id": 1,
  "slug": "zagolovok",
  "title": "Заголовок",
  "rubric": "Школа",
  "author": "Автор",
  "lead": "Лид",
  "body_md": "# Markdown",
  "published": false,
  "published_at": null,
  "created_at": "2026-11-18 10:00:00"
}
```

### Ошибка

```json
{"error":"такой slug уже существует"}
```

Сервер не возвращает SQL, путь к базе или stack trace.

---

## 21. Почему запрос безопасен

### SQL injection

Все значения передаются через `.bind`. SQL-структура запроса остаётся
константой. Пользовательский текст не может добавить новый SQL-запрос.

### XSS в статье

Путь:

```text
body_md → pulldown-cmark → ammonia → safe в шаблоне
```

Опасный HTML удаляется до `safe`.

### XSS в поиске

Строка `query` выводится без `safe`. Minijinja экранирует `<`, `>` и
кавычки. Поэтому поиск `<script>` безопасен.

### Кража пароля

Настоящий пароль не сохраняется. Хранится медленный Argon2id-хеш со
случайной солью.

### Кража cookie

- токен случайный;
- в базе только SHA-256;
- `HttpOnly` запрещает чтение из JS;
- `Secure` требует HTTPS в production;
- срок 12 часов.

### Подбор пароля

Пять ошибок на username запускают 15-минутную блокировку. Лимитер хранится
в памяти процесса, поэтому после рестарта счётчики сбрасываются. Для одного
школьного редактора это принятый компромисс.

### CSRF

Мутация требует `X-Vivec-CSRF: 1`. Чужой простой HTML-форме браузер не даст
добавить такой JavaScript-заголовок.

### Опасная загрузка

- MIME определяется по содержимому;
- разрешены только JPEG/PNG/WebP/GIF;
- имя генерирует сервер;
- размер до 8 MiB;
- файл лежит в отдельном `uploads`;
- SVG, HTML и PHP не принимаются.

### Огромный JSON

Общий `DefaultBodyLimit` — 256 KiB. Upload-маршрут отдельно получает около
8 MiB и не повышает лимит остальных маршрутов.

---

## 22. Как проследить один запрос вручную

Возьмём `GET /remont-sportzala`.

1. tokio принимает TCP-соединение.
2. axum разбирает HTTP.
3. Router проверяет методы и пути.
4. `/{slug}` выбирает `article`.
5. `Path<String>` получает `remont-sportzala`.
6. `State<App>` даёт пул и шаблоны.
7. `articles::by_slug` выполняет `SELECT`.
8. `.bind` защищает slug.
9. `fetch_optional` возвращает `Some(row)`.
10. `to_article` читает колонки.
11. `markdown::to_html` создаёт безопасный HTML.
12. minijinja рендерит `article.html.jinja`.
13. `Html` ставит content type.
14. axum отправляет `200` и HTML.
15. `TraceLayer` пишет запрос в лог.

Возьмём `PUT /api/admin/articles/1`.

1. `/api/admin` вложен в `admin_api`.
2. CSRF-middleware проверяет заголовок.
3. session-middleware проверяет cookie.
4. `Path<i64>` получает `1`.
5. `Json<ArticleInput>` разбирает тело.
6. `validate` нормализует строки.
7. `get_by_id` проверяет существование.
8. UPDATE выполняется с bind-параметрами.
9. уникальный конфликт превращается в `409`.
10. `get_by_id` читает свежие данные.
11. axum отправляет JSON `200`.

---

## 23. Как запускать и проверять

### Только проверка типов

```sh
cargo check --all-targets
```

### Тесты

```sh
cargo test --all-targets
```

### Линтер

```sh
cargo clippy --all-targets -- -D warnings
```

### Хеш пароля

```sh
cargo run --bin hash-password
```

Ввести пароль в stdin и скопировать строку `$argon2id$...`.

### Запуск

```sh
PORT=8099 \
DATA_DIR=data \
ADMIN_USER=editor \
ADMIN_PASSWORD_HASH='$argon2id$...' \
cargo run --bin vivec
```

### Быстрый HTTP-ответ

```sh
curl -i http://127.0.0.1:8099/healthz
```

`-i` показывает статус и headers.

### Полный smoke

```sh
./scripts/smoke.sh
```

---

## 24. Что означают типовые ошибки

| Ошибка | Перевод |
|---|---|
| `cannot find type` | тип не импортирован |
| `mismatched types` | один тип передан вместо другого |
| `value moved here` | владение уже передано |
| `borrowed value does not live long enough` | ссылка переживает данные |
| `trait bound not satisfied` | тип не умеет нужную операцию |
| `no method named ...` | метод не существует или не импортирован trait |
| `unused variable` | переменная не используется |
| `unreachable pattern` | вариант `match` уже покрыт |
| `non-exhaustive patterns` | не все варианты enum обработаны |
| `use of moved value` | значение попытались использовать после передачи владения |

Хороший порядок исправления:

1. читать первую ошибку;
2. открыть указанную строку;
3. определить ожидаемый тип;
4. прочитать `help`;
5. исправить одну причину;
6. снова `cargo check`.

Нельзя лечить всё `.clone()`, `unwrap()` и `#[allow]`.

---

## 25. Упражнения для проверки понимания

### Упражнение 1: публичная статья

Объяснить без подсказки:

1. какой handler вызывается для `GET /test`;
2. где slug получает значение;
3. почему черновик даёт 404;
4. где Markdown очищается;
5. почему `safe` не создаёт XSS.

### Упражнение 2: поиск

1. Почему нельзя просто написать `LIKE '%ремонт%'`?
2. Что делает `to_lowercase()`?
3. Почему поиск не возвращает черновик?
4. Где ограничение 20 результатов?

### Упражнение 3: вход

1. Что приходит в JSON login?
2. Где проверяется пароль?
3. Что сохраняется в cookie?
4. Что сохраняется в БД?
5. Чем logout отличается от удаления cookie?

### Упражнение 4: API

1. Почему `create` возвращает 201?
2. Почему `delete` возвращает 204?
3. Почему duplicate slug — 409?
4. Почему `published_at` не приходит от клиента?

### Упражнение 5: upload

1. Почему нельзя доверять расширению `.png`?
2. Что делает `infer`?
3. Почему имя генерирует сервер?
4. Где находится лимит 8 MiB?

### Практические задания

1. Добавь тест для `ru_date("2026-01-31") == "31 января"`.
2. Добавь тест slug `"Привет, мир!"`.
3. Создай статью с Markdown-таблицей и проверь HTML.
4. Отправь duplicate slug и объясни ответ 409.
5. Загрузи CSS вместо PNG и объясни ответ 400.
6. Запусти сервер с пустым `DATA_DIR` и найди созданные таблицы.

---

## 26. Известные ограничения текущего backend

1. `today` вычисляется при старте. Если сервер работает через полночь,
   дата в шапке обновится только после рестарта.
2. Rate limit хранится в памяти и сбрасывается при рестарте.
3. Один env-пользователь; несколько редакторов не поддержаны.
4. Поиск перебирает опубликованные строки в Rust. Для школьного объёма
   это нормально; FTS5 запланирован после сдачи.
5. Upload держит файл до 8 MiB в памяти перед записью.
6. Статическая Vue-оболочка `/admin` доступна без входа, но данные и
   мутации защищены API-сессией.
7. Старые сессии не привязаны к username. После смены `ADMIN_USER`
   существующие сессии живут до истечения или logout.
8. Удаление статьи не удаляет осиротевшие изображения.
Это не «скрытые баги для красоты», а осознанные ограничения v1. Каждое
изменение поведения сначала идёт в `docs/backlog.md`, затем в отдельную
ветку.

---

## 27. Мини-словарь backend

| Термин | Смысл |
|---|---|
| Handler | функция обработки запроса |
| Extractor | тип, который axum достаёт из запроса |
| Middleware | проверка до/после handler |
| State | общие данные приложения |
| Pool | набор соединений БД |
| Bind | безопасная подстановка SQL-параметра |
| Row | одна SQL-строка |
| Response | HTTP-ответ |
| JSON | формат API |
| Cookie | данные браузера для сессии |
| Session | серверная запись входа |
| CSRF | подделка запроса от имени пользователя |
| XSS | внедрение опасного HTML/JS |
| MIME | тип содержимого файла |
| SSR | HTML собран на сервере |
| SPA | приложение работает в браузере |
| Graceful shutdown | завершить текущие запросы до остановки |

---

## 28. Главная формула, которую надо запомнить

Почти каждый backend-хендлер Vivec:

```text
взять параметры
→ проверить параметры
→ обратиться к БД/файлу
→ превратить результат в безопасный HTML/JSON
→ вернуть HTTP-код
```

Почти каждая ошибка:

```text
создать AppError
→ залогировать внутреннюю причину
→ вернуть клиенту безопасный статус
```

Почти каждая SQL-операция:

```text
SQL с ? 
→ bind значения
→ await
→ проверить Some/None или rows_affected
```

Если эти три схемы понятны, весь текущий backend Vivec читается как
набор повторений, а не как магия.

---

## 29. Containerfile: как backend попадает на сервер

Этот файл не исполняется внутри Rust-программы. Он объясняет, как собрать
бинарник и упаковать его для podman.

### Строки 1–7: сборка Vue

```dockerfile
FROM node:24-slim AS admin-build
WORKDIR /admin
COPY admin/package.json admin/package-lock.json ./
RUN npm ci
COPY admin/ ./
RUN npm run build
```

- Node используется только в builder-стадии;
- `npm ci` ставит точные версии из lock-файла;
- `npm run build` собирает Vue;
- Vite пишет результат в `/static/admin`;
- Node не попадает в финальный runtime image.

### Строки 9–10: Rust builder

```dockerfile
FROM rust:1-slim-bookworm AS build
WORKDIR /src
```

- `FROM` выбирает образ с готовым Rust;
- `AS build` даёт стадии имя `build`;
- `WORKDIR` создаёт рабочий каталог внутри стадии.

### Строки 12–18

```dockerfile
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src \
 && echo 'fn main() {}' > src/main.rs \
 && cargo build --release \
 && rm -rf src
```

Сначала копируются только manifest и lock. Затем создаётся пустой `main`,
чтобы собрать и закэшировать зависимости отдельно. Временный `src` удаляется.

`rm -rf` здесь выполняется внутри одноразовой build-стадии и удаляет только
созданный тут каталог `/src/src`.

### Строки 20–22

Копируются реальные исходники, шаблоны и миграции. Они нужны `include_str!`
на этапе компиляции.

### Строки 24–28

```dockerfile
RUN find src templates migrations -type f -exec touch {} + \
 && cargo build --release
```

`COPY` сохраняет mtime файлов с хост-машины. Без `touch` cargo иногда
считал старый пустой бинарник актуальным. Это уже реально случалось
в проекте; комментарий фиксирует причину.

### Строки 30–33

```dockerfile
FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl \
 && rm -rf /var/lib/apt/lists/*
```

- начинается чистый runtime-образ;
- `ca-certificates` нужны для HTTPS;
- `curl` используется healthcheck;
- списки apt удаляются после установки;
- Node.js в runtime отсутствует.

### Строки 35–38

```dockerfile
WORKDIR /app
COPY --from=build /src/target/release/vivec /app/vivec
COPY static ./static
COPY --from=admin-build /static/admin ./static/admin
```

- Rust-бинарник копируется из Rust stage;
- обычная статика копируется из build context;
- собранная Vue-админка копируется из Node stage;
- `.dockerignore` может исключать `static/admin`, потому что итоговая папка
  создаётся внутри image, а не берётся с локального диска.

### Строки 40–44

```dockerfile
RUN useradd --uid 10001 --user-group --no-create-home \
      --shell /usr/sbin/nologin vivec \
 && mkdir -p /data \
 && chown vivec:vivec /data
USER 10001
```

- создаётся непривилегированный пользователь;
- `/data` принадлежит ему;
- процесс запускается не от root.

На хосте rootless podman дополнительно нужен
`--userns=keep-id:uid=10001,gid=10001`, иначе UID контейнера не совпадёт
с владельцем bind mount.

### Строки 46–50

```dockerfile
ENV DATA_DIR=/data \
    RUST_LOG=info
EXPOSE 8080
CMD ["/app/vivec"]
```

- база и uploads живут в `/data`;
- стандартный уровень логов `info`;
- порт документируется как 8080;
- контейнер запускает Rust-бинарник.

`EXPOSE` не публикует порт наружу. На хосте podman публикует только loopback,
а Caddy проксирует HTTPS.
