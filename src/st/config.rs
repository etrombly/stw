use askama::Template;
use bcrypt::{hash, BcryptResult, DEFAULT_COST};
use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};

#[derive(Template, Clone, Debug, Serialize, Deserialize)]
#[template(path = "config.xml")]
pub struct ConfigTemplate {
    pub local_device_id: String,
    pub local_device_name: String,
    pub remote_device_id: String,
    pub remote_device_name: String,
    pub gui_password: String,
    pub folders: Vec<Folder>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Folder {
    pub id: String,
    pub path: String,
}

pub fn generate_password() -> BcryptResult<(String, String)> {
    let charset: Vec<u8> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*_-+="
        .as_bytes()
        .to_owned();
    let mut rng = thread_rng();
    let pass: String = (0..16)
        .map(|_| charset.choose(&mut rng).unwrap().to_owned() as char)
        .collect();
    let hash = hash(&pass, DEFAULT_COST)?;
    Ok((pass, hash))
}
