#![recursion_limit = "512"]

use std::net::ToSocketAddrs;
use std::num::NonZeroUsize;
use std::sync::Arc;

use anyhow::Context as _;

use poprako_server::{
    AppConfig, AsyncEffectDevelop, Harn, HybNucl, HybRepo, JwtAuth, RdbContext,
    RdbCore, RdbNucl, ReptRead, Sched, Serial, new_obj_dept, new_prom,
};

/// Application entry point.
///
/// Parses CLI flags, loads configuration, initializes runtime dependencies
/// (database pool, authentication, object department, effect dispatcher), wires them
/// into an application harness, and launches the HTTP server. Pass `--swagger`
/// to print the `OpenAPI` spec to stdout instead of launching the server.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    //
    poprako_server::init_log();

    if let Err(err) = dotenvy::dotenv() {
        //
        tracing::warn!(
            operation = "load_dotenv",
            sdk_err = ?err,
            ".env loading failed; continuing with process environment",
        );
    }

    let config = AppConfig::from_default_file()
        .await
        .context("failed to load application configuration")?;

    let http_addr = ToSocketAddrs::to_socket_addrs(&format!(
        "{}:{}",
        config.http.host, config.http.port
    ))
    .into_iter()
    .find_map(|mut addrs| addrs.next())
    .context("no address resolved for HTTP listen address")?;

    let core = RdbCore::from_env()?;

    let nucl = HybNucl::new(
        RdbNucl::<ReptRead>::new(core.clone()),
        RdbNucl::<Serial>::new(core.clone()),
    );

    let repo = HybRepo::new(core.clone());

    let auth = JwtAuth::from_env()?;

    let obj_dept = new_obj_dept(core.clone())?;

    let sched = Sched::new(core.clone(), obj_dept.clone());

    let develop = AsyncEffectDevelop::new::<RdbContext<ReptRead>, _>(
        Arc::new(HybRepo::new(core.clone())),
        NonZeroUsize::new(1024).context("buf_size cannot be 0")?,
    );

    let prom = new_prom(core.clone(), obj_dept.view(), develop.clone());

    let harn = Harn::new(config, (nucl, repo, obj_dept, prom, auth, develop));

    let serve_rest = poprako_server::serve(harn.clone(), http_addr).await;

    sched.close().await;

    tokio::join!(
        harn.obj_dept().close(),
        harn.prom().close(),
        harn.develop().close(),
    );

    serve_rest
}
