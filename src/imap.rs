use eyre::{Context, eyre};
use imap::{ClientBuilder, Connection, Session};

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
        let query = format!("FROM \"{sender}\" UID {last_uid}:*");
        let uids = self
            .session
            .uid_search(&query)
            .wrap_err("IMAP UID SEARCH failed")?;
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
