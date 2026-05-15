use identity_service::infrastructure::{postgres, sqlite};

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

    let backend = migration_backend_from_env(&database_url);
    let result = match (backend, command) {
        (MigrationBackend::Postgres, MigrationCommand::Up) => {
            postgres::run_pending_migrations(&database_url)
                .await
                .map(|report| report.available_up_migrations)
        }
        (MigrationBackend::Postgres, MigrationCommand::Down { target_version }) => {
            postgres::revert_migrations(&database_url, target_version)
                .await
                .map(|report| report.available_up_migrations)
        }
        (MigrationBackend::Sqlite, MigrationCommand::Up) => {
            sqlite::run_pending_migrations(&database_url)
                .await
                .map(|report| report.available_up_migrations)
        }
        (MigrationBackend::Sqlite, MigrationCommand::Down { target_version }) => {
            sqlite::revert_migrations(&database_url, target_version)
                .await
                .map(|report| report.available_up_migrations)
        }
    };

    match result {
        Ok(available_up_migrations) => {
            println!(
                "database migrations completed; available up migrations: {}",
                available_up_migrations
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

enum MigrationBackend {
    Postgres,
    Sqlite,
}

fn migration_backend_from_env(database_url: &str) -> MigrationBackend {
    match std::env::var("IDENTITY_PERSISTENCE_BACKEND") {
        Ok(value) if value == "postgres" => MigrationBackend::Postgres,
        Ok(value) if value == "sqlite" => MigrationBackend::Sqlite,
        Ok(value) if value == "memory" => {
            exit_with_config_error("memory backend does not use database migrations")
        }
        Ok(value) => exit_with_config_error(&format!(
            "IDENTITY_PERSISTENCE_BACKEND must be one of: sqlite, postgres; got {value}"
        )),
        Err(_) if database_url.starts_with("sqlite:") => MigrationBackend::Sqlite,
        Err(_) => MigrationBackend::Postgres,
    }
}

fn exit_with_config_error(message: &str) -> ! {
    eprintln!("configuration error: {message}");
    std::process::exit(1);
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
