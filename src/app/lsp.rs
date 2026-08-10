use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::SystemTime;

use eyre::{Context, ContextCompat, Result};
use module::Merge;
use module::types::Overridable;
use serde::Deserialize;
use tokio::sync::Mutex;
use tower_lsp::lsp_types::*;
use tower_lsp::{LanguageServer, LspService, Server};

use crate::config::Config;
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
pub struct File {
    #[merge(rename = "client-id")]
    client_id: Option<Overridable<String>>,
}

///////////////////////////////////////////////////////////////////////////////

pub async fn run(config: &Config) -> Result<ExitCode> {
    let client_id = config
        .lsp
        .client_id
        .as_ref()
        .map(|x| &***x)
        .unwrap_or(CLIENT_ID);

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|_| LspTask {
        state: Mutex::new(State {
            client: None,
            start: SystemTime::now(),
            documents: HashMap::new(),
            active_document: None,

            last_activity_params: None,
            discord: Discord::builder().client_id(client_id).finish(),
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

    last_activity_params: Option<ActivityVolatileParams>,
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
            server_info: Some(ServerInfo {
                name: env!("CARGO_PKG_NAME").to_owned(),
                version: Some(env!("CARGO_PKG_VERSION").to_owned()),
            }),

            capabilities: ServerCapabilities {
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::NONE,
                )),

                ..Default::default()
            },
        })
    }

    async fn shutdown(&self) -> tower_lsp::jsonrpc::Result<()> {
        trace!("shutdown");
        let mut state = self.state.lock().await;

        state.active_document = None;
        state.documents.clear();
        self.update_presence(&mut state).await;
        Ok(())
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        trace!("did_change");
        self.set_focus(params.text_document.uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        trace!("did_close");
        let mut state = self.state.lock().await;
        state.active_document = None;
        state.documents.remove(&params.text_document.uri);
        self.update_presence(&mut state).await;
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

        self.update_presence(&mut state).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        trace!("did_save");
        self.set_focus(params.text_document.uri).await;
    }

    async fn hover(&self, params: HoverParams) -> tower_lsp::jsonrpc::Result<Option<Hover>> {
        trace!("hover");
        self.set_focus(params.text_document_position_params.text_document.uri)
            .await;
        Ok(None)
    }

    async fn initialized(&self, _params: InitializedParams) {
        trace!("initialized");
    }
}

impl LspTask {
    async fn set_focus(&self, active_document: Url) {
        let mut state = self.state.lock().await;
        state.active_document = Some(active_document);
        self.update_presence(&mut state).await;
    }

    async fn update_presence(&self, state: &mut State) {
        let r = try2!(async {
            match state.active_document.as_ref() {
                Some(active_document_uri) => {
                    let active_document = state
                        .documents
                        .get(active_document_uri)
                        .context("active document was never opened")?;

                    let new_params = ActivityVolatileParams {
                        document_path: urlencoding::decode(active_document_uri.path())
                            .map(Cow::into_owned)
                            .map(Into::into)
                            .context("non-utf8 document path")?,

                        language: active_document.language.clone(),
                    };

                    if state
                        .last_activity_params
                        .as_ref()
                        .is_some_and(|old| *old == new_params)
                    {
                        trace!("skip update");
                        return Ok(());
                    }

                    let activity = build_activity(
                        ActivityPersistentParams {
                            client: state.client.as_deref(),
                            start: state.start,
                        },
                        &new_params,
                    );

                    trace!("update");
                    state
                        .discord
                        .set_activity(activity)
                        .await
                        .context("cannot update rich presence")?;

                    state.last_activity_params = Some(new_params);
                    Ok(())
                }

                None => {
                    trace!("clear");
                    state
                        .discord
                        .clear_activity()
                        .await
                        .context("cannot clear rich presence")
                }
            }
        });

        if let Err(e) = r {
            error!("{e:#}");
        }
    }
}

struct ActivityPersistentParams<'a> {
    client: Option<&'a str>,
    start: SystemTime,
}

#[derive(PartialEq, Eq)]
struct ActivityVolatileParams {
    document_path: PathBuf,
    language: String,
}

fn build_activity(
    persistent_params: ActivityPersistentParams<'_>,
    volatile_params: &ActivityVolatileParams,
) -> Activity<'static> {
    let ActivityPersistentParams { client, start } = persistent_params;
    let ActivityVolatileParams {
        document_path,
        language,
    } = volatile_params;

    let mut activity = Activity::new()
        .activity_type(ActivityType::Playing)
        .status_display_type(StatusDisplayType::Name)
        .timestamps(Timestamps::new().start(start.duration_since_epoch().as_secs() as i64))
        .party(Party::new().size([1, 1]));

    if let Some(client) = client {
        activity = activity.name(client.to_owned());
    }

    activity = activity.details(
        None.or_else(|| {
            let repo = find_repo_root(document_path)?;
            let repo_name = repo.file_name()?;
            let relative_document_path = document_path.strip_prefix(repo).unwrap_or(document_path);

            Some(format!(
                "{}: {}",
                repo_name.display(),
                relative_document_path.display(),
            ))
        })
        .or_else(|| {
            let home = home_dir()?;
            let document_path = document_path.strip_prefix(home).unwrap_or(document_path);
            Some(format!("{}", document_path.display()))
        })
        .unwrap_or_else(|| format!("{}", document_path.display())),
    );

    if let Some(branch) = get_vcs_branch(document_path.parent().unwrap_or(document_path))
        .ok()
        .flatten()
    {
        activity = activity.state(branch);
    }

    activity = activity.assets(Assets::new().large_image(language.clone()));

    activity
}
