// @generated automatically by Diesel CLI.

diesel::table! {
    snippets (uuid) {
        uuid -> Text,
        title -> Text,
        content -> Text,
        tags -> Text,
        embedding -> Nullable<Binary>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    snippets_fts (rowid) {
        rowid -> Integer,
        title -> Nullable<Binary>,
        content -> Nullable<Binary>,
        tags -> Nullable<Binary>,
        #[sql_name = "snippets_fts"]
        snippet_content -> Nullable<Binary>,
        rank -> Nullable<Binary>,
    }
}

diesel::table! {
    snippets_fts_config (k) {
        k -> Binary,
        v -> Nullable<Binary>,
    }
}

diesel::table! {
    snippets_fts_content (id) {
        id -> Nullable<Integer>,
        c0 -> Nullable<Binary>,
        c1 -> Nullable<Binary>,
        c2 -> Nullable<Binary>,
    }
}

diesel::table! {
    snippets_fts_data (id) {
        id -> Nullable<Integer>,
        block -> Nullable<Binary>,
    }
}

diesel::table! {
    snippets_fts_docsize (id) {
        id -> Nullable<Integer>,
        sz -> Nullable<Binary>,
    }
}

diesel::table! {
    snippets_fts_idx (segid, term) {
        segid -> Binary,
        term -> Binary,
        pgno -> Nullable<Binary>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    snippets,
    snippets_fts,
    snippets_fts_config,
    snippets_fts_content,
    snippets_fts_data,
    snippets_fts_docsize,
    snippets_fts_idx,
);
