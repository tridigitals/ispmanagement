use saas_tauri_lib::db::init_db;
use saas_tauri_lib::db::{
    run_seed, seed_defaults, seed_plans, seed_roles, DbPool, SeedMode, SeedOptions,
};
use std::env;

fn parse_args() -> SeedOptions {
    let mut opts = SeedOptions::default();

    // Usage:
    //   seed [dev|prod] [--email x] [--password y] [--name z] [--tenant-name n] [--tenant-slug s] [--tz Asia/Jakarta]
    let argv: Vec<String> = env::args().skip(1).collect();
    let mut i = 0usize;
    if let Some(first) = argv.get(0) {
        if first == "dev" {
            opts.mode = SeedMode::Dev;
            i = 1;
        } else if first == "prod" {
            opts.mode = SeedMode::Prod;
            i = 1;
        }
    }

    let mut it = argv.into_iter().skip(i).peekable();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--email" => {
                if let Some(v) = it.next() {
                    opts.admin_email = v;
                }
            }
            "--password" => {
                if let Some(v) = it.next() {
                    opts.admin_password = v;
                }
            }
            "--name" => {
                if let Some(v) = it.next() {
                    opts.admin_name = v;
                }
            }
            "--tenant-name" => {
                if let Some(v) = it.next() {
                    opts.tenant_name = v;
                    opts.tenant_slug = saas_tauri_lib::db::slugify(&opts.tenant_name);
                }
            }
            "--tenant-slug" => {
                if let Some(v) = it.next() {
                    opts.tenant_slug = v;
                }
            }
            "--tz" | "--timezone" => {
                if let Some(v) = it.next() {
                    opts.app_timezone = v;
                }
            }
            _ => {}
        }
    }

    opts
}

async fn ensure_core_seed(pool: &DbPool) -> anyhow::Result<()> {
    // init_db() already runs these, but seed can be executed independently in the future
    // if you ever want to call it against an already-migrated pool.
    let _ = seed_defaults(pool).await;
    let _ = seed_roles(pool).await;
    let _ = seed_plans(pool).await;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let opts = parse_args();

    // init_db runs migrations + core seeds. temp_dir is fine for postgres mode.
    let pool = init_db(std::env::temp_dir()).await?;
    ensure_core_seed(&pool).await?;

    run_seed(&pool, opts).await?;

    Ok(())
}
