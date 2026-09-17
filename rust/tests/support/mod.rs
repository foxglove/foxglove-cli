use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub struct Workspace(pub PathBuf);

impl Workspace {
    pub fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "foxglove-integration-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    pub fn command(&self, base_url: &str) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_foxglove-rust"));
        command
            .current_dir(&self.0)
            .env("HOME", &self.0)
            .env("USERPROFILE", &self.0)
            .env("BASE_URL", base_url)
            .env("BEARER_TOKEN", "fixture")
            .env_remove("AUTH_TYPE")
            .env_remove("DEFAULT_PROJECT_ID")
            .env_remove("SSL_CERT_FILE")
            .env_remove("SSL_CERT_DIR")
            .env_remove("HTTP_PROXY")
            .env_remove("HTTPS_PROXY")
            .env_remove("ALL_PROXY")
            .env_remove("http_proxy")
            .env_remove("https_proxy")
            .env_remove("all_proxy")
            .env("NO_PROXY", "*")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub struct Process(Option<Child>);

impl Process {
    pub fn spawn(command: &mut Command) -> Self {
        Self(Some(command.spawn().unwrap()))
    }

    #[cfg(all(unix, feature = "compat-test"))]
    pub fn interrupt(&self) {
        assert!(Command::new("kill")
            .args(["-INT", &self.0.as_ref().unwrap().id().to_string()])
            .status()
            .unwrap()
            .success());
    }

    pub fn finish(mut self) -> Output {
        let deadline = Instant::now() + Duration::from_secs(30);
        while self.0.as_mut().unwrap().try_wait().unwrap().is_none() {
            assert!(Instant::now() < deadline, "CLI timed out");
            thread::sleep(Duration::from_millis(10));
        }
        self.0.take().unwrap().wait_with_output().unwrap()
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

pub struct Reply {
    pub method: &'static str,
    pub path: &'static str,
    pub status: u16,
    pub body: Vec<u8>,
    /// Promise more body than is sent, then hold the connection open.
    pub stall: bool,
    /// Promise more body than is sent, then close. The client sees the transfer
    /// fail after it has already accepted part of the response.
    pub truncate: bool,
}

impl Reply {
    pub fn json(method: &'static str, path: &'static str, body: &str) -> Self {
        Self {
            method,
            path,
            status: 200,
            body: body.as_bytes().to_vec(),
            stall: false,
            truncate: false,
        }
    }
}

pub struct Server {
    pub url: String,
    pub requests: Receiver<String>,
    task: JoinHandle<()>,
}

impl Server {
    pub fn new(replies: Vec<Reply>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server_url = url.clone();
        let (sender, requests) = mpsc::channel();
        let task = thread::spawn(move || {
            for reply in replies {
                let mut stream = accept(&listener);
                let request = read_request(&mut stream);
                let mut words = request.split_whitespace();
                assert_eq!(words.next().unwrap(), reply.method);
                let target = words.next().unwrap();
                assert_eq!(target.split('?').next().unwrap(), reply.path);
                let body = if reply.path == "/v1/data/stream" {
                    String::from_utf8(reply.body)
                        .unwrap()
                        .replace("{BASE_URL}", &server_url)
                        .into_bytes()
                } else {
                    reply.body
                };
                let length = if reply.stall || reply.truncate {
                    body.len() + 1000
                } else {
                    body.len()
                };
                write!(stream, "HTTP/1.1 {} OK\r\nContent-Type: application/json\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n", reply.status).unwrap();
                stream.write_all(&body).unwrap();
                stream.flush().unwrap();
                sender.send(request).unwrap();
                if reply.stall {
                    let _ = stream.read(&mut [0]);
                }
            }
        });
        Self {
            url,
            requests,
            task,
        }
    }

    pub fn finish(self) -> Vec<String> {
        self.task.join().unwrap();
        self.requests.try_iter().collect()
    }
}

fn accept(listener: &TcpListener) -> TcpStream {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(10)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(10)))
                    .unwrap();
                return stream;
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(Instant::now() < deadline, "test server timed out");
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("accept: {error}"),
        }
    }
}

fn read_request(stream: &mut impl Read) -> String {
    let mut reader = BufReader::new(stream);
    let mut request = String::new();
    let mut content_length = 0;
    loop {
        let mut line = String::new();
        assert_ne!(
            reader.read_line(&mut line).unwrap(),
            0,
            "missing HTTP headers"
        );
        if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            content_length = value.trim().parse().unwrap();
        }
        request.push_str(&line);
        if line == "\r\n" {
            break;
        }
    }
    let mut body = vec![0; content_length];
    reader.read_exact(&mut body).unwrap();
    request.push_str(std::str::from_utf8(&body).unwrap());
    request
}

pub fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
