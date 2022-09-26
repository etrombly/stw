use askama::Template;
use serde::{Deserialize, Serialize};

#[derive(Template, Clone, Debug, Serialize, Deserialize)]
#[template(path = "config.xml")]
pub struct ConfigTemplate {
    pub local_device_id: String,
    pub local_device_name: String,
    pub remote_device_id: String,
    pub remote_device_name: String,
    pub folders: Vec<Folder>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Folder {
    pub id: String,
    pub path: String,
}
