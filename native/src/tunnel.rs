use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use hyper_util::client::proxy::matcher::Matcher;

const HEAD_MOST: usize = 8192;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Way {
    Direct,
    Http { host: String, port: u16, auth: Option<String> },
    Socks { host: String, port: u16, login: Option<(String, String)> },
}

fn own_machine(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost") || host.parse::<std::net::IpAddr>().is_ok_and(|address| address.is_loopback())
}

pub fn way_of(matcher: &Matcher, host: &str, port: u16, secure: bool) -> Result<Way, String> {
    if own_machine(host) {
        return Ok(Way::Direct);
    }
    let named = if host.contains(':') { format!("[{host}]") } else { host.to_owned() };
    let target: tungstenite::http::Uri = format!("{}://{named}:{port}", if secure { "https" } else { "http" }).parse().map_err(|_| format!("{host} is not a host name"))?;
    let Some(proxy) = matcher.intercept(&target) else { return Ok(Way::Direct) };
    let uri = proxy.uri();
    let at = uri.host().ok_or("the system proxy has no host")?.trim_matches(['[', ']']).to_owned();
    match uri.scheme_str().unwrap_or("http") {
        "http" => Ok(Way::Http {
            host: at,
            port: uri.port_u16().unwrap_or(80),
            auth: proxy.basic_auth().and_then(|value| value.to_str().ok()).map(str::to_owned),
        }),
        "socks5" | "socks5h" => Ok(Way::Socks {
            host: at,
            port: uri.port_u16().unwrap_or(1080),
            login: proxy.raw_auth().map(|(name, word)| (name.to_owned(), word.to_owned())),
        }),
        other => Err(format!("a {other} proxy cannot carry a live connection")),
    }
}

fn dial(host: &str, port: u16, patience: Duration) -> Result<TcpStream, String> {
    let mut why = format!("{host}:{port} has no address");
    for address in std::net::ToSocketAddrs::to_socket_addrs(&(host, port)).map_err(|why| why.to_string())? {
        match TcpStream::connect_timeout(&address, patience) {
            Ok(stream) => return Ok(stream),
            Err(error) => why = error.to_string(),
        }
    }
    Err(why)
}

fn through_http(stream: &mut TcpStream, host: &str, port: u16, auth: Option<&str>) -> Result<(), String> {
    let named = if host.contains(':') { format!("[{host}]") } else { host.to_owned() };
    let mut asked = format!("CONNECT {named}:{port} HTTP/1.1\r\nHost: {named}:{port}\r\n");
    if let Some(auth) = auth {
        asked.push_str(&format!("Proxy-Authorization: {auth}\r\n"));
    }
    asked.push_str("\r\n");
    stream.write_all(asked.as_bytes()).map_err(|why| why.to_string())?;
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        if head.len() >= HEAD_MOST {
            return Err("the proxy answered with too long a head".into());
        }
        stream.read_exact(&mut byte).map_err(|why| format!("the proxy did not answer: {why}"))?;
        head.push(byte[0]);
    }
    let head = String::from_utf8_lossy(&head);
    let status = head.lines().next().unwrap_or_default();
    match status.split_whitespace().nth(1) {
        Some("200") => Ok(()),
        _ => Err(format!("the proxy refused: {status}")),
    }
}

fn through_socks(stream: &mut TcpStream, host: &str, port: u16, login: Option<&(String, String)>) -> Result<(), String> {
    let said = |why: std::io::Error| format!("the proxy did not answer: {why}");
    let short = |text: &str| u8::try_from(text.len()).map_err(|_| "a name too long for the proxy".to_owned());
    stream.write_all(if login.is_some() { &[5, 2, 0, 2] } else { &[5, 1, 0] }).map_err(said)?;
    let mut chosen = [0u8; 2];
    stream.read_exact(&mut chosen).map_err(said)?;
    match (chosen, login) {
        ([5, 0], _) => {}
        ([5, 2], Some((name, word))) => {
            let mut asked = vec![1, short(name)?];
            asked.extend_from_slice(name.as_bytes());
            asked.push(short(word)?);
            asked.extend_from_slice(word.as_bytes());
            stream.write_all(&asked).map_err(said)?;
            let mut answer = [0u8; 2];
            stream.read_exact(&mut answer).map_err(said)?;
            if answer[1] != 0 {
                return Err("the proxy refused the login".into());
            }
        }
        _ => return Err("the proxy accepts no way of signing in that we have".into()),
    }
    let mut asked = vec![5, 1, 0, 3, short(host)?];
    asked.extend_from_slice(host.as_bytes());
    asked.extend_from_slice(&port.to_be_bytes());
    stream.write_all(&asked).map_err(said)?;
    let mut answer = [0u8; 4];
    stream.read_exact(&mut answer).map_err(said)?;
    if answer[1] != 0 {
        return Err(format!("the proxy refused: code {}", answer[1]));
    }
    let rest = match answer[3] {
        1 => 4,
        4 => 16,
        3 => {
            let mut length = [0u8; 1];
            stream.read_exact(&mut length).map_err(said)?;
            usize::from(length[0])
        }
        other => return Err(format!("the proxy answered with an address of kind {other}")),
    };
    let mut bound = vec![0u8; rest + 2];
    stream.read_exact(&mut bound).map_err(said)
}

pub fn through(way: &Way, host: &str, port: u16, patience: Duration) -> Result<TcpStream, String> {
    let (at, at_port) = match way {
        Way::Direct => return dial(host, port, patience),
        Way::Http { host, port, .. } | Way::Socks { host, port, .. } => (host.as_str(), *port),
    };
    let mut stream = dial(at, at_port, patience).map_err(|why| format!("the proxy {at}:{at_port} is not reachable: {why}"))?;
    stream.set_read_timeout(Some(patience)).map_err(|why| why.to_string())?;
    stream.set_write_timeout(Some(patience)).map_err(|why| why.to_string())?;
    match way {
        Way::Http { auth, .. } => through_http(&mut stream, host, port, auth.as_deref())?,
        Way::Socks { login, .. } => through_socks(&mut stream, host, port, login.as_ref())?,
        Way::Direct => {}
    }
    Ok(stream)
}

pub fn reach(host: &str, port: u16, secure: bool, patience: Duration) -> Result<TcpStream, String> {
    let way = way_of(&Matcher::from_system(), host, port, secure)?;
    through(&way, host, port, patience)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    const PATIENCE: Duration = Duration::from_secs(3);

    fn echo() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut said = [0u8; 4];
            stream.read_exact(&mut said).unwrap();
            stream.write_all(&said).unwrap();
        });
        port
    }

    fn pipe(mut near: TcpStream, far_port: u16) {
        let mut far = TcpStream::connect(("127.0.0.1", far_port)).unwrap();
        let mut said = [0u8; 4];
        near.read_exact(&mut said).unwrap();
        far.write_all(&said).unwrap();
        far.read_exact(&mut said).unwrap();
        near.write_all(&said).unwrap();
    }

    fn head(stream: &mut TcpStream) -> String {
        let mut head = Vec::new();
        let mut byte = [0u8; 1];
        while !head.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).unwrap();
            head.push(byte[0]);
        }
        String::from_utf8(head).unwrap()
    }

    fn talk(mut stream: TcpStream) {
        stream.write_all(b"ping").unwrap();
        let mut said = [0u8; 4];
        stream.read_exact(&mut said).unwrap();
        assert_eq!(&said, b"ping");
    }

    #[test]
    fn the_way_follows_the_proxy_settings_and_leaves_exceptions_direct() {
        let none = Matcher::builder().build();
        assert_eq!(way_of(&none, "bot.example", 443, true), Ok(Way::Direct));
        let plain = Matcher::builder().all("http://reader:word@127.0.0.1:7890").no("left.example").build();
        assert_eq!(way_of(&plain, "bot.example", 443, true), Ok(Way::Http { host: "127.0.0.1".into(), port: 7890, auth: Some("Basic cmVhZGVyOndvcmQ=".into()) }));
        assert_eq!(way_of(&plain, "bot.example", 80, false), Ok(Way::Http { host: "127.0.0.1".into(), port: 7890, auth: Some("Basic cmVhZGVyOndvcmQ=".into()) }));
        assert_eq!(way_of(&plain, "left.example", 443, true), Ok(Way::Direct));
        for own in ["127.0.0.1", "localhost", "::1"] {
            assert_eq!(way_of(&plain, own, 8080, false), Ok(Way::Direct), "a server on this machine is never asked for through a proxy");
        }
        let socks = Matcher::builder().all("socks5h://reader:word@127.0.0.1").build();
        assert_eq!(way_of(&socks, "bot.example", 443, true), Ok(Way::Socks { host: "127.0.0.1".into(), port: 1080, login: Some(("reader".into(), "word".into())) }));
        let secure = Matcher::builder().https("https://proxy.example:8443").build();
        assert!(way_of(&secure, "bot.example", 443, true).is_err(), "a proxy we cannot speak to is an error and never a silent direct connection");
        assert_eq!(way_of(&secure, "bot.example", 80, false), Ok(Way::Direct));
    }

    #[test]
    fn an_http_proxy_is_asked_to_connect_and_then_carries_the_bytes() {
        let target = echo();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let proxy = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let asked = head(&mut stream);
            stream.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n").unwrap();
            pipe(stream, target);
            asked
        });
        let way = Way::Http { host: "127.0.0.1".into(), port, auth: Some("Basic abc".into()) };
        talk(through(&way, "bot.example", 443, PATIENCE).unwrap());
        assert_eq!(proxy.join().unwrap(), "CONNECT bot.example:443 HTTP/1.1\r\nHost: bot.example:443\r\nProxy-Authorization: Basic abc\r\n\r\n");
    }

    #[test]
    fn a_proxy_that_refuses_or_is_gone_is_told_apart() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let proxy = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            head(&mut stream);
            stream.write_all(b"HTTP/1.1 407 Proxy Authentication Required\r\n\r\n").unwrap();
        });
        let way = Way::Http { host: "127.0.0.1".into(), port, auth: None };
        let why = through(&way, "bot.example", 443, PATIENCE).unwrap_err();
        assert!(why.contains("407"), "{why}");
        proxy.join().unwrap();
        let why = through(&way, "bot.example", 443, PATIENCE).unwrap_err();
        assert!(!why.contains("407"), "{why}");
    }

    #[test]
    fn a_socks_proxy_is_signed_in_to_and_given_the_host_by_name() {
        let target = echo();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let proxy = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut greeting = [0u8; 4];
            stream.read_exact(&mut greeting).unwrap();
            assert_eq!(greeting, [5, 2, 0, 2]);
            stream.write_all(&[5, 2]).unwrap();
            let mut login = [0u8; 13];
            stream.read_exact(&mut login).unwrap();
            assert_eq!(&login, b"\x01\x06reader\x04word");
            stream.write_all(&[1, 0]).unwrap();
            let mut asked = [0u8; 18];
            stream.read_exact(&mut asked).unwrap();
            assert_eq!(&asked, b"\x05\x01\x00\x03\x0bbot.example\x01\xbb");
            stream.write_all(&[5, 0, 0, 1, 0, 0, 0, 0, 0, 0]).unwrap();
            pipe(stream, target);
        });
        let way = Way::Socks { host: "127.0.0.1".into(), port, login: Some(("reader".into(), "word".into())) };
        talk(through(&way, "bot.example", 443, PATIENCE).unwrap());
        proxy.join().unwrap();
    }

    #[test]
    fn without_a_proxy_the_host_is_reached_directly() {
        let target = echo();
        talk(through(&Way::Direct, "127.0.0.1", target, PATIENCE).unwrap());
    }
}
