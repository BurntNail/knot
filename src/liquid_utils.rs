use crate::{error::KnotError, liquid_utils::partials::PARTIALS, PROJECT_NAME};
use axum::response::Html;
use chrono::NaiveDateTime;
use liquid::{model::Value, Object, ParserBuilder};
use once_cell::sync::Lazy;
use std::{
    env::{self, var},
    fmt::Debug,
    path::Path,
};
use tokio::fs::read_to_string;
use tracing::Instrument;

pub mod partials;

pub static CFT_SITEKEY: Lazy<String> =
    Lazy::new(|| var("CFT_SITEKEY").expect("missing environment variable `CFT_SITEKEY`"));
pub static DOMAIN: Lazy<(bool, String)> = Lazy::new(|| {
    if let Ok(dom) = var("DOMAIN") {
        (true, dom)
    } else {
        (false, String::new())
    }
});

#[instrument(level = "debug", skip(globals))]
pub async fn compile(
    path: impl AsRef<Path> + Debug,
    mut globals: Object,
) -> Result<Html<String>, KnotError> {
    let (liquid, partial_compiler) = async move {
        debug!("Reading in file + partials");
        (
            read_to_string(path).await,
            PARTIALS.read().await.to_compiler(),
        )
    }
    .instrument(debug_span!("compile_preparations"))
    .await;
    let liquid = liquid?;

    debug!("Inserting globals");

    globals.insert("cft_sitekey".into(), Value::scalar(CFT_SITEKEY.as_str()));
    globals.insert(
        "siteinfo".into(),
        Value::Object(liquid::object!({
            "instance_name": PROJECT_NAME.as_str(),
            "domain_exists": DOMAIN.0,
            "domain": DOMAIN.1.as_str()
        })),
    );

    let html = tokio::task::spawn_blocking(move || {
        debug!("Compiling");
        ParserBuilder::with_stdlib()
            .partials(partial_compiler)
            .build()?
            .parse(&liquid)?
            .render(&globals)
    })
    .instrument(debug_span!("acc_compilation"))
    .await?;

    Ok(html?).map(Html)
}

pub trait EnvFormatter {
    fn to_env_string(&self) -> String;
}
impl EnvFormatter for NaiveDateTime {
    fn to_env_string(&self) -> String {
        self.format(&env::var("DATE_TIME_FORMAT").unwrap_or_else(|e| {
            warn!(%e, "Missing DATE_TIME_FORMAT");
            "%c".into()
        }))
        .to_string()
    }
}
