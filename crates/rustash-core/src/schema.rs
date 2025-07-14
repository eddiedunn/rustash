// @generated automatically by Diesel CLI.

diesel::table! {
    relations (from_uuid, to_uuid, relation_type) {
        from_uuid -> Text,
        to_uuid -> Text,
        relation_type -> Text,
        created_at -> Timestamp,
    }
}

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
    vss_snippets (rowid) {
        rowid -> Integer,
        embedding -> Nullable<Binary>,
    }
}

diesel::joinable!(relations -> snippets (from_uuid));
diesel::joinable!(vss_snippets -> snippets (rowid));

diesel::allow_tables_to_appear_in_same_query!(relations, snippets, vss_snippets,);
