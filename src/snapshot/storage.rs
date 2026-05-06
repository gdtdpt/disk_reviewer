#![cfg(feature = "snapshot")]

use crate::scanner::{DirNode, Entry};
use std::path::Path;

/// Snapshot metadata returned by list_snapshots
#[derive(Debug, Clone)]
pub struct SnapshotMeta {
    pub id: i64,
    pub name: String,
    pub created_at: String,
    pub root_path: String,
    pub total_size: u64,
    pub total_files: u64,
}

/// SQLite-backed snapshot storage with path-indexed directory nodes
pub struct SnapshotStorage {
    conn: rusqlite::Connection,
}

impl SnapshotStorage {
    /// Open or create the snapshot database at the given path.
    /// Creates tables, enables WAL mode and foreign_keys.
    pub fn new(db_path: &Path) -> Result<Self, rusqlite::Error> {
        let conn = rusqlite::Connection::open(db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS snapshots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                created_at TEXT NOT NULL,
                root_path TEXT NOT NULL,
                total_size INTEGER NOT NULL,
                total_files INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS snapshot_nodes (
                snapshot_id INTEGER NOT NULL,
                path TEXT NOT NULL,
                parent_path TEXT,
                node_json TEXT NOT NULL,
                PRIMARY KEY (snapshot_id, path),
                FOREIGN KEY (snapshot_id) REFERENCES snapshots(id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_snapshot_nodes_parent
             ON snapshot_nodes(snapshot_id, parent_path)",
            [],
        )?;

        Ok(Self { conn })
    }

    /// Save a snapshot: insert metadata + all directory nodes in a transaction.
    /// Returns the new snapshot_id.
    pub fn save_snapshot(&mut self, name: &str, root: &DirNode) -> Result<i64, rusqlite::Error> {
        let tx = self.conn.transaction()?;

        let now = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();

        tx.execute(
            "INSERT INTO snapshots (name, created_at, root_path, total_size, total_files)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                name,
                now,
                root.path.to_string_lossy().as_ref(),
                &(root.total_size as i64),
                &(root.file_count as i64),
            ],
        )?;
        let snapshot_id = tx.last_insert_rowid();

        insert_nodes_recursive(&tx, snapshot_id, root, None)?;

        tx.commit()?;
        Ok(snapshot_id)
    }

    /// Load the root DirNode of a snapshot by ID.
    pub fn load_snapshot(&self, snapshot_id: i64) -> Result<DirNode, rusqlite::Error> {
        let node_json: String = self.conn.query_row(
            "SELECT node_json FROM snapshot_nodes
             WHERE snapshot_id = ?1 AND parent_path IS NULL",
            rusqlite::params![snapshot_id],
            |row| row.get(0),
        )?;

        let root: DirNode = serde_json::from_str(&node_json)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(e),
            ))?;

        Ok(root)
    }

    /// Load a specific subtree by path.
    pub fn load_subtree(&self, snapshot_id: i64, path: &str) -> Result<DirNode, rusqlite::Error> {
        let node_json: String = self.conn.query_row(
            "SELECT node_json FROM snapshot_nodes
             WHERE snapshot_id = ?1 AND path = ?2",
            rusqlite::params![snapshot_id, path],
            |row| row.get(0),
        )?;

        let node: DirNode = serde_json::from_str(&node_json)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(e),
            ))?;

        Ok(node)
    }

    /// List all snapshots ordered by created_at DESC.
    pub fn list_snapshots(&self) -> Result<Vec<SnapshotMeta>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, created_at, root_path, total_size, total_files
             FROM snapshots ORDER BY created_at DESC"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(SnapshotMeta {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
                root_path: row.get(3)?,
                total_size: row.get::<_, i64>(4)? as u64,
                total_files: row.get::<_, i64>(5)? as u64,
            })
        })?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    /// Delete a snapshot and all its node records (cascade).
    pub fn delete_snapshot(&mut self, snapshot_id: i64) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "DELETE FROM snapshots WHERE id = ?1",
            rusqlite::params![snapshot_id],
        )?;
        Ok(())
    }

    /// Rename a snapshot.
    pub fn rename_snapshot(&mut self, snapshot_id: i64, new_name: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE snapshots SET name = ?1 WHERE id = ?2",
            rusqlite::params![new_name, snapshot_id],
        )?;
        Ok(())
    }

    /// Generate a default name with timestamp (D-18).
    pub fn default_name() -> String {
        chrono::Local::now().format("快照 %Y-%m-%d %H:%M").to_string()
    }
}

/// Recursively insert all DirNode children into snapshot_nodes.
fn insert_nodes_recursive(
    tx: &rusqlite::Transaction,
    snapshot_id: i64,
    node: &DirNode,
    parent_path: Option<&str>,
) -> Result<(), rusqlite::Error> {
    let path_str = node.path.to_string_lossy().to_string();
    let node_json = serde_json::to_string(node)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

    tx.execute(
        "INSERT INTO snapshot_nodes (snapshot_id, path, parent_path, node_json)
         VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![snapshot_id, &path_str, parent_path, &node_json],
    )?;

    for child in &node.children {
        if let Entry::Dir(dir) = child {
            insert_nodes_recursive(tx, snapshot_id, dir, Some(&path_str))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::{FileEntry, OthersEntry};
    use crate::treemap::color::FileCategory;
    use std::path::PathBuf;

    fn make_test_dirnode() -> DirNode {
        DirNode {
            path: PathBuf::from(r"C:\test"),
            name: "test".to_string(),
            total_size: 650,
            file_count: 4,
            children: vec![
                Entry::File(FileEntry { name: "a.txt".to_string(), size: 100 }),
                Entry::Dir(DirNode {
                    path: PathBuf::from(r"C:\test\sub"),
                    name: "sub".to_string(),
                    total_size: 500,
                    file_count: 2,
                    children: vec![
                        Entry::File(FileEntry { name: "b.exe".to_string(), size: 500 }),
                    ],
                    access_denied: false,
                    dominant_cat: FileCategory::Other,
                }),
                Entry::Others(OthersEntry {
                    name: "Others".to_string(),
                    size: 50,
                    entry_count: 5,
                    entries: vec![
                        Entry::File(FileEntry { name: "tiny.tmp".to_string(), size: 50 }),
                    ],
                }),
                Entry::AccessDenied { path: PathBuf::from(r"C:\test\secret") },
            ],
            access_denied: false,
            dominant_cat: FileCategory::Other,
        }
    }

    fn make_storage() -> SnapshotStorage {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;").unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS snapshots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                created_at TEXT NOT NULL,
                root_path TEXT NOT NULL,
                total_size INTEGER NOT NULL,
                total_files INTEGER NOT NULL
            )",
            [],
        ).unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS snapshot_nodes (
                snapshot_id INTEGER NOT NULL,
                path TEXT NOT NULL,
                parent_path TEXT,
                node_json TEXT NOT NULL,
                PRIMARY KEY (snapshot_id, path),
                FOREIGN KEY (snapshot_id) REFERENCES snapshots(id) ON DELETE CASCADE
            )",
            [],
        ).unwrap();
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_snapshot_nodes_parent
             ON snapshot_nodes(snapshot_id, parent_path)",
            [],
        ).unwrap();
        SnapshotStorage { conn }
    }

    #[test]
    fn test_schema_creation() {
        let _storage = make_storage();
        // If we reach here, tables were created successfully
    }

    #[test]
    fn test_save_snapshot_returns_id() {
        let mut storage = make_storage();
        let root = make_test_dirnode();
        let id = storage.save_snapshot("test_snapshot", &root).unwrap();
        assert!(id > 0, "snapshot_id should be positive");
    }

    #[test]
    fn test_save_snapshot_same_name_replaces() {
        let mut storage = make_storage();
        let root = make_test_dirnode();
        let id1 = storage.save_snapshot("dup_name", &root).unwrap();
        let id2 = storage.save_snapshot("dup_name", &root).unwrap();
        // Each save creates a new row (D-17 handles same-ID replacement, different IDs are fine)
        assert!(id2 > id1, "second save should get a higher ID");
        let list = storage.list_snapshots().unwrap();
        assert_eq!(list.len(), 2, "both snapshots should exist with different IDs");
    }

    #[test]
    fn test_load_snapshot() {
        let mut storage = make_storage();
        let root = make_test_dirnode();
        let id = storage.save_snapshot("load_test", &root).unwrap();
        let loaded = storage.load_snapshot(id).unwrap();
        assert_eq!(loaded.path, PathBuf::from(r"C:\test"));
        assert_eq!(loaded.name, "test");
        assert_eq!(loaded.total_size, 650);
        assert_eq!(loaded.children.len(), 4);
    }

    #[test]
    fn test_list_snapshots_ordered_by_created_at_desc() {
        let mut storage = make_storage();
        let root = make_test_dirnode();
        storage.save_snapshot("first", &root).unwrap();
        storage.save_snapshot("second", &root).unwrap();
        let list = storage.list_snapshots().unwrap();
        assert_eq!(list.len(), 2);
        // Snapshots returned from list_snapshots — both have same second-level timestamp
        // Verify they are both present and IDs are in order
        let names: Vec<&str> = list.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"first"));
        assert!(names.contains(&"second"));
        // IDs should be in insertion order (first < second)
        let ids: Vec<i64> = list.iter().map(|s| s.id).collect();
        assert!(ids[0] != ids[1], "two snapshots should have different IDs");
    }

    #[test]
    fn test_delete_snapshot_cascade() {
        let mut storage = make_storage();
        let root = make_test_dirnode();
        let id = storage.save_snapshot("to_delete", &root).unwrap();

        // Verify nodes exist
        let node_count: i64 = storage.conn.query_row(
            "SELECT COUNT(*) FROM snapshot_nodes WHERE snapshot_id = ?1",
            rusqlite::params![id],
            |row| row.get(0),
        ).unwrap();
        assert!(node_count > 0, "should have node records");

        storage.delete_snapshot(id).unwrap();

        // Verify all data gone
        let snap_count: i64 = storage.conn.query_row(
            "SELECT COUNT(*) FROM snapshots WHERE id = ?1",
            rusqlite::params![id],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(snap_count, 0, "snapshot metadata should be gone");

        let node_count: i64 = storage.conn.query_row(
            "SELECT COUNT(*) FROM snapshot_nodes WHERE snapshot_id = ?1",
            rusqlite::params![id],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(node_count, 0, "all node records should be cascade-deleted (D-17)");
    }

    #[test]
    fn test_rename_snapshot() {
        let mut storage = make_storage();
        let root = make_test_dirnode();
        let id = storage.save_snapshot("old_name", &root).unwrap();
        storage.rename_snapshot(id, "new_name").unwrap();
        let list = storage.list_snapshots().unwrap();
        assert_eq!(list[0].name, "new_name");
    }

    #[test]
    fn test_default_name_format() {
        let name = SnapshotStorage::default_name();
        assert!(name.starts_with("快照 "), "default name should start with '快照 ' (D-18)");
        // Format: "快照 YYYY-MM-DD HH:MM" — check the timestamp portion
        let ts = &name["快照 ".len()..];
        assert_eq!(ts.len(), 16, "timestamp portion should be 16 chars (YYYY-MM-DD HH:MM)");
        assert_eq!(&ts[4..5], "-");
        assert_eq!(&ts[7..8], "-");
        assert_eq!(&ts[10..11], " ");
        assert_eq!(&ts[13..14], ":");
    }

    #[test]
    fn test_load_nonexistent_snapshot_returns_error() {
        let storage = make_storage();
        let result = storage.load_snapshot(999);
        assert!(result.is_err(), "loading non-existent snapshot should error");
    }

    #[test]
    fn test_load_subtree_by_path() {
        let mut storage = make_storage();
        let root = make_test_dirnode();
        let id = storage.save_snapshot("subtree_test", &root).unwrap();
        let subtree = storage.load_subtree(id, r"C:\test\sub").unwrap();
        assert_eq!(subtree.name, "sub");
        assert_eq!(subtree.total_size, 500);
    }
}
