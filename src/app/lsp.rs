use std::collections::HashMap;
use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, Instant, SystemTime};

use eyre::{Context, Result};
use module::Merge;
use serde::Deserialize;
use tokio::sync::{Mutex, MutexGuard};
use tower_lsp::{LanguageServer, LspService, Server, lsp_types::*};

use crate::discord::*;
use crate::util::{SystemTimeExt, find_repo_root, get_vcs_branch, home_dir};

const CLIENT_ID: &str = "1523025249845903410";

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, clap::Parser)]
#[command(name = "lsp")]
pub struct Command {}

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Default, Deserialize, Merge)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct File {}

///////////////////////////////////////////////////////////////////////////////

pub async fn run() -> Result<ExitCode> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|_| LspTask {
        state: Mutex::new(State {
            client: None,
            start: SystemTime::now(),
            documents: HashMap::new(),
            active_document: None,

            last_update: None,
            discord: Discord::builder().client_id(CLIENT_ID).finish(), /* TODO: fetch client id from config file */
        }),
    });

    Server::new(stdin, stdout, socket).serve(service).await;
    Ok(ExitCode::SUCCESS)
}

///////////////////////////////////////////////////////////////////////////////

struct LspTask {
    state: Mutex<State>,
}

struct State {
    client: Option<String>,
    start: SystemTime,
    documents: HashMap<Url, Document>,
    active_document: Option<Url>,

    last_update: Option<Instant>,
    discord: Discord,
}

struct Document {
    language: String,
}

#[tower_lsp::async_trait]
impl LanguageServer for LspTask {
    async fn initialize(
        &self,
        params: InitializeParams,
    ) -> tower_lsp::jsonrpc::Result<InitializeResult> {
        trace!("initialize");

        let mut state = self.state.lock().await;

        if let Some(client_info) = params.client_info {
            state.client = Some(client_info.name);
        }

        state.start = SystemTime::now();

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::NONE,
                )),

                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        trace!("initialized");
    }

    async fn shutdown(&self) -> tower_lsp::jsonrpc::Result<()> {
        trace!("shutdown");
        let mut state = self.state.lock().await;

        state.active_document = None;
        state.documents.clear();
        self.update_presence(&mut state, true).await;
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        trace!("did_open");
        let uri = params.text_document.uri;
        let mut state = self.state.lock().await;

        state.documents.insert(
            uri.clone(),
            Document {
                language: params.text_document.language_id,
            },
        );
        state.active_document = Some(uri);

        self.update_presence(&mut state, true).await;
    }

    async fn hover(&self, params: HoverParams) -> tower_lsp::jsonrpc::Result<Option<Hover>> {
        trace!("hover");
        let mut state = self.state.lock().await;

        state.active_document = Some(params.text_document_position_params.text_document.uri);
        self.update_presence(&mut state, false).await;

        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String("hovering file".to_string())),
            range: None,
        }))
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        trace!("did_change");
        let mut state = self.state.lock().await;

        state.active_document = Some(params.text_document.uri);
        self.update_presence(&mut state, false).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        trace!("did_save");
        let mut state = self.state.lock().await;

        state.active_document = Some(params.text_document.uri);
        self.update_presence(&mut state, false).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        trace!("did_close");
        let mut state = self.state.lock().await;

        state.active_document = None;
        state.documents.remove(&params.text_document.uri);
        self.update_presence(&mut state, true).await;
    }
}

impl LspTask {
    async fn update_presence(&self, state: &mut MutexGuard<'_, State>, important: bool) {
        let now = Instant::now();

        if !important
            && state.last_update.is_some_and(
                |x| now - x < Duration::from_secs(1), /* TODO: fetch interval from config file */
            )
        {
            trace!("skip");
            return;
        }

        state.last_update = Some(now);

        let r = try2!(async {
            if state.active_document.is_some() {
                trace!("update");

                // SAFETY: We checked the precondition above.
                let activity = self.build_activity(state);

                state
                    .discord
                    .set_activity(activity)
                    .await
                    .context("cannot update rich presence")
            } else {
                trace!("clear");
                state
                    .discord
                    .clear_activity()
                    .await
                    .context("cannot clear rich presence")
            }
        });

        if let Err(e) = r {
            error!("{e:#}");
        }
    }

    /// # Panics
    ///
    /// If `state.active_document` is `None`.
    fn build_activity(&self, state: &State) -> Activity<'static> {
        let State { start, .. } = state;

        let mut activity = Activity::new()
            .activity_type(ActivityType::Playing)
            .status_display_type(StatusDisplayType::Name)
            .timestamps(Timestamps::new().start(start.duration_since_epoch().as_secs() as i64))
            .party(Party::new().size([1, 1]));

        if let Some(ref client) = state.client {
            activity = activity.name(client.clone());
        }

        if let Some(active_document) = state.active_document.as_ref() {
            let document_path = Path::new(active_document.path());

            activity = activity.details(
                None.or_else(|| {
                    let repo = find_repo_root(document_path)?;
                    let repo_name = repo.file_name()?;
                    let relative_document_path =
                        document_path.strip_prefix(repo).unwrap_or(document_path);

                    Some(format!(
                        "{}: {}",
                        repo_name.display(),
                        relative_document_path.display()
                    ))
                })
                .or_else(|| {
                    let home = home_dir()?;

                    Some(match document_path.strip_prefix(home) {
                        Ok(x) => format!("{}", Path::new("~").join(x).display()),
                        Err(_) => format!("{}", document_path.display()),
                    })
                })
                .unwrap_or_else(|| format!("{}", document_path.display())),
            );

            if let Some(branch) = get_vcs_branch(document_path.parent().unwrap_or(document_path))
                .ok()
                .flatten()
            {
                activity = activity.state(branch);
            }

            let mut assets = Assets::new();

            if let Some(document) = state.documents.get(active_document) {
                assets = assets.large_image(document.language.clone());
            }

            activity = activity.assets(assets);
        }

        activity
    }
}
