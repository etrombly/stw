use anyhow::Result;

use askama::Template;

use std::net::TcpStream;
use stw::{
    config::load_config,
    signal,
    st::{
        config::{ConfigTemplate, Folder},
        deviceid::get_device_id,
    },
};

fn main() -> Result<()> {
    //let device_id = get_device_id("/home/eric/.config/syncthing/cert.pem")?;
    //println!("{}", device_id);

    signal::init();
    let mut config = load_config(None)?;
    config.generate_config_templates()?;

    /*
    let mut local_folders = Vec::new();
    for folder in &config.folders {
        local_folders.push(Folder {
            id: folder.get_id(),
            path: folder.local_path.clone(),
        });
    }
    let local_config_template = ConfigTemplate {
        path: config.get_folder(),
        local_device_id: device_id.clone(),
        local_device_name: "local".into(),
        remote_device_id: device_id.clone(),
        remote_device_name: "remote".into(),
        folders: local_folders,
    };
    println!("{}", local_config_template.render().unwrap());
    */
    Ok(())
}
