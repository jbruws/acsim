//! Functions for formatting database data into
//! HTML, which is then taken by the server to display to users.

use chrono::{offset::Local, DateTime, NaiveDateTime};
use handlebars::Handlebars;
use indexmap::map::IndexMap;
use rand::prelude::SliceRandom;
use regex::Regex;
use serde_json::json;
use std::fs::read_to_string;
use std::ops::Not;
use std::path::Path;
use std::str;

use crate::db_control::{MessageRow, SubmessageRow};
use crate::BoardConfig;

/// File categories that can be sent by users
#[derive(PartialEq)]
pub enum FileType {
    Image,
    Video,
    Invalid,
}

impl Not for FileType {
    type Output = bool;

    fn not(self) -> Self::Output {
        matches!(self, FileType::Invalid)
    }
}

/// Message types that can be formatted by `format_into_message`
#[derive(PartialEq)]
pub enum BoardMessageType {
    Message,        // messages on main page
    ParentMessage,  // parent message on topic pages
    Submessage,     // submessages on topic pages
    CatalogMessage, // message blocks in board catalog
}

/// Returns current date and time in 'YYYY-MM-DD hh:mm:ss' 24-hour format.
pub fn get_time(since_epoch: i64) -> String {
    let offset = *Local::now().offset(); // local offset
    let naive = NaiveDateTime::from_timestamp_opt(since_epoch, 0).unwrap(); // UNIX epoch to datetime
    let dt = DateTime::<Local>::from_naive_utc_and_offset(naive, offset).to_string();
    dt[..dt.len() - 7].to_string() // 7 was chosen experimentally
}

/// Validates images sent by users using libmagic
pub fn valid_file(image: &str) -> FileType {
    if image.is_empty() {
        return FileType::Invalid;
    }

    let mut image_fs_path: String = format!("./{}", image);
    if image.starts_with('/') {
        image_fs_path = image.to_string();
    }
    let image_fs_path = &image_fs_path[..image_fs_path.len()]; // path ends with "\" for some reason

    // libmagic image validation
    let cookie = magic::Cookie::open(magic::cookie::Flags::ERROR).unwrap();
    let database = Default::default();
    let cookie = cookie.load(&database).unwrap();
    let file_type = cookie.file(image_fs_path);

    if let Ok(raw_type) = file_type {
        if Path::new(image_fs_path).exists() {
            if raw_type.contains("image data") {
                return FileType::Image;
            } else if raw_type.contains("MP4 Base Media") || raw_type.contains("WebM") {
                return FileType::Video;
            }
        }
    }

    FileType::Invalid
}

/// Gets seconds elapsed since Unix epoch.
pub fn since_epoch() -> i64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(n) => n.as_secs().try_into().unwrap(),
        Err(_) => 1,
    }
}

/// Struct containing data necessary for data formatting, such as chosen frontend directory,
/// templating engine and list of regular expressions
pub struct HtmlFormatter<'a> {
    pub work_dir: String,
    handle: Handlebars<'a>,
    formatting_rules: IndexMap<String, String>,
}

impl HtmlFormatter<'_> {
    pub fn new(frontend_name: String) -> HtmlFormatter<'static> {
        let mut obj = HtmlFormatter {
            work_dir: format!("./frontends/{}", frontend_name),
            handle: Handlebars::new(),
            formatting_rules: IndexMap::new(),
        };

        let rules = match obj.load_rules() {
            Ok(r) => r,
            Err(_) => IndexMap::new(),
        };

        obj.formatting_rules = rules;
        obj
    }

    /// Returns contents of specified file from `work_dir` or its subdirectories
    fn get_file(&self, rel_path: &str) -> String {
        read_to_string(format!("{}/{}", &self.work_dir, rel_path))
            .unwrap_or_else(|_| panic!("Can't read {}/{}", &self.work_dir, rel_path))
    }

    /// Loads message formatting rules from YAML file
    fn load_rules(&self) -> Result<IndexMap<String, String>, serde_yaml::Error> {
        let raw_config = serde_yaml::from_str(&self.get_file("formatting_rules.yaml"))?;
        Ok(raw_config)
    }

    /// Turns string of file paths (separated by semicolons) into HTML image blocks
    pub fn process_image_data(&self, images: &str, message_type: &BoardMessageType) -> String {
        let mut image_container = String::new();
        for image in images.split(';') {
            let file_type = valid_file(image);
            if file_type != FileType::Invalid {
                let image_web_path: String = if message_type == &BoardMessageType::ParentMessage
                    || message_type == &BoardMessageType::Submessage
                {
                    // descend two dirs if message is in topic (/{board}/topic/*)
                    format!("../../{}", image)
                } else {
                    format!("../{}", image)
                };

                let template_path = match file_type {
                    FileType::Image => "templates/message_contents/image_block.html",
                    FileType::Video => "templates/message_contents/video_block.html",
                    _ => "templates/message_contents/image_block.html",
                };

                image_container.push_str(
                    &self
                        .handle
                        .render_template(
                            &self.get_file(template_path),
                            &json!({ "img_link": image_web_path, "img_name": image[12..]}),
                        )
                        .unwrap(),
                );
            }
        }
        image_container
    }

    /// Fits form data into submessage HTML template. Accepts `SubmessageRow` structs.
    pub async fn format_into_submessage(
        &self,
        db_row: SubmessageRow,
        _board: &str,
        _page: &str,
        msgid_override: i64,
    ) -> String {

        let msg = self.create_formatting(&db_row.submsg).await;

        // processing images
        let images = db_row.image;
        let image_container = self.process_image_data(&images, &BoardMessageType::Submessage);

        let msg_contents = self
            .handle
            .render_template(
                &self.get_file("templates/message_contents/contents.html"),
                &json!({"img_block": image_container, "msg": msg}),
            )
            .unwrap();

        self.handle
            .render_template(
                &self.get_file("templates/message_blocks/submessage.html"),
                &json!({"id": msgid_override,
                "time": get_time(db_row.time),
                "author": db_row.author,
                "msg": msg_contents}),
            )
            .unwrap()
    }

    /// Fits form data into one of several HTML templates. Only accepts `MessageRow` structs.
    pub async fn format_into_message(
        &self,
        message_type: BoardMessageType,
        db_row: MessageRow,
        board: &str,
        page: &str,
        msgid_override: Option<i64>,
    ) -> String {
        const CATALOG_MSG_LENGTH: usize = 200;

        let id = match msgid_override {
            Some(n) => n,
            None => db_row.msgid,
        };

        // formatting message body (and cropping if needed)
        let mut msg = db_row.msg;
        if message_type == BoardMessageType::CatalogMessage {
            msg = if msg.len() < CATALOG_MSG_LENGTH {
                msg
            } else {
                msg[0..CATALOG_MSG_LENGTH].to_string()
            };
        }
        msg = self.create_formatting(&msg).await;

        // processing images/videos
        let images = db_row.image;
        let image_container = self.process_image_data(&images, &message_type);

        let msg_contents = self
            .handle
            .render_template(
                &self.get_file("templates/message_contents/contents.html"),
                &json!({"img_block": image_container, "msg": msg}),
            )
            .unwrap();

        let time = get_time(db_row.time);
        let author = db_row.author;

        match message_type {
            BoardMessageType::Message => self
                .handle
                .render_template(
                    &self.get_file("templates/message_blocks/message.html"),
                    &json!({"board": board,
                "id": id,
                "time": time,
                "page": page,
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
                "page": page,
                "author": author,
                "id": id,
                "msg": msg_contents}),
                )
                .unwrap(),
            BoardMessageType::CatalogMessage => self
                .handle
                .render_template(
                    &self.get_file("templates/message_blocks/catalog_message.html"),
                    &json!({"id": id,
                "time": time,
                "board": board,
                "page": page,
                "msg": msg_contents}),
                )
                .unwrap(),
            _ => String::new(),
        }
    }

    /// Formats data into `board.html` (board pages)
    pub async fn format_into_board(
        &self,
        acsim_config: &BoardConfig,
        board_designation: &String,
        inserted_msg: &String,
        query_prev: &String,
        query_next: &String,
    ) -> String {
        // getting data about visited board
        let empty = String::from("");
        let board_desc = acsim_config.boards.get(board_designation).unwrap();
        let random_tagline = match acsim_config.taglines.choose(&mut rand::thread_rng()) {
            Some(s) => s,
            None => &empty,
        };

        // creating board link block
        let mut board_links = String::new();
        for c in acsim_config.boards.keys() {
            board_links.push_str(&format!("<a href=\"/{}\">/{}/</a>\n ", c, c));
        }

        self.handle
            .render_template(
                &self.get_file("web_data/board.html"),
                &json!({"site_name": acsim_config.site_name,
                "board_designation": board_designation,
                "board_desc": board_desc,
                "random_tagline": random_tagline,
                "board_links": board_links,
                "messages": inserted_msg,
                "query_prev": query_prev,
                "query_next": query_next,
                    }),
            )
            .unwrap()
    }

    /// Formats data into `index.html` (main page)
    pub async fn format_into_root(
        &self,
        site_name: &String,
        links: Vec<(&String, &String)>,
    ) -> String {
        let mut links_block = String::new();
        for i in links {
            links_block.push_str(&format!(
                "<div class=\"main_page_link\"><a href=\"/{i}\">/{i}/ - {desc}</a></div><hr>",
                i = i.0,
                desc = i.1
            ));
        }

        self.handle
            .render_template(
                &self.get_file("web_data/index.html"),
                &json!({"board_name": site_name, "links_block": links_block}),
            )
            .unwrap()
    }

    /// Formats data into `topic.html` (topic pages)
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
                &self.get_file("web_data/topic.html"),
                &json!({"site_name": site_name,
            "board_designation": board_designation,
            "topic_number": topic_number,
            "head_message": head_message,
            "submessages": submessages}),
            )
            .unwrap()
    }

    /// Formats data into board catalog pages
    pub async fn format_into_catalog(
        &self,
        board_designation: &String,
        message_blocks: &String,
        query_data_prev: &String,
        query_data_next: &String,
    ) -> String {
        self.handle
            .render_template(
                &self.get_file("web_data/catalog.html"),
                &json!({"board_designation": board_designation,
                    "message_blocks": message_blocks,
                    "query_data_prev": query_data_prev,
                    "query_data_next": query_data_next,
                }),
            )
            .unwrap()
    }

    /// Removes HTML tags from strings. Called when writing data to database
    pub async fn filter_tags(&self, inp_string: &str) -> String {
        let filter = Regex::new(r##"<.*?>"##).unwrap();
        String::from(filter.replace_all(inp_string, ""))
    }

    /// Turns raw message text pulled from the database into workable HTML,
    /// which is later piped into other functions. Called when loading data from database
    pub async fn create_formatting(&self, inp_string: &str) -> String {
        let mut result = String::from(inp_string);

        for (template, expr) in self.formatting_rules.iter() {
            result = Regex::new(expr)
                .unwrap()
                .replace_all(&result, template)
                .to_string();
        }

        result
    }
}
