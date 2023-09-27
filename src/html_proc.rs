use chrono::{DateTime, FixedOffset, Local, NaiveDateTime};
use regex::Regex;
use std::str;

#[derive(PartialEq)]
pub enum BoardMessageType {
    Message,       // messages on main page
    ParentMessage, // parent message on topic pages
    Submessage,    // submessages on topic pages
}

// TODO: i'll probably have to move all those functions to one struct since i have to use DBs
// ...or do i?

// get seconds elapsed since unix epoch
pub fn since_epoch() -> i64 {
    let res = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(n) => n.as_secs().try_into().unwrap(),
        Err(_) => 1,
    };
    res
}

// returns current date in time in 'YYYY-MM-DD hh:mm:ss' 24-hour format
pub fn get_time(since_epoch: i64) -> String {
    let offset = FixedOffset::east_opt(3 * 3600).unwrap(); // +3 (hours) offset
    let naive = NaiveDateTime::from_timestamp_opt(since_epoch, 0).unwrap(); // UNIX epoch to datetime
    let dt = DateTime::<Local>::from_naive_utc_and_offset(naive, offset).to_string();
    dt[..dt.len() - 7].to_string() // 7 was chosen experimentally
}

// fits form data into one of several html templates
pub async fn format_into_html(
    message_type: BoardMessageType,
    board: &str,
    id: &i64,
    time: &str,
    author: &str,
    msg: &str,
    image: &str,
) -> String {
    let msg_contents: String;
    // if message has an image...
    if image != "" {
        let mut formatted_img: String;
        // for messages in topics, we need do descend to parent dir
        if message_type == BoardMessageType::ParentMessage
            || message_type == BoardMessageType::Submessage
        {
            formatted_img = String::from("."); // additional dot as a prefix
            formatted_img.push_str(image);
        } else {
            formatted_img = String::from(image);
        }

        msg_contents = format!(
            include_str!("../templates/message_contents/contents_img.html"),
            formatted_img, formatted_img, msg
        );
    } else {
        msg_contents = format!(
            include_str!("../templates/message_contents/contents_noimg.html"),
            msg
        );
    }

    let f_result = match message_type {
        BoardMessageType::Message => format!(
            include_str!("../templates/message_blocks/message.html"),
            board = board,
            id = id,
            time = time,
            author = author,
            msg = msg_contents,
        ),
        BoardMessageType::ParentMessage => format!(
            include_str!("../templates/message_blocks/parent_message.html"),
            time = time,
            author = author,
            id = id,
            msg = msg_contents,
        ),
        BoardMessageType::Submessage => format!(
            include_str!("../templates/message_blocks/submessage.html"),
            id = id,
            time = time,
            author = author,
            msg = msg_contents,
        ),
    };
    f_result
}

// removes html tags from message.
pub async fn filter_string(inp_string: &String) -> String {
    let filter = Regex::new(r##"<.*?>"##).unwrap();
    String::from(filter.replace_all(inp_string.as_str(), ""))
}

// turns message raw text from the database into workable html,
// which is later piped into format_into_html()
pub async fn prepare_msg(inp_string: &String) -> String {
    // in the format "{letters}>{digits}.{digits}"
    let msg_link_match = Regex::new(r##"\w{1,16}>\d+(\.\d+)?"##).unwrap();

    let mut result = String::new();
    let mut start_of_next: usize; // start of next match
    let mut end_of_last: usize = 0; // end of previous match

    // inserting links to other messages
    let msg_matches_iter = msg_link_match.find_iter(&inp_string);
    for m_raw in msg_matches_iter {
        let m = m_raw.as_str().to_string();

        start_of_next = m_raw.start();
        let finished_link: String;
        result.push_str(&inp_string[end_of_last..start_of_next]); // text between matches
        let board = m.split(">").collect::<Vec<&str>>()[0];

        // if it's a link to a submessage
        if m.contains(".") {
            let link_parts = m.split(".").collect::<Vec<&str>>();
            finished_link = format!(
                include_str!("../templates/message_contents/msglink.html"),
                board,
                &link_parts[0][2..],
                &link_parts[1],
                &m
            );
        } else {
            finished_link = format!(
                include_str!("../templates/message_contents/msglink.html"),
                board,
                &m[2..],
                "",
                &m
            );
        }
        // trimming a newline (that is there for some reason)
        result.push_str(&finished_link[..finished_link.len() - 1]);
        end_of_last = m_raw.end();
    }

    result.push_str(&inp_string[end_of_last..]);
    result
}
