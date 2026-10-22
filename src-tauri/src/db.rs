use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Batas baris yang dikirim ke popup (500 riwayat + item yang disematkan).
const LIST_LIMIT: i64 = 600;
/// Item tidak disematkan yang disimpan; sisanya dipangkas saat insert.
const KEEP_LIMIT: i64 = 500;
/// Jumlah karakter yang dibaca untuk deteksi jenis dan pratinjau.
const SAMPLE_CHARS: i64 = 2000;
const PREVIEW_CHARS: usize = 400;

pub struct Db(pub Mutex<Connection>);

#[derive(Serialize)]
pub struct Clip {
    pub id: i64,
    pub preview: String,
    pub length: i64,
    pub lines: i64,
    pub kind: &'static str,
    pub pinned: bool,
    pub last_used_at: i64,
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn escape_like(q: &str) -> String {
    q.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

fn is_color(t: &str) -> bool {
    if let Some(hex) = t.strip_prefix('#') {
        return matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit());
    }
    let l = t.to_ascii_lowercase();
    t.len() < 64
        && ["rgb(", "rgba(", "hsl(", "hsla("].iter().any(|p| l.starts_with(p))
        && l.ends_with(')')
}

fn is_email(t: &str) -> bool {
    let mut parts = t.split('@');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(user), Some(domain), None) => {
            !user.is_empty()
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
        }
        _ => false,
    }
}

fn is_link(t: &str) -> bool {
    !t.is_empty()
        && !t.contains(char::is_whitespace)
        && (["http://", "https://", "www.", "mailto:"].iter().any(|p| t.starts_with(p))
            || is_email(t))
}

fn looks_like_code(t: &str) -> bool {
    const MARKERS: [&str; 18] = [
        "{", "};", "=>", "->", "</", "/>", "();", "fn ", "def ", "function ", "const ",
        "import ", "#include", "SELECT ", "public ", "class ", "return ", "!=",
    ];
    let hits = MARKERS.iter().filter(|m| t.contains(*m)).count();
    let multiline = t.contains('\n');
    let indented = t.lines().any(|l| l.starts_with("    ") || l.starts_with('\t'));
    hits >= 2 || (multiline && (hits >= 1 || indented))
}

fn detect_kind(sample: &str) -> &'static str {
    let t = sample.trim();
    if is_color(t) {
        "color"
    } else if is_link(t) {
        "link"
    } else if looks_like_code(t) {
        "code"
    } else {
        "text"
    }
}

impl Db {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS clips (
               id           INTEGER PRIMARY KEY AUTOINCREMENT,
               content      TEXT NOT NULL UNIQUE,
               created_at   INTEGER NOT NULL,
               last_used_at INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_clips_last_used ON clips(last_used_at DESC);",
        )?;
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version < 1 {
            conn.execute_batch(
                "ALTER TABLE clips ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0;
                 PRAGMA user_version = 1;",
            )?;
        }
        Ok(Db(Mutex::new(conn)))
    }

    /// Simpan teks; jika sudah ada cukup naikkan ke paling atas (dedupe).
    pub fn upsert(&self, content: &str) -> rusqlite::Result<()> {
        let conn = self.0.lock().unwrap();
        let now = now_ms();
        conn.execute(
            "INSERT INTO clips (content, created_at, last_used_at) VALUES (?1, ?2, ?2)
             ON CONFLICT(content) DO UPDATE SET last_used_at = excluded.last_used_at",
            params![content, now],
        )?;
        conn.execute(
            "DELETE FROM clips WHERE pinned = 0 AND id NOT IN
               (SELECT id FROM clips WHERE pinned = 0 ORDER BY last_used_at DESC LIMIT ?1)",
            params![KEEP_LIMIT],
        )?;
        Ok(())
    }

    /// Daftar riwayat (disematkan dulu), opsional difilter dengan pencarian teks.
    pub fn list(&self, query: &str) -> rusqlite::Result<Vec<Clip>> {
        let conn = self.0.lock().unwrap();
        let query = query.trim();
        let pattern = format!("%{}%", escape_like(query));
        let mut stmt = conn.prepare(
            r"SELECT id, substr(content, 1, ?1), length(content),
                     length(content) - length(replace(content, char(10), '')) + 1,
                     pinned, last_used_at
              FROM clips
              WHERE ?2 = '' OR content LIKE ?3 ESCAPE '\'
              ORDER BY pinned DESC, last_used_at DESC
              LIMIT ?4",
        )?;
        let rows = stmt.query_map(params![SAMPLE_CHARS, query, pattern, LIST_LIMIT], |r| {
            let sample: String = r.get(1)?;
            Ok(Clip {
                id: r.get(0)?,
                kind: detect_kind(&sample),
                preview: sample.chars().take(PREVIEW_CHARS).collect(),
                length: r.get(2)?,
                lines: r.get(3)?,
                pinned: r.get::<_, i64>(4)? != 0,
                last_used_at: r.get(5)?,
            })
        })?;
        rows.collect()
    }

    /// Ambil isi item sekaligus menandainya sebagai terbaru.
    pub fn touch(&self, id: i64) -> rusqlite::Result<Option<String>> {
        let conn = self.0.lock().unwrap();
        let content: Option<String> = conn
            .query_row("SELECT content FROM clips WHERE id = ?1", params![id], |r| r.get(0))
            .optional()?;
        if content.is_some() {
            conn.execute(
                "UPDATE clips SET last_used_at = ?1 WHERE id = ?2",
                params![now_ms(), id],
            )?;
        }
        Ok(content)
    }

    pub fn set_pinned(&self, id: i64, pinned: bool) -> rusqlite::Result<()> {
        self.0.lock().unwrap().execute(
            "UPDATE clips SET pinned = ?1 WHERE id = ?2",
            params![pinned as i64, id],
        )?;
        Ok(())
    }

    pub fn delete(&self, id: i64) -> rusqlite::Result<()> {
        self.0
            .lock()
            .unwrap()
            .execute("DELETE FROM clips WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Hapus riwayat; item yang disematkan ikut terhapus hanya jika `include_pinned`.
    pub fn clear(&self, include_pinned: bool) -> rusqlite::Result<()> {
        let sql = if include_pinned {
            "DELETE FROM clips"
        } else {
            "DELETE FROM clips WHERE pinned = 0"
        };
        self.0.lock().unwrap().execute(sql, [])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::detect_kind;

    #[test]
    fn detects_kinds() {
        assert_eq!(detect_kind("#2f6feb"), "color");
        assert_eq!(detect_kind("rgb(1, 2, 3)"), "color");
        assert_eq!(detect_kind("https://claude.ai"), "link");
        assert_eq!(detect_kind("nama@contoh.com"), "link");
        assert_eq!(detect_kind("fn main() {\n    println!(\"hi\");\n}"), "code");
        assert_eq!(detect_kind("const x = () => 1;"), "code");
        assert_eq!(detect_kind("Halo, apa kabar hari ini?"), "text");
        assert_eq!(detect_kind("#hashtag"), "text");
    }
}
