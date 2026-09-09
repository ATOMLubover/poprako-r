#![recursion_limit = "512"]

use std::net::ToSocketAddrs;
use std::num::NonZeroUsize;

use anyhow::Context as _;

use poprako_obj_dept::actor::ObjDeptActor;
use poprako_server::{
    AppConfig, AsyncEffectDevelop, EffectActor, Harn, HybNucl, HybRepo,
    JwtAuth, NormObjDept, R2ObjDeptPool, RdbContext, RdbCore, RdbNucl,
    RdbObjDeptProm, RdbProm, RdbPromActor, RdbPromRepo, ReptRead, Sched,
    Serial,
};

/// Application entry point.
///
/// Constructs application ports, injects background dependencies, starts actors
/// explicitly, and serves HTTP. Runtime descriptors remain owned here and are
/// joined in dependency order on both normal exit and HTTP startup failure.
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

    let (
        config,
        rdb_core,
        auth,
        obj_dept_pool,
        prom,
        prom_repo,
        (develop, effect_recv),
    ) = (
        AppConfig::from_default_file()
            .await
            .context("failed to load application configuration")?,
        RdbCore::from_env()?,
        JwtAuth::from_env()?,
        R2ObjDeptPool::from_env()?,
        RdbProm::new(),
        RdbPromRepo::new(),
        AsyncEffectDevelop::new(
            NonZeroUsize::new(1024).context("buf_size cannot be 0")?,
        ),
    );

    let (http_addr, rept_read_nucl, serial_nucl, repo, obj_dept_prom) = (
        ToSocketAddrs::to_socket_addrs(&format!(
            "{}:{}",
            config.http.host, config.http.port,
        ))
        .into_iter()
        .find_map(|mut addrs| addrs.next())
        .context("no address resolved for HTTP listen address")?,
        RdbNucl::<ReptRead>::new(rdb_core.clone()),
        RdbNucl::<Serial>::new(rdb_core.clone()),
        HybRepo::new(rdb_core.clone()),
        RdbObjDeptProm::new(rdb_core.clone()),
    );

    let (obj_dept, nucl) = (
        NormObjDept::new(
            rdb_core.clone(),
            obj_dept_pool.clone(),
            obj_dept_prom.clone(),
        ),
        HybNucl::new(rept_read_nucl.clone(), serial_nucl),
    );

    let (prom_actor, effect_actor, obj_dept_actor, sched) = (
        RdbPromActor::new(
            (rept_read_nucl.clone(), prom_repo),
            (
                rept_read_nucl.clone(),
                repo.clone(),
                obj_dept.view(),
                develop.clone(),
            ),
        ),
        EffectActor::new(repo.clone(), effect_recv),
        ObjDeptActor::new(obj_dept_prom, move |task| {
            //
            let (rdb_core, obj_dept_pool) =
                (rdb_core.clone(), obj_dept_pool.clone());

            async move {
                //
                NormObjDept::<R2ObjDeptPool, RdbObjDeptProm>::dispatch(
                    rdb_core,
                    obj_dept_pool,
                    task,
                )
                .await
            }
        }),
        Sched::new(rept_read_nucl, repo.clone(), obj_dept.clone()),
    );

    let (
        harn,
        obj_dept_actor_desc,
        effect_actor_desc,
        prom_actor_desc,
        sched_desc,
    ) = (
        Harn::new(config, (nucl, repo, obj_dept, prom, auth, develop)),
        obj_dept_actor.run_detach(),
        effect_actor.run_detach::<RdbContext<ReptRead>>(),
        prom_actor.run_detach(),
        sched.run_detach(),
    );

    let serve_rest = poprako_server::serve(harn, http_addr).await;

    let (sched_rest, prom_rest) = tokio::join!(
        async {
            //
            sched_desc.cancel();

            sched_desc.join().await
        },
        async {
            //
            prom_actor_desc.cancel();

            prom_actor_desc.join().await
        },
    );

    let (effect_rest, obj_dept_rest) = tokio::join!(
        async {
            //
            effect_actor_desc.cancel();

            effect_actor_desc.join().await
        },
        async {
            //
            obj_dept_actor_desc.cancel();

            obj_dept_actor_desc.join().await
        },
    );

    let mut shutdown_err = None;

    for (actor, rest) in [
        ("scheduler", sched_rest),
        ("prom", prom_rest),
        ("effect", effect_rest),
        ("object_dept", obj_dept_rest),
    ] {
        //
        if let Err(err) = rest {
            //
            tracing::error!(actor, err = ?err, "background supervisor failed");

            shutdown_err.get_or_insert_with(|| {
                //
                anyhow::Error::new(err)
                    .context(format!("{} supervisor failed", actor))
            });
        }
    }

    serve_rest?;

    match shutdown_err {
        //
        Some(err) => Err(err),

        None => Ok(()),
    }
}
