use std::net::UdpSocket;
use std::io::{Result, ErrorKind};
use std::{thread, time};
use std::collections::HashMap;

pub fn start() -> Result<()> {
    let socket = UdpSocket::bind("127.0.0.1:42010")?;
    let sleep_intl = time::Duration::from_millis(100);
    let client_ttl = sleep_intl * 100;
    socket.set_nonblocking(true).unwrap();
    let mut clients = HashMap::new();
    let mut buf = [0; 10];
    loop {
        let now = time::SystemTime::now();
        match socket.recv_from(&mut buf) {
            Ok((_amt, src)) => {
                clients.insert(src, now);
                socket.send_to("hello".as_bytes(), &src)?;
            },
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
            error => panic!(error)
        };
        thread::sleep(sleep_intl);
        let old_len = clients.len();
        clients = clients.into_iter().filter(|&(_, time)| {now.duration_since(time).unwrap() < client_ttl}).collect();
        if (clients.len() == 0) && (old_len > 0) {
            break;
        }
    };
    Ok(())
}
