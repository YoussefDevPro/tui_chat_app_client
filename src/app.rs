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
            "*".repeat(self.value.len())
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
    pub token: Option<String>,
    pub is_loading: bool,
    pub chat_messages: Vec<crate::chat_tui::ChatMessage>,
    pub chat_input: String,
    pub show_cmd_popup: bool,
    pub cmd_input: String,
    pub should_quit: bool,
    // Add your chat state here if needed
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
            token: None,
            is_loading: false,
            chat_input: String::new(),
            chat_messages: Vec::new(),
            should_quit: false,
            cmd_input: String::new(),
            show_cmd_popup: false,
        }
    }
}
