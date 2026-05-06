use crate::config::Config;
use crate::imap::{AccountOpenError, AccountSession, ScannedSender};

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

        let mut session = match AccountSession::open(account_name, account) {
            Ok(s) => s,
            Err(AccountOpenError::PasswordResolution(e)) => {
                log::error!("account '{account_name}': failed to resolve password");
                log::debug!("account '{account_name}': {e}");
                reports.push(ScanReport {
                    account: account_name.clone(),
                    senders: Vec::new(),
                    error: Some(
                        "failed to resolve password; re-run with -vv for details".to_string(),
                    ),
                });
                continue;
            }
            Err(AccountOpenError::Connection(e)) => {
                log::error!("account '{account_name}': connection failed: {e}");
                reports.push(ScanReport {
                    account: account_name.clone(),
                    senders: Vec::new(),
                    error: Some(format!("connection failed: {e}")),
                });
                continue;
            }
        };

        if let Err(e) = session.client_mut().uid_validity(&account.mailbox) {
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

        let mut senders = match session.client_mut().scan_senders() {
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

        senders.sort_by(|a, b| b.count.cmp(&a.count));

        reports.push(ScanReport {
            account: account_name.clone(),
            senders,
            error: None,
        });
    }

    reports
}
