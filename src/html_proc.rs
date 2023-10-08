//! Functions for formatting database data into
//! HTML, which is then taken by the server to display to users.

use chrono::{offset::Local, DateTime, NaiveDateTime};
use handlebars::Handlebars;
use regex::Regex;
use serde_json::json;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::str;

/// Returns current date and time in 'YYYY-MM-DD hh:mm:ss' 24-hour format.
pub fn get_time(since_epoch: i64) -> String {
    let offset = *Local::now().offset(); // local offset
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

/// Enum representing three message types: board messages, topic head messages and topic submessages
#[derive(PartialEq)]
pub enum BoardMessageType {
    Message,       // messages on main page
    ParentMessage, // parent message on topic pages
    Submessage,    // submessages on topic pages
}

/// Struct used to choose ACSIM frontend directory
pub struct HtmlFormatter<'a> {
    pub work_dir: String,
    handle: Handlebars<'a>,
    template_map: HashMap<&'a str, String>,
}

impl HtmlFormatter<'_> {
    pub fn new(frontend_name: String) -> HtmlFormatter<'static> {
        let mut obj = HtmlFormatter {
            work_dir: format!("./frontends/{}", frontend_name),
            handle: Handlebars::new(),
            template_map: HashMap::new(),
        };

        // loading regex templates
        obj.template_map
            .insert("quote", obj.get_file("regex-templates/quote.html"));
        obj.template_map
            .insert("newline", String::from("<br>"));
        obj.template_map
            .insert("msglink", obj.get_file("regex-templates/msglink.html"));
        obj.template_map
            .insert("codeblock", obj.get_file("regex-templates/codeblock.html"));
        obj.template_map
            .insert("bold", obj.get_file("regex-templates/bold.html"));
        obj.template_map
            .insert("italic", obj.get_file("regex-templates/italic.html"));

        obj
    }

    /// Returns contents of specified file in `work_dir`
    fn get_file(&self, rel_path: &str) -> String {
        read_to_string(format!("{}/{}", &self.work_dir, rel_path))
            .unwrap_or_else(|_| panic!("Can't read {}/{}", &self.work_dir, rel_path))
    }

    /// Fits form data into one of several HTML message templates.
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
        let mut result = String::from(inp_string);

        // regex strings
        let msg_link_match =
            Regex::new(r##"(?<board>\w{1,16})>(?<msg>\d+)(?<dotted>\.(?<submsg>\d+))?"##).unwrap();
        let italic_match = Regex::new(r##"\*(?<text>[^*]*)\*"##).unwrap();
        let bold_match = Regex::new(r##"\*\*(?<text>[^*]*)\*\*"##).unwrap();
        let code_match = Regex::new(r##"`(?<text>[^`]*)`"##).unwrap();
        let quote_match = Regex::new(r##"(^|(?<nl>\n))(?<text>>[^\n]+)"##).unwrap();
        let newline_match = Regex::new(r##"(?<newline>(\r\n)+|(\n+))"##).unwrap();

        // formatting
        result = quote_match.replace_all(&result, self.template_map.get("quote").unwrap()).to_string(); 
        result = newline_match.replace_all(&result, self.template_map.get("newline").unwrap()).to_string(); 
        result = msg_link_match.replace_all(&result, self.template_map.get("msglink").unwrap()).to_string(); 
        result = code_match.replace_all(&result, self.template_map.get("codeblock").unwrap()).to_string(); 
        result = bold_match.replace_all(&result, self.template_map.get("bold").unwrap()).to_string(); 
        result = italic_match.replace_all(&result, self.template_map.get("italic").unwrap()).to_string(); 

        result
    }
}
