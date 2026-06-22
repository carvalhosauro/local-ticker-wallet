use clap::{Parser, Subcommand};
use local_ticker_wallet::client;
use local_ticker_wallet::config::Config;
use local_ticker_wallet::ipc::{Action, Request, Response};

#[derive(Parser)]
#[command(name = "ltw")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    Daemon,
    Tui,
    Add {
        symbol: String,
        quantity: String,
        price: String,
        #[arg(long, default_value = "BUY")]
        side: String,
        #[arg(long, default_value = "0")]
        fees: String,
        #[arg(long)]
        date: String,
        #[arg(long)]
        note: Option<String>,
    },
    List {
        #[arg(long)]
        symbol: Option<String>,
    },
    Delete {
        id: i64,
    },
    Refresh {
        #[arg(long)]
        symbol: Option<String>,
    },
    Search {
        query: String,
    },
    Import {
        path: String,
    },
    Export {
        path: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Daemon => local_ticker_wallet::daemon::run(Config::load()?).await,
        Cmd::Tui => local_ticker_wallet::tui::run().await,
        cmd => {
            let req = to_request(cmd);
            let resp = client::send(req).await?;
            print_response(resp);
            Ok(())
        }
    }
}

fn to_request(cmd: Cmd) -> Request {
    match cmd {
        Cmd::Add {
            symbol,
            quantity,
            price,
            side,
            fees,
            date,
            note,
        } => Request::new(
            Action::AddTransaction,
            serde_json::json!({"symbol": symbol, "quantity": quantity, "price": price, "side": side, "fees": fees, "executed_at": date, "note": note}),
        ),
        Cmd::List { symbol } => Request::new(
            Action::ListTransactions,
            serde_json::json!({"symbol": symbol}),
        ),
        Cmd::Delete { id } => Request::new(
            Action::DeleteTransaction,
            serde_json::json!({"id": id}),
        ),
        Cmd::Refresh { symbol } => Request::new(
            Action::RefreshNow,
            serde_json::json!({"symbol": symbol}),
        ),
        Cmd::Search { query } => {
            Request::new(Action::Search, serde_json::json!({"query": query}))
        }
        Cmd::Import { path } => {
            Request::new(Action::Import, serde_json::json!({"path": path}))
        }
        Cmd::Export { path } => {
            Request::new(Action::Export, serde_json::json!({"path": path}))
        }
        Cmd::Daemon | Cmd::Tui => unreachable!(),
    }
}

fn print_response(resp: Response) {
    match resp {
        Response::Ok { data, .. } => {
            println!("{}", serde_json::to_string_pretty(&data).unwrap())
        }
        Response::Error { error, .. } => {
            eprintln!("error [{:?}]: {}", error.code, error.message)
        }
    }
}
