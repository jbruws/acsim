CREATE TABLE IF NOT EXISTS messages (
	msgid BIGSERIAL PRIMARY KEY,
	board TEXT NOT NULL,
	time BIGINT NOT NULL,
	author TEXT NOT NULL,
	msg TEXT NOT NULL,
	image TEXT NOT NULL,
	latest_submsg BIGINT NOT NULL
);
CREATE TABLE IF NOT EXISTS submessages (
	parent_msg BIGINT NOT NULL,
	submsg_id BIGINT NOT NULL,
	board TEXT NOT NULL,
	time BIGINT NOT NULL,
	author TEXT NOT NULL,
	submsg TEXT NOT NULL,
	image TEXT NOT NULL,
	CONSTRAINT bind_msg
		FOREIGN KEY(parent_msg)
			REFERENCES messages(msgid)
			ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS flagged_messages (
	entry_id BIGSERIAL PRIMARY KEY,
	msg_type TEXT NOT NULL,
	msgid BIGINT NOT NULL UNIQUE,
	submsg_index BIGINT,
	UNIQUE(msgid,submsg_index),
	CONSTRAINT bind_msg
		FOREIGN KEY(msgid)
			REFERENCES messages(msgid)
			ON DELETE CASCADE
);
