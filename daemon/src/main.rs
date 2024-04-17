use aes_gcm::{
    aead::{Aead, KeyInit, Nonce, OsRng},
    Aes256Gcm,
};
use anyhow::Result;
use std::fs::{self, File, Permissions};
use std::io::{prelude::*, BufReader};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use virtualdevice::VirtualInput;

mod config;
mod virtualdevice;

fn main() -> Result<()> {
    let socket_path = "/run/tempest.socket";
    // Remove the socket if it exists
    let _ = fs::remove_file(socket_path);
    let listener = UnixListener::bind(socket_path)?;
    fs::set_permissions(socket_path, Permissions::from_mode(0o622))?;

    let key = Aes256Gcm::generate_key(OsRng);
    let cipher = Aes256Gcm::new(&key);
    println!("please use this token to authenticate with the daemon");
    println!("{}", hex::encode(key));

    let conf: config::RawConfig = {
        let reader = File::open("config.yml")?;
        serde_yaml::from_reader(reader)?
    };
    let conf: config::Config = conf.into();
    let mut device = VirtualInput::new()?;

    match listener.accept() {
        Ok((socket, addr)) => {
            println!("Got a client: {:?} - {:?}", socket, addr);
            let mut reader = BufReader::new(&socket);
            let mut response = String::new();
            while let Ok(bytes_read) = reader.read_line(&mut response) {
                if bytes_read == 0 {
                    println!("Got an end of stream, ignoring");
                    continue;
                }
                let resp = response.trim();
                println!("got response: {resp}");
                let bytes = hex::decode(&resp)?;
                println!("which decodes to: {bytes:?}");
                let nonce_bytes = &bytes[..12];
                let ciphertext = &bytes[12..];

                println!("nonce: {nonce_bytes:?}");
                println!("ciphertext: {ciphertext:?}");
                let nonce = Nonce::<Aes256Gcm>::clone_from_slice(nonce_bytes);
                let Ok(resp) = cipher.decrypt(&nonce, ciphertext.as_ref()) else {
                    eprintln!("failed to decrypt ciphertext sent by client");
                    continue;
                };
                let resp = std::str::from_utf8(&resp)?;
                println!("got: {}", resp);
                if let Some(config::Action::Keys(keys)) = conf.actions.get(resp) {
                    device.key_chord(keys.as_slice());
                }
                response = String::new();
            }
        }
        Err(e) => println!("accept function failed: {:?}", e),
    }
    Ok(())
}
