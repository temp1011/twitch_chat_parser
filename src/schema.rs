table! {
    messages (id) {
        id -> Nullable<Text>,
        badge_info -> Nullable<Text>,
        badges -> Nullable<Text>,
        bits -> Nullable<Integer>,
        color -> Nullable<Text>,
        display_name -> Nullable<Text>,
        emotes -> Nullable<Text>,
        message_id -> Nullable<Text>,
        #[sql_name = "mod"]
        mod_ -> Nullable<Bool>,
        room_id -> Nullable<Integer>,
        tmi_sent_ts -> Nullable<Timestamp>,
        user_id -> Nullable<Text>,
        channel -> Nullable<Text>,
        message -> Nullable<Text>,
        raw_message -> Nullable<Text>,
    }
}
