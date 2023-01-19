use serde_derive::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Badge {
    pub hover: String,
    pub image: String,
    pub link: String,
}

impl Badge {
    pub fn render(&self) -> String {
        format!(
            "[![{}]({})]({})",
            self.hover, self.image, self.link
        )
    }
}

pub trait Badgeable {
    fn badge(&self) -> Badge;
}
