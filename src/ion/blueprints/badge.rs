use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Badge {
    pub hover: String,
    pub image: String,
    pub link: String,
}

impl Badge {
    pub fn render(&self) -> String {
        format!("[![{}]({})]({})", self.hover, self.image, self.link)
    }
}

pub trait Badgeable {
    fn badge(&self) -> Badge;
}
