use std::net::UdpSocket;
use std::io::{Result, ErrorKind};
use std::{thread, time};
use std::collections::HashMap;

pub fn start<FA, FB, FC>(start_fn: FA, looper: FB, end_fn: FC) -> Result<()>
    where FA: Fn() -> (), FB: Fn() -> usize, FC: Fn() -> ()
{
    let socket = UdpSocket::bind("0.0.0.0:42010")?;
    let sleep_intl = time::Duration::from_millis(100);
    let client_ttl = sleep_intl * 100;
    socket.set_nonblocking(true).unwrap();
    let mut clients = HashMap::new();
    let mut buf = [0; 10];
    loop {
        let now = time::SystemTime::now();
        let old_len = clients.len();
        match socket.recv_from(&mut buf) {
            Ok((_amt, src)) => {
                clients.insert(src, now);
            },
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
            error => panic!(error)
        };
        let calls = looper();
        /* FIXME: fold Result instead of foreach */
        if calls > 0 {
            let message = format!("calls: {}\n", calls);
            clients.iter().for_each(|(client, _)| {
                socket.send_to(message.as_bytes(), client).unwrap();
            });
        }
        thread::sleep(sleep_intl);
        clients = clients.into_iter().filter(|&(_, time)| {now.duration_since(time).unwrap() < client_ttl}).collect();
        if old_len != clients.len() {
            if old_len == 0 {
                start_fn();
            }
            if clients.len() == 0 {
                end_fn();
            }
        }
    };
}
