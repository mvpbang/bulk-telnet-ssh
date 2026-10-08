use crate::error::AppError;
use ssh2::Session;
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone)]
pub struct SshClient {
    pub host: String,
    pub port: u16,
    pub addr: String,
    session: Arc<Mutex<Session>>,
}

impl SshClient {
    pub fn connect(
        host: &str,
        port: u16,
        user: &str,
        password: &str,
        timeout: Duration,
        max_retries: usize,
    ) -> Result<Self, AppError> {
        let addr = format!("{}:{}", host, port);
        let mut last_err = None;

        for attempt in 1..=max_retries {
            match Self::try_connect(host, port, user, password, timeout) {
                Ok(client) => return Ok(client),
                Err(err) => {
                    let err_msg = err.to_string();
                    let is_retryable = err_msg.to_lowercase().contains("handshake")
                        || err_msg.to_lowercase().contains("reset")
                        || err_msg.to_lowercase().contains("timed out")
                        || err_msg.to_lowercase().contains("timeout")
                        || err_msg.to_lowercase().contains("broken pipe");

                    eprintln!(
                        "SSH connect to {} attempt {}/{} failed: {}",
                        addr, attempt, max_retries, err_msg
                    );

                    last_err = Some(err);
                    if !is_retryable || attempt == max_retries {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(500 * attempt as u64));
                }
            }
        }

        Err(last_err.unwrap_or_else(|| AppError::Ssh("Unknown connection error".to_string())))
    }

    fn try_connect(
        host: &str,
        port: u16,
        user: &str,
        password: &str,
        timeout: Duration,
    ) -> Result<Self, AppError> {
        let addr = format!("{}:{}", host, port);

        // DNS 解析与连接
        let sock_addr = addr
            .to_socket_addrs()
            .map_err(AppError::Io)?
            .next()
            .ok_or_else(|| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Failed to resolve address: {}", addr),
                ))
            })?;

        let tcp = TcpStream::connect_timeout(&sock_addr, timeout).map_err(AppError::Io)?;
        tcp.set_read_timeout(Some(timeout)).map_err(AppError::Io)?;
        tcp.set_write_timeout(Some(timeout)).map_err(AppError::Io)?;

        let mut sess = Session::new().map_err(|e| AppError::Ssh(e.to_string()))?;
        sess.set_timeout(timeout.as_millis() as u32);
        sess.set_tcp_stream(tcp);

        sess.handshake().map_err(|e| AppError::Ssh(e.to_string()))?;

        sess.userauth_password(user, password)
            .map_err(|e| AppError::Ssh(e.to_string()))?;

        if !sess.authenticated() {
            return Err(AppError::Ssh("Authentication failed: invalid username or password".to_string()));
        }

        Ok(Self {
            host: host.to_string(),
            port,
            addr,
            session: Arc::new(Mutex::new(sess)),
        })
    }

    pub fn with_session<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Session) -> R,
    {
        let sess = self.session.lock().expect("session mutex poisoned");
        f(&sess)
    }

    pub fn with_session_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Session) -> R,
    {
        let mut sess = self.session.lock().expect("session mutex poisoned");
        f(&mut sess)
    }
}
