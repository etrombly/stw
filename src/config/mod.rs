use askama::Template;
use directories::ProjectDirs;
use gethostname::gethostname;
use md5;
use openssl::{
    asn1::Asn1Time,
    bn::{BigNum, MsbOption},
    ec::{EcGroup, EcKey},
    hash::MessageDigest,
    nid::Nid,
    pkey::PKey,
    x509::{X509Extension, X509},
};
use serde::{Deserialize, Serialize};
use std::{
    env,
    fs::{self, File},
    include_bytes,
    io::{ Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    path::Path,
    sync::Mutex,
    thread,
    time::Duration,
};
use thiserror::Error;
use typed_path::{PathBuf, UnixEncoding};

use crate::{
    ssh::{create_session, SshError},
    st::{
        config::{self, ConfigTemplate},
        deviceid::get_device_id,
    },
    CHANNEL,
};

#[derive(Error, Debug)]
pub enum ConfError {
    #[error("Error parsing config to yaml")]
    Yaml(#[from] serde_yaml::Error),
    #[error("config file IO error")]
    Io(#[from] std::io::Error),
    #[error("Error templating config file")]
    Askama(#[from] askama::Error),
    #[error("ssh error")]
    Ssh2(#[from] ssh2::Error),
    #[error("ssh error")]
    Ssh(#[from] SshError),
    #[error("syncthing lib error")]
    St(#[from] crate::st::error::Error),
    #[error("Couldn't find config directory")]
    NotFound,
    #[error("Couldn't create remote directory")]
    RemoteFolder,
    #[error("Couldn't set channel")]
    Channel,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(unused)]
pub struct Conf {
    pub remote_address: String,
    pub remote_user: String,
    pub ssh_key: Option<String>,
    pub folders: Vec<Folder>,
    pub local_config: Option<ConfigTemplate>,
    pub remote_config: Option<ConfigTemplate>,
}

impl Conf {
    pub fn get_folder(&self) -> String {
        let digest = md5::compute(format!("{}{}", &self.remote_address, gethostname().to_string_lossy().to_string()));
        format!("{:x}", digest)
    }

    pub fn generate_config_templates(&mut self) -> Result<(), ConfError> {
        // Find local config folder for session
        let config_folder = self.get_folder();
        let local_config_folder = match ProjectDirs::from("com", "etromb", "stw") {
            Some(proj_dirs) => {
                let conf_dir = proj_dirs.config_dir();
                let conf_dir = conf_dir.join(&config_folder);
                Ok(conf_dir)
            },
            None => Err(ConfError::NotFound),
        }?;

        // Verify connectivity and find config folder for remote session
        let session = create_session(&self.remote_address, &self.remote_user, self.ssh_key.as_ref())?;

        println!("Creating remote config folder");
        let mut channel = session.channel_session()?;
        channel.exec("eval echo ~$USER")?;
        let mut s = String::new();
        channel.read_to_string(&mut s)?;
        channel.wait_close()?;
        let remote_config_folder = PathBuf::<UnixEncoding>::from(&s.trim())
            .join(".config/stw/")
            .join(&config_folder);
        let remote_data_folder = PathBuf::<UnixEncoding>::from(&s.trim()).join(".local/share/stw/");
        let mut channel = session.channel_session()?;
        channel.exec("hostname")?;
        let mut s = String::new();
        channel.read_to_string(&mut s)?;
        channel.wait_close()?;
        let remote_hostname = s.trim();

        // Create local config folder
        if !local_config_folder.exists() {
            fs::create_dir_all(&local_config_folder)?;
        }

        // Create local keypairs
        let local_hostname = gethostname().to_string_lossy().to_string();
        // set cn to syncthing instead of hostname
        let local_keypair = KeyPair::new("syncthing");
        let local_key_path = local_config_folder.join("key.pem");
        let local_cert_path = local_config_folder.join("cert.pem");
        {
            let mut local_key = File::create(&local_key_path)?;
            local_key.write_all(local_keypair.key.as_bytes())?;
            let mut local_cert = File::create(&local_cert_path)?;
            local_cert.write_all(local_keypair.cert.as_bytes())?;
        }
        let local_device_id = get_device_id(&local_keypair.cert)?;

        // Create remote config folder
        let mut channel = session.channel_session()?;
        channel.exec(&format!(
            "mkdir -p {:#?}",
            remote_config_folder.as_path().to_string_lossy()
        ))?;
        let mut s = String::new();
        channel.read_to_string(&mut s)?;
        channel.wait_close()?;
        if channel.exit_status()? != 0 {
            return Err(ConfError::RemoteFolder);
        }

        // Create remote data folder
        let mut channel = session.channel_session()?;
        channel.exec(&format!(
            "mkdir -p {:#?}",
            remote_data_folder.as_path().to_string_lossy()
        ))?;
        let mut s = String::new();
        channel.read_to_string(&mut s)?;
        channel.wait_close()?;
        if channel.exit_status()? != 0 {
            return Err(ConfError::RemoteFolder);
        }

        println!("Uploading syncthing to remote");
        // Upload syncthing to remote
        let remote_syncthing_path = remote_data_folder.join("syncthing");
        let remote_syncthing_path = Path::new(remote_syncthing_path.as_path().to_str().unwrap());
        let syncthing_binary_compressed = include_bytes!("../../resources/syncthing-linux-amd64-v1.21.0.xz");
        let mut f = std::io::Cursor::new(syncthing_binary_compressed);
        let mut syncthing_binary = Vec::new();
        lzma_rs::xz_decompress(&mut f, &mut syncthing_binary).unwrap();
        let mut sent = 0;
        let size = syncthing_binary.len();
        let mut remote_syncthing = session.scp_send(remote_syncthing_path, 0o755, size as u64, None)?;
        while sent < size {
            sent += remote_syncthing.write(&syncthing_binary[sent..])?;
        }
        remote_syncthing.send_eof().unwrap();
        remote_syncthing.wait_eof().unwrap();
        remote_syncthing.close().unwrap();
        remote_syncthing.wait_close().unwrap();

        println!("Uploading config to remote");
        // Create remote keypair
        // set cn to syncthing instead of hostname
        let remote_keypair = KeyPair::new("syncthing");
        let remote_key_path = remote_config_folder.join("key.pem");
        let remote_key_path = Path::new(remote_key_path.as_path().to_str().unwrap());
        let remote_cert_path = remote_config_folder.join("cert.pem");
        let remote_cert_path = Path::new(remote_cert_path.as_path().to_str().unwrap());
        {
            let mut remote_key = session.scp_send(
                remote_key_path,
                0o640,
                remote_keypair.key.as_bytes().len() as u64,
                None,
            )?;
            remote_key.write_all(remote_keypair.key.as_bytes())?;
            remote_key.send_eof().unwrap();
            remote_key.wait_eof().unwrap();
            remote_key.close().unwrap();
            remote_key.wait_close().unwrap();
            let mut remote_cert = session.scp_send(
                remote_cert_path,
                0o640,
                remote_keypair.cert.as_bytes().len() as u64,
                None,
            )?;
            remote_cert.write_all(remote_keypair.cert.as_bytes())?;
            remote_cert.send_eof().unwrap();
            remote_cert.wait_eof().unwrap();
            remote_cert.close().unwrap();
            remote_cert.wait_close().unwrap();
        }
        let remote_device_id = get_device_id(&remote_keypair.cert)?;

        // Generate local config file
        let local_folders = self
            .folders
            .iter()
            .map(|x| config::Folder {
                id: x.get_id(),
                path: x.local_path.clone(),
            })
            .collect();
        let local_config = ConfigTemplate {
            local_device_id: local_device_id.clone(),
            local_device_name: local_hostname.clone(),
            remote_device_id: remote_device_id.clone(),
            remote_device_name: remote_hostname.into(),
            folders: local_folders,
        };

        let local_config_file_path = local_config_folder.join("config.xml");
        {
            let mut local_config_file = File::create(&local_config_file_path)?;
            local_config_file.write_all(local_config.render()?.as_bytes())?;
        }

        // Generate remote config file
        let remote_folders = self
            .folders
            .iter()
            .map(|x| config::Folder {
                id: x.get_id(),
                path: x.remote_path.clone(),
            })
            .collect();
        let remote_config = ConfigTemplate {
            local_device_id: remote_device_id,
            local_device_name: remote_hostname.into(),
            remote_device_id: local_device_id,
            remote_device_name: local_hostname,
            folders: remote_folders,
        };
        let remote_config = remote_config.render()?;

        let remote_config_file_path = remote_config_folder.join("config.xml");
        let remote_config_file_path = Path::new(remote_config_file_path.as_path().to_str().unwrap());
        {
            let mut remote_config_file = session.scp_send(
                &remote_config_file_path,
                0o640,
                remote_config.as_bytes().len() as u64,
                None,
            )?;
            remote_config_file.write_all(&remote_config.as_bytes())?;
            remote_config_file.send_eof().unwrap();
            remote_config_file.wait_eof().unwrap();
            remote_config_file.close().unwrap();
            remote_config_file.wait_close().unwrap();
        }

        // set ssh session to non-blocking
        session.set_blocking(false);

        // create remote port forward
        let (mut remote_listener, _) = loop {
            let remote_listener = session.channel_forward_listen(22001, None, None);
            if let Ok(remote_listener) = remote_listener {
                break remote_listener;
            }
        };

        // craete local listener to forward
        let local_listener = TcpListener::bind("127.0.0.1:22001")?;
        local_listener.set_nonblocking(true)?;

        // create remote channel to run syncthing
        let mut channel = loop {
            let channel = session.channel_session();
            if let Ok(channel) = channel {
                break channel;
            }
        };
        while channel.request_pty("xterm", None, None).is_err() {}
        while channel
            .exec(&format!(
                "{:#?} serve --home={:#?}",
                remote_syncthing_path,
                remote_config_folder.as_path().to_str().unwrap()
            ))
            .is_err()
        {}
        unsafe {
            CHANNEL.set(Mutex::new(channel)).map_err(|_| ConfError::Channel)?;
        }
        println!("Remote syncthing started");
        println!(
            "Run `syncthing serve --home={:#?}` on local machine to sync",
            local_config_folder
        );

        loop {
            match remote_listener.accept() {
                Ok(mut channel) => {
                    let mut stream = loop {
                        let stream = TcpStream::connect("127.0.0.1:22000");
                        if let Ok(stream) = stream {
                            break stream;
                        }
                    };
                    thread::spawn(move || loop {
                        let mut buf = [0_u8; 128];
                        match channel.read(&mut buf) {
                            Ok(amount) => {
                                if amount == 0 {
                                    stream.shutdown(Shutdown::Both).unwrap();
                                    break;
                                }
                                stream.write_all(&buf[0..amount]).unwrap();
                            },
                            Err(x) => {},
                        }
                        match stream.read(&mut buf) {
                            Ok(amount) => {
                                if amount == 0 {
                                    while channel.send_eof().is_err() {}
                                    while channel.wait_eof().is_err() {}
                                    break;
                                }
                                channel.write_all(&buf[0..amount]).unwrap();
                            },
                            Err(x) => {},
                        }
                        if channel.eof() {
                            stream.shutdown(Shutdown::Both).unwrap();
                            break;
                        }
                        thread::sleep(Duration::new(0, 1));
                    });
                },
                Err(_) => {},
            };
            match local_listener.accept() {
                Ok((mut stream, _)) => {
                    stream.set_nonblocking(true).unwrap();
                    let mut channel = loop {
                        let channel = session.channel_direct_tcpip("127.0.0.1", 22000, None);
                        if let Ok(channel) = channel {
                            break channel;
                        }
                    };
                    thread::spawn(move || loop {
                        let mut buf = [0_u8; 128];
                        match stream.read(&mut buf) {
                            Ok(amount) => {
                                if amount == 0 {
                                    while channel.send_eof().is_err() {}
                                    while channel.wait_eof().is_err() {}
                                    break;
                                }
                                channel.write_all(&buf[0..amount]).unwrap();
                            },
                            Err(x) => {},
                        };
                        match channel.read(&mut buf) {
                            Ok(amount) => {
                                if amount == 0 {
                                    stream.shutdown(Shutdown::Both).unwrap();
                                    break;
                                }
                                stream.write_all(&buf[0..amount]).unwrap();
                            },
                            Err(x) => {},
                        }
                        thread::sleep(Duration::new(0, 1));
                    });
                },
                Err(_) => {},
            }
            thread::sleep(Duration::new(0, 1));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Folder {
    pub local_path: String,
    pub remote_path: String,
}

impl Folder {
    pub fn get_id(&self) -> String {
        let digest = md5::compute(&self.local_path);
        format!("{:x}", digest)
    }
}

pub struct KeyPair {
    key: String,
    cert: String,
}

impl KeyPair {
    pub fn new(cn: &str) -> KeyPair {
        let ec_group = EcGroup::from_curve_name(Nid::SECP384R1).unwrap();
        let ec = EcKey::generate(&ec_group).unwrap();
        let pkey = PKey::from_ec_key(ec).unwrap();

        let mut subject_name = openssl::x509::X509NameBuilder::new().unwrap();
        subject_name.append_entry_by_text("O", "Syncthing").unwrap();
        subject_name
            .append_entry_by_text("OU", "Automatically Generated")
            .unwrap();
        subject_name.append_entry_by_text("CN", cn).unwrap();
        let subject_name = subject_name.build();
        let mut cert = X509::builder().unwrap();
        cert.set_version(2).unwrap();
        let serial_number = {
            let mut serial = BigNum::new().unwrap();
            serial.rand(159, MsbOption::MAYBE_ZERO, false).unwrap();
            serial.to_asn1_integer().unwrap()
        };
        cert.set_serial_number(&serial_number).unwrap();
        let context = cert.x509v3_context(None, None);
        let alternate_name =
            X509Extension::new(None, Some(&context), "subjectAltName", &format!("DNS:{}", cn)).unwrap();
        cert.append_extension(alternate_name).unwrap();
        let context = cert.x509v3_context(None, None);
        let key_usage =
            X509Extension::new(None, Some(&context), "keyUsage", "keyEncipherment, digitalSignature").unwrap();
        cert.append_extension(key_usage).unwrap();
        let context = cert.x509v3_context(None, None);
        let extended_key_usage =
            X509Extension::new(None, Some(&context), "extendedKeyUsage", "serverAuth, clientAuth").unwrap();
        cert.append_extension(extended_key_usage).unwrap();
        cert.set_subject_name(&subject_name).unwrap();
        cert.set_pubkey(&pkey).unwrap();
        cert.set_not_before(&Asn1Time::days_from_now(0).unwrap()).unwrap();
        cert.set_not_after(&Asn1Time::days_from_now(3650).unwrap()).unwrap();
        cert.sign(&pkey, MessageDigest::sha256()).unwrap();
        let cert = String::from_utf8(cert.build().to_pem().unwrap()).unwrap();
        let key = String::from_utf8(pkey.private_key_to_pem_pkcs8().unwrap()).unwrap();
        KeyPair { key, cert }
    }
}

pub fn load_config(conf_path: Option<String>) -> Result<Conf, ConfError> {
    let conf_dir = match conf_path {
        Some(path) => path.into(),
        None => env::current_dir()?,
    };
    let conf_file = conf_dir.join("config.yml");
    let conf: Conf = serde_yaml::from_str(&fs::read_to_string(conf_file)?)?;
    Ok(conf)
}
