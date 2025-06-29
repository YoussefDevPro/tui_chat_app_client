use std::time::Instant;

pub enum Page {
    Auth,
    Home,
    Chat,
}

#[derive(PartialEq)]
pub enum AuthMode {
    Login,
    Register,
}

pub struct InputBox {
    pub value: String,
    pub cursor: usize,
    pub is_password: bool,
    pub label: String,
}

impl InputBox {
    pub fn new(label: &str, is_password: bool) -> Self {
        Self {
            value: String::new(),
            cursor: 0,
            is_password,
            label: label.to_string(),
        }
    }
    pub fn display(&self) -> String {
        if self.is_password {
            "bla".repeat(self.value.len())
        } else {
            self.value.clone()
        }
    }
}

pub struct App {
    pub page: Page,
    pub auth_mode: AuthMode,
    pub input_boxes: Vec<InputBox>,
    pub focus: usize,
    pub error: Option<String>,
    pub error_time: Option<Instant>, // <-- add this line
    pub token: Option<String>,
    pub is_loading: bool,
    pub chat_messages: Vec<crate::chat_tui::ChatMessage>,
    pub chat_input: String,
    pub should_quit: bool,
    pub icon_index: usize,
    pub icons: Vec<&'static str>,
    pub current_icon: String,
    pub chat_scroll: u16,
    pub input_scroll: u16,
    pub input_cursor: usize,
    pub input_width: usize,
    pub last_sent: Option<std::time::Instant>,
    pub auto_scroll: bool,
    pub max_scroll: u16,
}

impl App {
    pub fn new() -> Self {
        Self {
            page: Page::Auth,
            auth_mode: AuthMode::Register,
            input_boxes: vec![
                InputBox::new("Username", false),
                InputBox::new("Password", true),
                InputBox::new("Icon", false),
            ],
            focus: 0,
            error: None,
            error_time: None, // <-- initialize here
            token: None,
            is_loading: false,
            chat_input: String::new(),
            chat_messages: Vec::new(),
            should_quit: false,
            icon_index: 0,
            icons: vec![
                "󰱨",
                "󰱩",
                "󰱫",
                "󰄛",
                "󰊖",
                "󱃞",
                "󰱬",
                "󰱮",
                "󰱯",
                "󰱰",
                "󰽌",
                "󰱱",
                "󰱲",
                "󱈔",
                "󰱸",
                "󰇳",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "󱢯",
                "",
                "󰇟",
                "󰞅",
                "🤓👆",
                "🗿",
                "🧑‍💻",
                "󰱴",
                "󰇷",
                "󱕼",
                "󰇹",
                "",
                "",
                "",
                "",
                "",
            ],
            current_icon: String::new(),
            input_cursor: 0,
            chat_scroll: 0,
            input_scroll: 0,
            input_width: 0,
            last_sent: None,
            auto_scroll: true,
            max_scroll: 0,
        }
    }
}
