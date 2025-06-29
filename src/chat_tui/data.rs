use serde::Deserialize;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ChatMessage {
    pub user: String,
    pub icon: Option<String>,
    pub content: String,
    pub timestamp: Option<i64>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Rgb(pub u8, pub u8, pub u8);

#[derive(Deserialize, Debug)]
pub struct Theme {
    pub border: Rgb,
    pub border_focus: Rgb,
    pub button_focus: Rgb,
    pub text: Rgb,
}
