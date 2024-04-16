use std::io::prelude::*;
use std::os::unix::net::UnixListener;

fn main() -> std::io::Result<()> {
    let listener = UnixListener::bind("/tmp/rst.sock")?;

    match listener.accept() {
        Ok((mut socket, addr)) => {
            println!("Got a client: {:?} - {:?}", socket, addr);
            socket.write_all(b"hello world")?;
            let mut response = String::new();
            socket.read_to_string(&mut response)?;
            println!("{}", response);
        }
        Err(e) => println!("accept function failed: {:?}", e),
    }
    Ok(())
}
