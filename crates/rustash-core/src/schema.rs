// @generated automatically by Diesel CLI.

// Main snippets table
diesel::table! {
    snippets (id) {
        id -> Nullable<Integer>,
        title -> Text,
        content -> Text,
        tags -> Text,
        embedding -> Nullable<Binary>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

// FTS5 virtual table for full-text search
diesel::table! {
    snippets_fts (rowid) {
        rowid -> Integer,
        title -> Text,
        content -> Text,
        tags -> Text,
    }
}

// FTS5 auxiliary tables (required by SQLite FTS5)
diesel::table! {
    snippets_fts_config (k) {
        k -> Text,
        v -> Nullable<Text>,
    }
}

diesel::table! {
    snippets_fts_content (id) {
        id -> Integer,
        c0 -> Nullable<Text>,
        c1 -> Nullable<Text>,
        c2 -> Nullable<Text>,
    }
}

diesel::table! {
    snippets_fts_data (id) {
        id -> Integer,
        block -> Nullable<Binary>,
    }
}

diesel::table! {
    snippets_fts_docsize (id) {
        id -> Integer,
        sz -> Nullable<Binary>,
    }
}

diesel::table! {
    snippets_fts_idx (segid, term) {
        segid -> Integer,
        term -> Text,
        pgno -> Nullable<Integer>,
    }
}

// Allow all tables to be used in the same query
diesel::allow_tables_to_appear_in_same_query!(
    snippets,
    snippets_fts,
    snippets_fts_config,
    snippets_fts_content,
    snippets_fts_data,
    snippets_fts_docsize,
    snippets_fts_idx,
);
