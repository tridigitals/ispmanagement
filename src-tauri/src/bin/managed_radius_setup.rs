use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use rand::{distributions::Alphanumeric, Rng};
use saas_tauri_lib::services::ManagedRadiusService;
use sqlx::PgPool;
use std::env;
use uuid::Uuid;

#[derive(Debug)]
struct Options {
    tenant_id: String,
    router_id: String,
    server_name: String,
    db_host: String,
    db_port: i32,
    db_name: String,
    db_user: String,
    db_password: String,
    nas_ip_or_cidr: Option<String>,
    nas_name: Option<String>,
    shortname: Option<String>,
    shared_secret: Option<String>,
}

fn build_database_url_from_env() -> Result<String> {
    let user = env::var("POSTGRES_USER").map_err(|_| anyhow!("Missing POSTGRES_USER"))?;
    let password =
        env::var("POSTGRES_PASSWORD").map_err(|_| anyhow!("Missing POSTGRES_PASSWORD"))?;
    let db = env::var("POSTGRES_DB").map_err(|_| anyhow!("Missing POSTGRES_DB"))?;
    let host = env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port = env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string());
    let sslmode = env::var("POSTGRES_SSLMODE").unwrap_or_else(|_| "disable".to_string());
    Ok(format!(
        "postgres://{}:{}@{}:{}/{}?sslmode={}",
        user, password, host, port, db, sslmode
    ))
}

fn parse_args() -> Result<Options> {
    let mut args = env::args().skip(1);

    let mut tenant_id = None;
    let mut router_id = None;
    let mut server_name = None;
    let mut db_host = None;
    let mut db_port = Some(55433);
    let mut db_name = Some("radius".to_string());
    let mut db_user = Some("radius".to_string());
    let mut db_password = None;
    let mut nas_ip_or_cidr = None;
    let mut nas_name = None;
    let mut shortname = None;
    let mut shared_secret = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--tenant-id" => tenant_id = args.next(),
            "--router-id" => router_id = args.next(),
            "--server-name" => server_name = args.next(),
            "--db-host" => db_host = args.next(),
            "--db-port" => {
                db_port = Some(
                    args.next()
                        .ok_or_else(|| anyhow!("Missing value for --db-port"))?
                        .parse()
                        .context("Invalid --db-port")?,
                )
            }
            "--db-name" => db_name = args.next(),
            "--db-user" => db_user = args.next(),
            "--db-password" => db_password = args.next(),
            "--nas-ip" => nas_ip_or_cidr = args.next(),
            "--nas-name" => nas_name = args.next(),
            "--shortname" => shortname = args.next(),
            "--shared-secret" => shared_secret = args.next(),
            other => return Err(anyhow!("Unknown argument: {other}")),
        }
    }

    Ok(Options {
        tenant_id: tenant_id.ok_or_else(|| anyhow!("Missing --tenant-id"))?,
        router_id: router_id.ok_or_else(|| anyhow!("Missing --router-id"))?,
        server_name: server_name.unwrap_or_else(|| "Managed RADIUS".to_string()),
        db_host: db_host.unwrap_or_else(|| "localhost".to_string()),
        db_port: db_port.unwrap_or(55433),
        db_name: db_name.unwrap_or_else(|| "radius".to_string()),
        db_user: db_user.unwrap_or_else(|| "radius".to_string()),
        db_password: db_password.ok_or_else(|| anyhow!("Missing --db-password"))?,
        nas_ip_or_cidr,
        nas_name,
        shortname,
        shared_secret,
    })
}

#[derive(Debug, sqlx::FromRow)]
struct RouterRow {
    id: String,
    name: String,
    host: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let opts = parse_args()?;
    let billing_db_url = env::var("DATABASE_URL").unwrap_or(build_database_url_from_env()?);
    let pool = PgPool::connect(&billing_db_url).await?;

    let router = sqlx::query_as::<_, RouterRow>(
        "SELECT id, name, host FROM mikrotik_routers WHERE id = $1 AND tenant_id = $2",
    )
    .bind(&opts.router_id)
    .bind(&opts.tenant_id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| anyhow!("Router not found for tenant"))?;

    let db_password_enc = ManagedRadiusService::encrypt_db_password(&opts.db_password)?;
    let generated_secret = opts.shared_secret.clone().unwrap_or_else(|| {
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect::<String>()
    });
    let shared_secret_enc = ManagedRadiusService::encrypt_shared_secret(&generated_secret)?;
    let now = Utc::now();

    let server_id: String = sqlx::query_scalar(
        r#"
        INSERT INTO managed_radius_servers (
          id, tenant_id, name, db_host, db_port, db_name, db_user, db_password_enc, is_active, created_at, updated_at
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,true,$9,$10)
        ON CONFLICT (tenant_id, name) DO UPDATE SET
          db_host = EXCLUDED.db_host,
          db_port = EXCLUDED.db_port,
          db_name = EXCLUDED.db_name,
          db_user = EXCLUDED.db_user,
          db_password_enc = EXCLUDED.db_password_enc,
          is_active = true,
          updated_at = EXCLUDED.updated_at
        RETURNING id
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&opts.tenant_id)
    .bind(&opts.server_name)
    .bind(&opts.db_host)
    .bind(opts.db_port)
    .bind(&opts.db_name)
    .bind(&opts.db_user)
    .bind(&db_password_enc)
    .bind(now)
    .bind(now)
    .fetch_one(&pool)
    .await?;

    let nas_name = opts.nas_name.unwrap_or_else(|| router.name.clone());
    let nas_ip_or_cidr = opts.nas_ip_or_cidr.unwrap_or_else(|| router.host.clone());
    let shortname = opts.shortname.or_else(|| Some(router.name.clone()));

    let nas_id: String = sqlx::query_scalar(
        r#"
        INSERT INTO managed_radius_nas (
          id, tenant_id, router_id, radius_server_id, nas_name, nas_ip_or_cidr, shared_secret_enc, shortname, is_active, created_at, updated_at
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,true,$9,$10)
        ON CONFLICT (tenant_id, router_id) DO UPDATE SET
          radius_server_id = EXCLUDED.radius_server_id,
          nas_name = EXCLUDED.nas_name,
          nas_ip_or_cidr = EXCLUDED.nas_ip_or_cidr,
          shared_secret_enc = EXCLUDED.shared_secret_enc,
          shortname = EXCLUDED.shortname,
          is_active = true,
          updated_at = EXCLUDED.updated_at
        RETURNING id
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&opts.tenant_id)
    .bind(&router.id)
    .bind(&server_id)
    .bind(&nas_name)
    .bind(&nas_ip_or_cidr)
    .bind(&shared_secret_enc)
    .bind(&shortname)
    .bind(now)
    .bind(now)
    .fetch_one(&pool)
    .await?;

    println!("managed_radius_server_id={server_id}");
    println!("managed_radius_nas_id={nas_id}");
    println!("router_name={}", router.name);
    println!("nas_ip_or_cidr={nas_ip_or_cidr}");
    println!("shared_secret={generated_secret}");

    Ok(())
}
