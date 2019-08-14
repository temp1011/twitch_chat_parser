table! {
    messages (id) {
        id -> Text,
        badge_info -> Nullable<Text>,
        badges -> Nullable<Text>,
        bits -> Nullable<Integer>,  //Could use 0 to represent no bits but I think the option is clearer
        color -> Nullable<Text>,    //TODO this is a rgb hex so make it bytes/int/whatever
        display_name -> Text,
        emotes -> Nullable<Text>,
        #[sql_name = "mod"]
        mod_ -> Nullable<Bool>,
        room_id -> Integer,
        tmi_sent_ts -> Timestamp,
        user_id -> Text,
        channel -> Text,
        message -> Text,
        raw_message -> Text,
    }
}
