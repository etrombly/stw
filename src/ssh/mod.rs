use rpassword::read_password;
use ssh2::{PublicKey, Session};
use std::{net::TcpStream, path::Path};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SshError {
    #[error("ssh error")]
    Ssh(#[from] ssh2::Error),
    #[error("network error")]
    Io(#[from] std::io::Error),
    //#[error("Couldn't find config directory")]
    //NotFound,
}

pub fn create_session(host: &str, user: &str, key: Option<&impl AsRef<Path>>) -> Result<Session, SshError> {
    let tcp = TcpStream::connect(format!("{}:22", host))?;
    let mut sess = Session::new()?;
    sess.set_tcp_stream(tcp);
    sess.handshake()?;

    let mut agent = sess.agent()?;

    if agent.connect().is_ok() {
        match key {
            Some(key) => {
                let key = key.as_ref().to_string_lossy().to_string();
                agent.list_identities()?;
                for identity in &agent.identities()? {
                    if identity.comment() == key {
                        agent.userauth(user, identity)?
                    }
                }
            },
            None => {
                sess.userauth_agent(user)?;
            },
        }
    }
    if sess.authenticated() == false {
        match key {
            Some(key) => {
                let pubkey_location = format!("{:#?}.pub", key.as_ref());
                let pubkey = Path::new(&pubkey_location);
                let pubkey = match pubkey.exists() {
                    true => Some(pubkey),
                    false => None,
                };
                if sess.userauth_pubkey_file(user, pubkey, key.as_ref(), None).is_err() {
                    println!("Type ssh key password: ");
                    let password = read_password()?;
                    sess.userauth_pubkey_file(user, pubkey, key.as_ref(), Some(&password))?
                }
            },
            None => {
                println!("Type ssh password: ");
                let password = read_password()?;
                sess.userauth_password(user, &password)?;
            },
        }
    }
    Ok(sess)
}
