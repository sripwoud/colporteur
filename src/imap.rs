use std::collections::HashMap;

use chrono::{DateTime, Utc};
use eyre::{Context, eyre};
use imap::{ClientBuilder, Connection, Session};
use mailparse::{MailAddr, MailHeaderMap, addrparse_header, parse_headers};

use crate::config::AccountConfig;

pub trait EmailSource {
    fn uid_validity(&mut self, mailbox: &str) -> eyre::Result<u32>;
    fn search_from_since_uid(&mut self, sender: &str, last_uid: u32) -> eyre::Result<Vec<u32>>;
    fn fetch_email(&mut self, uid: u32) -> eyre::Result<FetchedEmail>;
}

pub struct ImapClient {
    session: Session<Connection>,
}

#[derive(Debug)]
pub enum AccountOpenError {
    PasswordResolution(eyre::Error),
    Connection(eyre::Error),
}

pub struct AccountSession {
    name: String,
    client: ImapClient,
}

impl AccountSession {
    pub fn open(name: &str, account: &AccountConfig) -> Result<Self, AccountOpenError> {
        let password = account
            .resolve_password()
            .map_err(AccountOpenError::PasswordResolution)?;

        let client = ImapClient::connect(&account.server, &account.username, &password)
            .map_err(AccountOpenError::Connection)?;

        Ok(Self {
            name: name.to_string(),
            client,
        })
    }

    pub fn client_mut(&mut self) -> &mut ImapClient {
        &mut self.client
    }
}

impl Drop for AccountSession {
    fn drop(&mut self) {
        if let Err(e) = self.client.logout() {
            log::warn!("account '{}': logout error: {e}", self.name);
        }
    }
}

pub struct FetchedEmail {
    pub uid: u32,
    pub raw: Vec<u8>,
}

#[derive(Debug, serde::Serialize)]
pub struct ScannedSender {
    pub address: String,
    pub name: Option<String>,
    pub count: usize,
    pub latest: DateTime<Utc>,
}

impl ImapClient {
    pub fn connect(server: &str, username: &str, password: &str) -> eyre::Result<Self> {
        let client = ClientBuilder::new(server, 993)
            .connect()
            .wrap_err_with(|| format!("failed to connect to {server}"))?;

        let session = client
            .login(username, password)
            .map_err(|(err, _)| err)
            .wrap_err("IMAP login failed")?;

        Ok(Self { session })
    }

    pub fn uid_validity(&mut self, mailbox: &str) -> eyre::Result<u32> {
        let mailbox = self
            .session
            .select(mailbox)
            .wrap_err_with(|| format!("failed to SELECT {mailbox}"))?;
        mailbox
            .uid_validity
            .ok_or_else(|| eyre!("server did not return UIDVALIDITY"))
    }

    pub fn search_from_since_uid(&mut self, sender: &str, last_uid: u32) -> eyre::Result<Vec<u32>> {
        let start = last_uid.max(1);
        let query = format!("FROM \"{sender}\" UID {start}:*");
        let uids = match self.session.uid_search(&query) {
            Ok(uids) => uids,
            Err(imap::error::Error::No(no)) => {
                log::debug!("sender '{sender}': search returned NO: {no}");
                return Ok(Vec::new());
            }
            Err(e) => return Err(e).wrap_err("IMAP UID SEARCH failed"),
        };
        let mut uids: Vec<u32> = uids.into_iter().filter(|&uid| uid > last_uid).collect();
        uids.sort();
        Ok(uids)
    }

    pub fn fetch_email(&mut self, uid: u32) -> eyre::Result<FetchedEmail> {
        let fetches = self
            .session
            .uid_fetch(uid.to_string(), "RFC822")
            .wrap_err_with(|| format!("failed to FETCH UID {uid}"))?;

        let fetch = fetches
            .get(0)
            .ok_or_else(|| eyre!("no message returned for UID {uid}"))?;

        let body = fetch.body().ok_or_else(|| eyre!("UID {uid} has no body"))?;

        Ok(FetchedEmail {
            uid,
            raw: body.to_vec(),
        })
    }

    pub fn move_to_trash(&mut self, uids: &[u32], trash_folder: &str) -> eyre::Result<()> {
        if uids.is_empty() {
            return Ok(());
        }
        let uid_set = uids
            .iter()
            .map(|u| u.to_string())
            .collect::<Vec<_>>()
            .join(",");

        self.session
            .uid_mv(&uid_set, trash_folder)
            .wrap_err_with(|| format!("failed to move UIDs to {trash_folder}"))?;

        Ok(())
    }

    pub fn test_connection(server: &str, username: &str, password: &str) -> eyre::Result<()> {
        let mut client = Self::connect(server, username, password)?;
        client.session.logout().wrap_err("IMAP logout failed")?;
        Ok(())
    }

    pub fn logout(&mut self) -> eyre::Result<()> {
        self.session.logout().wrap_err("IMAP logout failed")?;
        Ok(())
    }

    pub fn scan_senders(&mut self) -> eyre::Result<Vec<ScannedSender>> {
        let uids = self
            .session
            .uid_search("ALL")
            .wrap_err("IMAP UID SEARCH ALL failed")?;

        if uids.is_empty() {
            return Ok(Vec::new());
        }

        let uid_set = "1:*";
        let fetches = self
            .session
            .uid_fetch(uid_set, "BODY.PEEK[HEADER.FIELDS (FROM DATE)]")
            .wrap_err("IMAP UID FETCH header fields failed")?;

        let mut aggregated: HashMap<String, ScannedSender> = HashMap::new();

        for fetch in fetches.iter() {
            let raw = match fetch.header() {
                Some(h) => h,
                None => {
                    log::warn!("scan: UID {:?} has no header data, skipping", fetch.uid);
                    continue;
                }
            };

            let (headers, _) = match parse_headers(raw) {
                Ok(h) => h,
                Err(e) => {
                    log::warn!("scan: failed to parse headers for UID {:?}: {e}", fetch.uid);
                    continue;
                }
            };

            let from_header = match headers.get_first_header("From") {
                Some(h) => h,
                None => {
                    log::warn!("scan: UID {:?} has no From header, skipping", fetch.uid);
                    continue;
                }
            };

            let addrs = match addrparse_header(from_header) {
                Ok(a) => a,
                Err(e) => {
                    log::warn!(
                        "scan: failed to parse From header for UID {:?}: {e}",
                        fetch.uid
                    );
                    continue;
                }
            };

            let single = match addrs.into_inner().into_iter().find_map(|a| match a {
                MailAddr::Single(s) => Some(s),
                MailAddr::Group(g) => g.addrs.into_iter().next(),
            }) {
                Some(s) => s,
                None => {
                    log::warn!(
                        "scan: no usable address in From header for UID {:?}, skipping",
                        fetch.uid
                    );
                    continue;
                }
            };

            let date_str = match headers.get_first_value("Date") {
                Some(v) => v,
                None => {
                    log::warn!("scan: UID {:?} has no Date header, skipping", fetch.uid);
                    continue;
                }
            };

            let timestamp = match mailparse::dateparse(&date_str) {
                Ok(ts) => ts,
                Err(e) => {
                    log::warn!(
                        "scan: failed to parse Date header for UID {:?}: {e}",
                        fetch.uid
                    );
                    continue;
                }
            };

            let date = match DateTime::from_timestamp(timestamp, 0) {
                Some(d) => d,
                None => {
                    log::warn!(
                        "scan: timestamp out of range for UID {:?}: {timestamp}",
                        fetch.uid
                    );
                    continue;
                }
            };

            let addr_lower = single.addr.to_lowercase();
            let entry = aggregated
                .entry(addr_lower.clone())
                .or_insert_with(|| ScannedSender {
                    address: addr_lower,
                    name: single.display_name.clone(),
                    count: 0,
                    latest: date,
                });

            entry.count += 1;
            if date > entry.latest {
                entry.latest = date;
                if single.display_name.is_some() {
                    entry.name = single.display_name;
                }
            }
        }

        Ok(aggregated.into_values().collect())
    }
}

impl EmailSource for ImapClient {
    fn uid_validity(&mut self, mailbox: &str) -> eyre::Result<u32> {
        self.uid_validity(mailbox)
    }

    fn search_from_since_uid(&mut self, sender: &str, last_uid: u32) -> eyre::Result<Vec<u32>> {
        self.search_from_since_uid(sender, last_uid)
    }

    fn fetch_email(&mut self, uid: u32) -> eyre::Result<FetchedEmail> {
        self.fetch_email(uid)
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    use imap::ConnectionMode;

    use super::*;
    use crate::config::AccountConfig;

    fn account_config(server: &str, password: &str) -> AccountConfig {
        AccountConfig {
            server: server.to_string(),
            username: "user@test.com".to_string(),
            password: password.to_string(),
            mailbox: "INBOX".to_string(),
        }
    }

    /// Minimal mock IMAP server that handles LOGIN and LOGOUT over plaintext TCP.
    /// Returns the port it's listening on and a receiver that fires once when LOGOUT is observed.
    fn spawn_mock_imap_server() -> (u16, mpsc::Receiver<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind mock server");
        let port = listener.local_addr().unwrap().port();
        let (logout_tx, logout_rx) = mpsc::channel();

        thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                let mut writer = stream.try_clone().unwrap();
                let reader = BufReader::new(stream);

                writer
                    .write_all(b"* OK IMAP4rev1 mock server ready\r\n")
                    .ok();

                for line in reader.lines() {
                    let line = match line {
                        Ok(l) => l,
                        Err(_) => break,
                    };

                    let parts: Vec<&str> = line.splitn(3, ' ').collect();
                    if parts.len() < 2 {
                        break;
                    }
                    let tag = parts[0];
                    let cmd = parts[1].to_uppercase();

                    match cmd.as_str() {
                        "LOGIN" => {
                            writer
                                .write_all(format!("{tag} OK LOGIN completed\r\n").as_bytes())
                                .ok();
                        }
                        "LOGOUT" => {
                            writer.write_all(b"* BYE Logging out\r\n").ok();
                            writer
                                .write_all(format!("{tag} OK LOGOUT completed\r\n").as_bytes())
                                .ok();
                            let _ = logout_tx.send(());
                            break;
                        }
                        _ => {
                            writer
                                .write_all(format!("{tag} BAD unknown command\r\n").as_bytes())
                                .ok();
                        }
                    }
                }
            }
        });

        (port, logout_rx)
    }

    fn connect_to_mock(port: u16) -> ImapClient {
        let client = ClientBuilder::new("127.0.0.1", port)
            .mode(ConnectionMode::Plaintext)
            .connect()
            .expect("failed to connect to mock server");
        let session = client
            .login("user@test.com", "password")
            .map_err(|(e, _)| e)
            .expect("mock login failed");
        ImapClient { session }
    }

    #[test]
    fn open_success_and_drop_calls_logout() {
        let (port, logout_rx) = spawn_mock_imap_server();

        let client = connect_to_mock(port);
        let session = AccountSession {
            name: "test-account".to_string(),
            client,
        };

        drop(session);

        logout_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("expected LOGOUT within 2s");
    }

    #[test]
    #[cfg(unix)]
    fn open_password_resolution_failure() {
        // "!false" is a shell command that exits with code 1 — resolution must fail
        let account = account_config("localhost", "!false");
        let result = AccountSession::open("test", &account);
        assert!(
            matches!(result, Err(AccountOpenError::PasswordResolution(_))),
            "expected PasswordResolution error"
        );
    }

    #[test]
    fn open_connection_failure() {
        // Use a hostname that does not exist — DNS lookup will fail fast
        let account = account_config("this.hostname.does.not.exist.colporteur.test", "password");
        let result = AccountSession::open("test", &account);
        assert!(
            matches!(result, Err(AccountOpenError::Connection(_))),
            "expected Connection error"
        );
    }
}
