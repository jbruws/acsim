use chrono::{DateTime, FixedOffset, Local, NaiveDateTime};
use regex::Regex;

pub enum BoardMessageType {
    Message,       // messages on main page
    ParentMessage, // parent message on topic pages
    Submessage,    // submessages on topic pages
}

// returns current date in time in 'YYYY-MM-DD hh:mm:ss' 24-hour format
pub async fn get_time(since_epoch: i64) -> String {
    let offset = FixedOffset::east_opt(3 * 3600).unwrap(); // +3 (hours) offset
    let naive = NaiveDateTime::from_timestamp_opt(since_epoch, 0).unwrap(); // UNIX epoch to datetime
    let dt = DateTime::<Local>::from_naive_utc_and_offset(naive, offset).to_string();
    dt[..dt.len() - 7].to_string() // 7 was chosen experimentally
}

// fits form data into one of several html templates
pub async fn format_into_html(
    message_type: BoardMessageType,
    address: &str,
    id: &i64,
    time: &str,
    author: &str,
    msg: &str,
) -> String {
    let f_result = match message_type {
        BoardMessageType::Message => format!(
            include_str!("../templates/message_blocks/message.html"),
            id = id,
            address = address,
            time = time,
            author = author,
            msg = msg
        ),
        BoardMessageType::ParentMessage => format!(
            include_str!("../templates/message_blocks/parent_message.html"),
            address = address,
            time = time,
            author = author,
            id = id,
            msg = msg
        ),
        BoardMessageType::Submessage => format!(
            include_str!("../templates/message_blocks/submessage.html"),
            id = id,
            time = time,
            author = author,
            msg = msg
        ),
    };
    f_result
}

// removes html tags from message
pub async fn filter_string(inp_string: &String) -> String {
    let filter = Regex::new(r##"<.*?>"##).unwrap();
    String::from(filter.replace_all(inp_string.as_str(), ""))
}

// turns message raw text from the database into workable html,
// which is later piped into format_into_html()
pub async fn prepare_msg(inp_string: &String, addr: &String) -> String {
    // "#>" followed by numbers
    let msg_link_match = Regex::new(r##"#>\d+(\.\d+)?"##).unwrap();
    // direct link to an image
    let img_link_match = Regex::new(r##"https?:\/\/[^<>]*?\.(png|gif|jpg|jpeg|webp)"##).unwrap();

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

        // if it's a link to a submessage("#>dddd.dd")
        if m.contains(".") {
            let link_parts = m.split(".").collect::<Vec<&str>>();
            finished_link = format!(
                include_str!("../templates/message_contents/msglink.html"),
                addr,
                &link_parts[0][2..],
                &link_parts[1],
                &m
            );
        } else {
            finished_link = format!(
                include_str!("../templates/message_contents/msglink.html"),
                addr,
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
    start_of_next = 0; // resetting for second loop
    end_of_last = 0;

    // inserting <img> tags in place of image links
    let mut image_block = String::new();
    let mut second_result = String::new();
    let img_matches_iter = img_link_match.find_iter(&result);
    for m_raw in img_matches_iter {
        let m = m_raw.as_str();
        start_of_next = m_raw.start();
        second_result.push_str(&result[end_of_last..start_of_next]);
        image_block.push_str(&format!(
            include_str!("../templates/message_contents/single_image.html"),
            &m, &m
        ));
        end_of_last = m_raw.end();
    }

    second_result.push_str(&result[end_of_last..]);
    let final_res: String;
    if image_block == String::new() {
        // if there's no images
        final_res = format!(
            include_str!("../templates/message_contents/contents_noimg.html"),
            second_result
        );
    } else {
        final_res = format!(
            include_str!("../templates/message_contents/contents_img.html"),
            image_block, second_result
        );
    }
    final_res
}
