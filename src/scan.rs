use crate::config::Config;
use crate::imap::{ImapClient, ScannedSender};

#[derive(Debug, serde::Serialize)]
pub struct ScanReport {
    pub account: String,
    pub senders: Vec<ScannedSender>,
    pub error: Option<String>,
}

pub fn run(config: &Config, account_filter: Option<&str>) -> Vec<ScanReport> {
    let mut reports = Vec::new();

    for (account_name, account) in &config.accounts {
        if let Some(filter) = account_filter
            && filter != account_name.as_str()
        {
            continue;
        }

        let password = match account.resolve_password() {
            Ok(p) => p,
            Err(e) => {
                log::error!("account '{account_name}': failed to resolve password: {e}");
                reports.push(ScanReport {
                    account: account_name.clone(),
                    senders: Vec::new(),
                    error: Some(format!("failed to resolve password: {e}")),
                });
                continue;
            }
        };

        let mut client = match ImapClient::connect(&account.server, &account.username, &password) {
            Ok(c) => c,
            Err(e) => {
                log::error!("account '{account_name}': connection failed: {e}");
                reports.push(ScanReport {
                    account: account_name.clone(),
                    senders: Vec::new(),
                    error: Some(format!("connection failed: {e}")),
                });
                continue;
            }
        };

        if let Err(e) = client.uid_validity(&account.mailbox) {
            log::error!(
                "account '{account_name}': failed to select mailbox '{}': {e}",
                account.mailbox
            );
            reports.push(ScanReport {
                account: account_name.clone(),
                senders: Vec::new(),
                error: Some(format!("failed to select mailbox: {e}")),
            });
            continue;
        }

        let mut senders = match client.scan_senders() {
            Ok(s) => s,
            Err(e) => {
                log::error!("account '{account_name}': scan failed: {e}");
                reports.push(ScanReport {
                    account: account_name.clone(),
                    senders: Vec::new(),
                    error: Some(format!("scan failed: {e}")),
                });
                continue;
            }
        };

        if let Err(e) = client.logout() {
            log::warn!("account '{account_name}': logout error: {e}");
        }

        senders.sort_by(|a, b| b.count.cmp(&a.count));

        reports.push(ScanReport {
            account: account_name.clone(),
            senders,
            error: None,
        });
    }

    reports
}
