use anyhow::Result;
use std::fs::{self, File, Permissions};
use std::io::{prelude::*, BufReader};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::time::Duration;
use virtualdevice::VirtualInput;

mod config;
mod virtualdevice;

fn main() -> Result<()> {
    let socket_path = "/tmp/tempest.socket";
    // Remove the socket if it exists
    let _ = fs::remove_file(socket_path);
    let listener = UnixListener::bind(socket_path)?;
    fs::set_permissions(socket_path, Permissions::from_mode(0o622))?;

    let conf: config::RawConfig = {
        let reader = File::open("config.yml")?;
        serde_yaml::from_reader(reader)?
    };
    let conf: config::Config = conf.into();
    let mut device = VirtualInput::new();

    match listener.accept() {
        Ok((mut socket, addr)) => {
            println!("Got a client: {:?} - {:?}", socket, addr);
            let mut reader = BufReader::new(&socket);
            let mut response = String::new();
            while let Ok(_bytes_read) = reader.read_line(&mut response) {
                let resp = response.trim();
                println!("got: {}", resp);
                if let Some(action) = conf.actions.get(resp) {
                    println!("{:?}", action);
                }
                response = String::new();
            }
        }
        Err(e) => println!("accept function failed: {:?}", e),
    }
    Ok(())
}
