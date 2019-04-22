CREATE TABLE messages (
	id TEXT PRIMARY KEY,
	badge_info TEXT,
	badges TEXT,
	bits INTEGER,
	colour TEXT,
	display_name TEXT,
	emotes TEXT,
	message_id TEXT,
	moderator BOOLEAN,
	room_id INTEGER,
	tmi_sent_ts DATETIME,
	user_id TEXT,
	channel TEXT,
	message TEXT,
	raw_message TEXT
)
