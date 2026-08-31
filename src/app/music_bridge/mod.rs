use std::process::ExitCode;
use std::time::SystemTime;

use ::module::Merge;
use ::module::types::{Ordered, Overridable};
use eyre::{Context, ContextCompat, Result, bail};
use serde::Deserialize;
use tokio::task::JoinSet;

use crate::discord::*;
use crate::util::{SystemTimeExt, capitalize_words};

mod metadata;
mod module;
mod pipeline;
mod source;

use self::metadata::{Metadata, TrackStatus};
use self::pipeline::{Pipeline, Source};

const CLIENT_ID: &str = "1485616471035088896";

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, clap::Parser)]
pub struct Command {}

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Default, Deserialize, Merge)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Config {
    #[merge(rename = "client-id")]
    client_id: Option<Overridable<String>>,

    source: Option<Overridable<source::Config>>,

    #[serde(default)]
    module: Ordered<Vec<Module>>,
}

#[derive(Debug, Deserialize)]
struct Module {
    #[serde(default = "crate::util::r#true")]
    enable: bool,

    #[serde(default = "default_module_order")]
    order: i64,

    #[serde(flatten)]
    inner: module::Config,
}

fn default_module_order() -> i64 {
    0
}

///////////////////////////////////////////////////////////////////////////////

pub async fn run(config: &Config) -> Result<ExitCode> {
    let mut pipeline = Pipeline::new();
    let mut tasks = JoinSet::new();

    let source = config
        .source
        .as_deref()
        .cloned()
        .or_else(source::Config::default_for_platform)
        .context("there is no default source for this platform")?;

    {
        let mut modules: Vec<_> = config.module.iter().filter(|x| x.enable).collect();
        modules.sort_by_key(|x| x.order);

        for module in modules {
            let module = module.inner.clone();
            let (source, sink) = pipeline.next();
            tasks.spawn_local(
                async move { module::run(&module, source, sink).await.context("module") },
            );
        }
    }

    let discord = Discord::builder()
        .client_id(
            config
                .client_id
                .as_ref()
                .map(|x| &***x)
                .unwrap_or(CLIENT_ID)
                .to_owned(),
        )
        .finish();

    let Pipeline { input, output } = pipeline;
    tasks.spawn_local(async move { source::run(&source, input).await.context("source") });
    tasks.spawn_local(async move { run_rpc(discord, output).await.context("discord") });

    while let Some(r) = tasks.join_next().await {
        let Ok(r) = r else {
            continue;
        };

        if let Err(e) = r {
            bail!("{e:#}");
        }
    }

    Ok(ExitCode::SUCCESS)
}

async fn run_rpc(mut discord: Discord, mut source: Source<Metadata>) -> Result<()> {
    loop {
        let Some(metadata) = source.pull().await else {
            return Ok(());
        };

        debug!("-> {metadata:#?}");

        match build_activity(metadata) {
            Some(activity) => discord
                .set_activity(activity)
                .await
                .context("failed to set activity")?,

            None => discord
                .clear_activity()
                .await
                .context("faled to clear activity")?,
        }
    }
}

fn build_activity(metadata: Metadata) -> Option<Activity<'static>> {
    if metadata.status == TrackStatus::Stopped {
        return None;
    }

    let mut activity = Activity::new()
        .activity_type(ActivityType::Listening)
        .status_display_type(StatusDisplayType::Details)
        .name(capitalize_words(&metadata.player));

    if let Some(title) = metadata.title {
        activity = activity.details(title);
    }

    match (metadata.artist, metadata.album) {
        (Some(artist), Some(album)) => activity = activity.state(format!("{artist} • {album}")),
        (Some(artist), None) => activity = activity.state(artist),
        (None, Some(album)) => activity = activity.state(album),
        (None, None) => {}
    }

    if let Some(position) = metadata.position
        && let Some(length) = metadata.length
    {
        let start = SystemTime::now() - position;
        let end = start + length;

        activity = activity.timestamps(
            Timestamps::new()
                .start(start.duration_since_epoch().as_secs() as i64)
                .end(end.duration_since_epoch().as_secs() as i64),
        );
    }

    if let Some(url) = metadata.url {
        activity = activity.buttons(vec![Button::new("Listen", url)]);
    }

    if let Some(art_url) = metadata.art_url {
        activity = activity.assets(Assets::new().large_image(art_url));
    }

    Some(activity)
}
