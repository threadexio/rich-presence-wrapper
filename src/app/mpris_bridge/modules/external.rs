use std::collections::HashMap;
use std::process::Stdio;
use std::time::Duration;

use eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::Command;

use super::prelude::*;

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Deserialize)]
pub struct Config {
    program: Vec<String>,

    #[serde(default)]
    env: HashMap<String, String>,
}

///////////////////////////////////////////////////////////////////////////////

pub async fn setup(pipeline: &mut pipeline::Builder<Record>, config: &Config) -> Result<()> {
    if config.program.is_empty() {
        bail!("`program` cannot be empty")
    }

    let mut command = Command::new(&config.program[0]);
    command.args(&config.program[1..]);
    command.envs(config.env.iter());

    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());

    pipeline.stage(ExternalBuilder { command });

    Ok(())
}

///////////////////////////////////////////////////////////////////////////////

struct ExternalBuilder {
    command: Command,
}

impl StageBuilder<Record> for ExternalBuilder {
    type Stage = ExternalStage;

    fn build(self, sink: Sink<Record>, source: Source<Record>) -> Self::Stage {
        let Self { command } = self;

        ExternalStage {
            command,
            source,
            sink,
        }
    }
}

struct ExternalStage {
    command: Command,
    source: Source<Record>,
    sink: Sink<Record>,
}

impl Stage<Record> for ExternalStage {
    async fn run(&mut self) -> Result<()> {
        let mut process = self
            .command
            .spawn()
            .context("failed to spawn external program")?;

        let mut stdin = process
            .stdin
            .take()
            .map(BufWriter::new)
            .expect("stdin was piped");

        let mut stdout = process
            .stdout
            .take()
            .map(BufReader::new)
            .expect("stdout was piped");

        let mut line = String::new();

        loop {
            let Some(record) = self.source.pull().await else {
                return Ok(());
            };

            let record = JsonRecord::from(record);

            try2!(async {
                let x = serde_json::to_vec(&record)?;
                stdin.write_all(&x).await?;
                stdin.write_all(b"\n").await?;
                stdin.flush().await?;
                Result::<()>::Ok(())
            })
            .context("failed to write record to program's stdin")?;

            let record = try2!(async {
                line.clear();
                stdout.read_line(&mut line).await?;

                let record: JsonRecord = serde_json::from_str(line.trim())?;
                Record::try_from(record)
            })
            .context("failed to read record from program's stdin")?;

            if !self.sink.push(record) {
                return Ok(());
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
struct JsonRecord {
    player: String,
    status: String,
    track_id: String,
    title: Option<String>,
    album: Option<String>,
    artist: Option<String>,
    url: Option<String>,
    art_url: Option<String>,
    position: Option<f32>,
    length: Option<f32>,
}

impl From<Record> for JsonRecord {
    fn from(x: Record) -> Self {
        Self {
            player: x.player,
            status: match x.status {
                TrackStatus::Playing => "playing",
                TrackStatus::Paused => "paused",
                TrackStatus::Stopped => "stopped",
            }
            .to_owned(),
            track_id: x.track_id,
            title: x.title,
            album: x.album,
            artist: x.artist,
            url: x.url,
            art_url: x.art_url,
            position: x.position.as_ref().map(Duration::as_secs_f32),
            length: x.length.as_ref().map(Duration::as_secs_f32),
        }
    }
}

impl TryFrom<JsonRecord> for Record {
    type Error = eyre::Error;

    fn try_from(x: JsonRecord) -> Result<Self, Self::Error> {
        Ok(Self {
            player: x.player,
            status: match x.status.as_str() {
                "playing" => TrackStatus::Playing,
                "paused" => TrackStatus::Paused,
                "stopped" => TrackStatus::Stopped,
                _ => bail!("invalid 'status'"),
            },
            track_id: x.track_id,
            title: x.title,
            album: x.album,
            artist: x.artist,
            url: x.url,
            art_url: x.art_url,
            position: x.position.map(Duration::from_secs_f32),
            length: x.length.map(Duration::from_secs_f32),
        })
    }
}
