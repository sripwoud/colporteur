use std::collections::HashMap;

use chrono::{DateTime, Utc};
use eyre::{Context, eyre};
use imap::{ClientBuilder, Connection, Session};
use mailparse::{MailAddr, MailHeaderMap, addrparse_header, parse_headers};

pub trait EmailSource {
    fn uid_validity(&mut self, mailbox: &str) -> eyre::Result<u32>;
    fn search_from_since_uid(&mut self, sender: &str, last_uid: u32) -> eyre::Result<Vec<u32>>;
    fn fetch_email(&mut self, uid: u32) -> eyre::Result<FetchedEmail>;
}

pub struct ImapClient {
    session: Session<Connection>,
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
