use clap::Parser;
use colporteur::cli::{Cli, Command, FetchArgs, TestArgs};
use colporteur::config::Config;
use colporteur::fetch;
use colporteur::state::AppState;

fn main() {
    let cli = Cli::parse();
    init_logger(cli.verbose, cli.quiet);

    let config = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e:#}");
            std::process::exit(3);
        }
    };

    let exit_code = match cli.command {
        Command::Fetch(args) => cmd_fetch(&config, &args, cli.json, cli.quiet),
        Command::Test(args) => cmd_test(&config, &args, cli.json),
        Command::List => cmd_list(&config, cli.json),
    };

    std::process::exit(exit_code);
}

fn init_logger(verbose: u8, quiet: bool) {
    if std::env::var("RUST_LOG").is_ok() {
        env_logger::init();
        return;
    }

    let level = if quiet {
        log::LevelFilter::Error
    } else {
        match verbose {
            0 => log::LevelFilter::Warn,
            1 => log::LevelFilter::Info,
            _ => log::LevelFilter::Debug,
        }
    };

    env_logger::Builder::new().filter_level(level).init();
}

fn cmd_fetch(config: &Config, args: &FetchArgs, json: bool, quiet: bool) -> i32 {
    let state_path = match AppState::default_path() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e:#}");
            return 1;
        }
    };

    let mut state = match AppState::load(&state_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e:#}");
            return 1;
        }
    };

    let filtered_config;
    let effective_config = if let Some(feed_key) = &args.feed {
        if !config.feeds.contains_key(feed_key.as_str()) {
            eprintln!("error: feed '{feed_key}' not found in config");
            return 1;
        }
        filtered_config = colporteur::config::Config {
            output_dir: config.output_dir.clone(),
            max_entries: config.max_entries,
            accounts: config.accounts.clone(),
            feeds: config
                .feeds
                .iter()
                .filter(|(k, _)| k.as_str() == feed_key.as_str())
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        };
        &filtered_config
    } else {
        config
    };

    let report = match fetch::run(effective_config, &mut state, &state_path, args.dry_run) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e:#}");
            return 1;
        }
    };

    if json {
        match serde_json::to_string_pretty(&report) {
            Ok(s) => println!("{s}"),
            Err(e) => eprintln!("error serializing JSON: {e}"),
        }
    } else if !quiet {
        println!("fetching feeds...");
        for feed in &report.feeds {
            if feed.ok {
                if let Some(output) = &feed.output {
                    println!(
                        "  {:<20} {} new  ->  {}",
                        feed.key, feed.new_entries, output
                    );
                } else {
                    println!("  {:<20} {} new  (skipped)", feed.key, feed.new_entries);
                }
            } else {
                let err = feed.error.as_deref().unwrap_or("unknown error");
                eprintln!("  {:<20} FAILED: {}", feed.key, err);
            }
        }
        println!("done. {} entries written.", report.total_new);
    }

    let failed = report.feeds.iter().filter(|f| !f.ok).count();
    let total = report.feeds.len();

    if failed == 0 {
        0
    } else if failed == total {
        1
    } else {
        4
    }
}

fn cmd_test(config: &Config, args: &TestArgs, json: bool) -> i32 {
    let results = fetch::test_connections(config, args.account.as_deref());

    if json {
        let json_results: Vec<serde_json::Value> = results
            .iter()
            .map(|(name, result)| {
                serde_json::json!({
                    "account": name,
                    "ok": result.is_ok(),
                    "error": result.as_ref().err().map(|e| format!("{e:#}")),
                })
            })
            .collect();
        match serde_json::to_string_pretty(&json_results) {
            Ok(s) => println!("{s}"),
            Err(e) => eprintln!("error serializing JSON: {e}"),
        }
    } else {
        println!("testing accounts...");
        for (name, result) in &results {
            let account = config.accounts.get(name.as_str());
            let server = account.map(|a| a.server.as_str()).unwrap_or("unknown");
            match result {
                Ok(()) => println!("  {:<12} {:<30} ok", name, server),
                Err(e) => eprintln!("  {:<12} {:<30} FAILED: {e:#}", name, server),
            }
        }
    }

    let any_failed = results.iter().any(|(_, r)| r.is_err());
    if any_failed { 5 } else { 0 }
}

fn cmd_list(config: &Config, json: bool) -> i32 {
    let state_path = match AppState::default_path() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e:#}");
            return 1;
        }
    };

    let state = match AppState::load(&state_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e:#}");
            return 1;
        }
    };

    if json {
        let entries: Vec<serde_json::Value> = config
            .feeds
            .iter()
            .map(|(key, feed)| {
                let last_uid = feed
                    .senders
                    .first()
                    .map(|s| state.last_uid(&feed.account, s))
                    .unwrap_or(0);
                serde_json::json!({
                    "feed": key,
                    "account": feed.account,
                    "senders": feed.senders,
                    "last_uid": last_uid,
                })
            })
            .collect();
        match serde_json::to_string_pretty(&entries) {
            Ok(s) => println!("{s}"),
            Err(e) => eprintln!("error serializing JSON: {e}"),
        }
    } else {
        println!(
            "{:<20} {:<12} {:<36} LAST UID",
            "FEED", "ACCOUNT", "SENDERS"
        );
        let mut keys: Vec<&String> = config.feeds.keys().collect();
        keys.sort();
        for key in keys {
            let feed = &config.feeds[key];
            let senders_display = feed.senders.join(", ");
            let senders_truncated = if senders_display.len() > 36 {
                format!("{}...", &senders_display[..33])
            } else {
                senders_display
            };
            let last_uid = feed
                .senders
                .first()
                .map(|s| state.last_uid(&feed.account, s))
                .unwrap_or(0);
            println!(
                "{:<20} {:<12} {:<36} {}",
                key, feed.account, senders_truncated, last_uid
            );
        }
    }

    0
}
