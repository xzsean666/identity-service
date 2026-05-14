use identity_service::infrastructure::postgres::{revert_migrations, run_pending_migrations};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let command = MigrationCommand::from_args(std::env::args().skip(1).collect());
    let database_url = match std::env::var("IDENTITY_DATABASE_URL") {
        Ok(database_url) => database_url,
        Err(_) => {
            eprintln!(
                "configuration error: missing required environment variable IDENTITY_DATABASE_URL"
            );
            std::process::exit(1);
        }
    };

    let result = match command {
        MigrationCommand::Up => run_pending_migrations(&database_url).await,
        MigrationCommand::Down { target_version } => {
            revert_migrations(&database_url, target_version).await
        }
    };

    match result {
        Ok(report) => {
            println!(
                "database migrations completed; available up migrations: {}",
                report.available_up_migrations
            );
        }
        Err(error) => {
            eprintln!("database migration failed: {error}");
            std::process::exit(1);
        }
    }
}

enum MigrationCommand {
    Up,
    Down { target_version: i64 },
}

impl MigrationCommand {
    fn from_args(args: Vec<String>) -> Self {
        match args.as_slice() {
            [] => Self::Up,
            [command] if command == "up" => Self::Up,
            [command] if command == "down" => Self::Down { target_version: 0 },
            [command, target_version] if command == "down" => Self::Down {
                target_version: parse_target_version(target_version),
            },
            _ => {
                eprintln!("usage: cargo run --bin migrate -- [up|down <target_version>]");
                std::process::exit(1);
            }
        }
    }
}

fn parse_target_version(value: &str) -> i64 {
    match value.parse::<i64>() {
        Ok(version) if version >= 0 => version,
        _ => {
            eprintln!("migration target_version must be a non-negative integer");
            std::process::exit(1);
        }
    }
}
