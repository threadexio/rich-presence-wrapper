use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use eyre::{Context, Result, bail};
use serde::Deserialize;

use super::prelude::*;

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Deserialize)]
pub struct Config {
    text: Option<String>,
    source: Option<PathBuf>,
}

///////////////////////////////////////////////////////////////////////////////

pub async fn setup(pipeline: &mut pipeline::Builder<Record>, config: &Config) -> Result<()> {
    let mut sources = rune::Sources::new();

    let source = match (&config.text, &config.source) {
        (Some(code), None) => rune::Source::memory(code).map_err(eyre::Error::from),

        (None, Some(path)) => {
            rune::Source::from_path(path).with_context(|| path.display().to_string())
        }

        (Some(_), Some(_)) | (None, None) => {
            bail!("exactly one of `text` or `source` must be specified")
        }
    }
    .context("cannot parse script source")?;

    sources.insert(source).context("cannot add script source")?;

    pipeline.stage(ScriptBuilder { sources });

    Ok(())
}

///////////////////////////////////////////////////////////////////////////////

struct ScriptBuilder {
    sources: rune::Sources,
}

impl StageBuilder<Record> for ScriptBuilder {
    type Stage = ScriptStage;

    fn build(self, sink: Sink<Record>, source: Source<Record>) -> Self::Stage {
        let Self { sources } = self;

        ScriptStage {
            sources,
            source,
            sink: Some(sink),
        }
    }
}

struct ScriptStage {
    sources: rune::Sources,

    source: Source<Record>,
    sink: Option<Sink<Record>>,
}

impl Stage<Record> for ScriptStage {
    async fn run(&mut self) -> Result<()> {
        let mut vm = self.build_vm().context("cannot build script")?;

        loop {
            let Some(record) = self.source.pull().await else {
                return Ok(());
            };

            vm.call(["process"], (emit::Record::from(record),))
                .context("script")?;
        }
    }
}

impl ScriptStage {
    fn build_vm(&mut self) -> Result<rune::Vm> {
        let mut cx = rune::Context::with_default_modules()?;
        cx.install(emit::module(
            self.sink.take().expect("build_vm() called twice?"),
        )?)?;

        let runtime = Arc::new(cx.runtime()?);
        let mut diagnostics = rune::Diagnostics::new();

        let result = rune::prepare(&mut self.sources)
            .with_context(&cx)
            .with_diagnostics(&mut diagnostics)
            .build();

        if !diagnostics.is_empty() {
            let mut out = rune::termcolor::Buffer::no_color();
            diagnostics.emit(&mut out, &self.sources)?;

            if let Ok(out) = str::from_utf8(out.as_slice()) {
                warn!("script: {out}");
            }
        }

        let unit = result?;
        let vm = rune::Vm::new(runtime, Arc::new(unit));
        Ok(vm)
    }
}

mod emit {
    use super::*;

    #[derive(rune::Any)]
    pub struct Record {
        #[rune(get, set)]
        player: String,
        #[rune(get, set)]
        status: String,
        #[rune(get, set)]
        track_id: String,
        #[rune(get, set)]
        title: Option<String>,
        #[rune(get, set)]
        album: Option<String>,
        #[rune(get, set)]
        artist: Option<String>,
        #[rune(get, set)]
        url: Option<String>,
        #[rune(get, set)]
        art_url: Option<String>,
        #[rune(get, set)]
        position: Option<f32>,
        #[rune(get, set)]
        length: Option<f32>,
    }

    impl From<super::Record> for Record {
        fn from(x: super::Record) -> Self {
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
                position: x.position.map(|x| x.as_secs_f32()),
                length: x.length.map(|x| x.as_secs_f32()),
            }
        }
    }

    impl TryFrom<Record> for super::Record {
        type Error = eyre::Error;

        fn try_from(x: Record) -> Result<Self, Self::Error> {
            Ok(Self {
                player: x.player,
                status: match x.status.as_str() {
                    "playing" => TrackStatus::Playing,
                    "paused" => TrackStatus::Paused,
                    "stopped" => TrackStatus::Stopped,
                    _ => bail!("invalid record 'status'"),
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

    pub fn module(sink: Sink<super::Record>) -> Result<rune::Module> {
        let mut m = rune::Module::new();

        m.ty::<Record>()?;

        m.function("emit", {
            struct EmitContext {
                sink: Sink<super::Record>,
            }

            let cx = Mutex::new(EmitContext { sink });

            move |record: Record| {
                // SAFETY: No one else has access to `cx`, so it is
                //         impossible for `cx` to be borrowed outside the
                //         context of this closure.
                let mut cx = cx.try_lock().expect("how");

                let record = match record.try_into().context("called `emit()`") {
                    Ok(x) => x,
                    Err(e) => {
                        error!("{e:#}");
                        return;
                    }
                };

                cx.sink.push(record);
            }
        })
        .build()?;

        Ok(m)
    }
}
