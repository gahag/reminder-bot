table! {
    reminders (id) {
        id -> Integer,
        due -> BigInt,
        recurrying -> Nullable<Integer>,
        chat -> BigInt,
        message -> Text,
    }
}

table! {
    trusted_chats (id) {
        id -> BigInt,
        username -> Nullable<Text>,
        title -> Nullable<Text>,
    }
}

allow_tables_to_appear_in_same_query!(
    reminders,
    trusted_chats,
);
