use chrono::{DateTime, FixedOffset, Local, NaiveDateTime};
use handlebars::Handlebars;
use regex::Regex;
use serde_json::json;
use std::fs::read_to_string;
use std::str;

/// Returns current date and time in 'YYYY-MM-DD hh:mm:ss' 24-hour format.
pub fn get_time(since_epoch: i64) -> String {
    let offset = FixedOffset::east_opt(3 * 3600).unwrap(); // +3 (hours) offset
    let naive = NaiveDateTime::from_timestamp_opt(since_epoch, 0).unwrap(); // UNIX epoch to datetime
    let dt = DateTime::<Local>::from_naive_utc_and_offset(naive, offset).to_string();
    dt[..dt.len() - 7].to_string() // 7 was chosen experimentally
}

/// Gets seconds elapsed since Unix epoch.
pub fn since_epoch() -> i64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(n) => n.as_secs().try_into().unwrap(),
        Err(_) => 1,
    }
}

#[derive(PartialEq)]
pub enum BoardMessageType {
    Message,       // messages on main page
    ParentMessage, // parent message on topic pages
    Submessage,    // submessages on topic pages
}

pub struct HtmlFormatter<'a> {
    pub work_dir: String,
    handle: Handlebars<'a>,
}

impl HtmlFormatter<'_> {
    pub fn new(frontend_name: String) -> HtmlFormatter<'static> {
        HtmlFormatter {
            work_dir: format!("./frontends/{}", frontend_name),
            handle: Handlebars::new(),
        }
    }

    /// Returns contents of specified file in work_dir
    fn get_file(&self, rel_path: &str) -> String {
        read_to_string(format!("{}/{}", &self.work_dir, rel_path))
            .unwrap_or_else(|_| panic!("Can't read {}/{}", &self.work_dir, rel_path))
    }

    /// Fits form data into one of several HTML templates.
    pub async fn format_into_template(
        &self,
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
        if !image.is_empty() {
            let mut formatted_img: String;
            // for messages in topics, we need do descend to parent dir
            if message_type == BoardMessageType::ParentMessage
                || message_type == BoardMessageType::Submessage
            {
                // descend two dirs (another dot is included in DB image path)
                formatted_img = String::from("../../");
                formatted_img.push_str(image);
            } else {
                formatted_img = format!("../{}", image);
            }

            msg_contents = self
                .handle
                .render_template(
                    &self.get_file("templates/message_contents/contents_img.html"),
                    &json!({"img_link": formatted_img, "msg": msg}),
                )
                .unwrap();
        } else {
            msg_contents = self
                .handle
                .render_template(
                    &self.get_file("templates/message_contents/contents_noimg.html"),
                    &json!({ "msg": msg }),
                )
                .unwrap();
        }

        match message_type {
            BoardMessageType::Message => self
                .handle
                .render_template(
                    &self.get_file("templates/message_blocks/message.html"),
                    &json!({"board": board,
                "id": id,
                "time": time,
                "author": author,
                "msg": msg_contents}),
                )
                .unwrap(),
            BoardMessageType::ParentMessage => self
                .handle
                .render_template(
                    &self.get_file("templates/message_blocks/parent_message.html"),
                    &json!({
                "time": time,
                "author": author,
                "id": id,
                "msg": msg_contents}),
                )
                .unwrap(),
            BoardMessageType::Submessage => self
                .handle
                .render_template(
                    &self.get_file("templates/message_blocks/submessage.html"),
                    &json!({"id": id,
                "time": time,
                "author": author,
                "msg": msg_contents}),
                )
                .unwrap(),
        }
    }

    /// Formats data into index.html (board pages)
    pub async fn format_into_index(
        &self,
        site_name: &String,
        board_designation: &String,
        board_desc: &String,
        random_tagline: &String,
        board_links: &String,
        inserted_msg: &String,
        current_page: i64,
    ) -> String {
        self.handle
            .render_template(
                &self.get_file("web-data/index.html"),
                &json!({"site_name": site_name,
            "board_designation": board_designation,
            "board_desc": board_desc,
            "random_tagline": random_tagline,
            "board_links": board_links,
            "messages": inserted_msg,
            //"front_path": self.work_dir[1..],
            "prev_p": current_page - 1,
            "next_p": current_page + 1}),
            )
            .unwrap()
    }

    /// Formats data into topic.html (topic pages)
    pub async fn format_into_topic(
        &self,
        site_name: &String,
        topic_number: &String,
        head_message: &String,
        submessages: &String,
        board_designation: &String,
    ) -> String {
        self.handle
            .render_template(
                &self.get_file("web-data/topic.html"),
                &json!({"site_name": site_name,
            "board_designation": board_designation,
            "topic_number": topic_number,
            "head_message": head_message,
            //"front_path": self.work_dir[1..],
            "submessages": submessages}),
            )
            .unwrap()
    }

    /// Removes HTML tags from strings.
    pub async fn filter_tags(&self, inp_string: &str) -> String {
        let filter = Regex::new(r##"<.*?>"##).unwrap();
        String::from(filter.replace_all(inp_string, ""))
    }

    /// Turns raw message text pulled from the database into workable HTML,
    /// which is later piped into other functions
    pub async fn create_formatting(&self, inp_string: &str) -> String {
        // regex strings
        let msg_link_match = Regex::new(r##"\w{1,16}>\d+(\.\d+)?"##).unwrap();
        let italic_match = Regex::new(r##"\*(?<text>[^*]*)\*"##).unwrap();
        let bold_match = Regex::new(r##"\*\*(?<text>[^*]*)\*\*"##).unwrap();
        let code_match = Regex::new(r##"`(?<text>[^`]*)`"##).unwrap();
        let newline_match = Regex::new(r##"(?<newline>\r\n+|\n+)"##).unwrap();

        let mut result = String::new();
        let mut start_of_next: usize; // start of next match
        let mut end_of_last: usize = 0; // end of previous match

        // inserting links to other messages
        let msg_matches_iter = msg_link_match.find_iter(inp_string);
        for m_raw in msg_matches_iter {
            let m = m_raw.as_str().to_string();

            start_of_next = m_raw.start();
            result.push_str(&inp_string[end_of_last..start_of_next]); // text between matches
            let separated = m.split('>').collect::<Vec<&str>>();

            // if it's a link to a submessage
            let finished_link: String = if m.contains('.') {
                let link_parts = separated[1].split('.').collect::<Vec<&str>>();
                self.handle.render_template(
                    self.get_file(
                        "templates/message_contents/msglink.html",
                    ).as_str(),
                    &json!({"board": separated[0], "topic_num": link_parts[0], "submsg_num": link_parts[1], "link": &m})
                ).unwrap()
            } else {
                self.handle.render_template(
                    self.get_file(
                        "templates/message_contents/msglink.html",
                    ).as_str(),
                    &json!({"board": separated[0], "topic_num": separated[1], "submsg_num": "", "link": &m})
                ).unwrap()
            };

            // trimming a newline (that is there for some reason)
            result.push_str(&finished_link[..finished_link.len() - 1]);
            end_of_last = m_raw.end();
        }
        result.push_str(&inp_string[end_of_last..]);

        // formatting
        result = newline_match.replace_all(&result, "<br>").to_string();
        result = bold_match
            .replace_all(&result, "<span class=\"bold\">${text}</span>")
            .to_string();
        result = italic_match
            .replace_all(&result, "<span class=\"italic\">${text}</span>")
            .to_string();
        result = code_match
            .replace_all(&result, "<span class=\"codeblock\">${text}</span>")
            .to_string();

        result
    }
}
