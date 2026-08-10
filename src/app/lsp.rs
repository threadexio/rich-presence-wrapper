use std::collections::HashMap;
use std::path::Path;
use std::process::ExitCode;
use std::time::SystemTime;

use eyre::{Context, Result};
use module::Merge;
use serde::Deserialize;
use tokio::sync::{Mutex, MutexGuard};
use tower_lsp::lsp_types::request::*;
use tower_lsp::lsp_types::*;
use tower_lsp::{LanguageServer, LspService, Server};

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

    async fn code_action(
        &self,
        params: CodeActionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<CodeActionResponse>> {
        trace!("code_action");
        self.set_focus(params.text_document.uri).await;
        Ok(None)
    }

    async fn code_lens(
        &self,
        params: CodeLensParams,
    ) -> tower_lsp::jsonrpc::Result<Option<Vec<CodeLens>>> {
        trace!("code_lens");
        self.set_focus(params.text_document.uri).await;
        Ok(None)
    }

    async fn color_presentation(
        &self,
        params: ColorPresentationParams,
    ) -> tower_lsp::jsonrpc::Result<Vec<ColorPresentation>> {
        trace!("color_presentation");
        self.set_focus(params.text_document.uri).await;
        Ok(Vec::new())
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<CompletionResponse>> {
        trace!("completion");
        self.set_focus(params.text_document_position.text_document.uri)
            .await;
        Ok(None)
    }

    async fn diagnostic(
        &self,
        params: DocumentDiagnosticParams,
    ) -> tower_lsp::jsonrpc::Result<DocumentDiagnosticReportResult> {
        trace!("diagnostic");
        self.set_focus(params.text_document.uri).await;
        Ok(DocumentDiagnosticReportResult::Report(
            DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    result_id: None,
                    items: Vec::new(),
                },
            }),
        ))
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

    async fn document_color(
        &self,
        params: DocumentColorParams,
    ) -> tower_lsp::jsonrpc::Result<Vec<ColorInformation>> {
        trace!("document_color");
        self.set_focus(params.text_document.uri).await;
        Ok(Vec::new())
    }

    async fn document_highlight(
        &self,
        params: DocumentHighlightParams,
    ) -> tower_lsp::jsonrpc::Result<Option<Vec<DocumentHighlight>>> {
        trace!("document_highlight");
        self.set_focus(params.text_document_position_params.text_document.uri)
            .await;
        Ok(None)
    }

    async fn document_link(
        &self,
        params: DocumentLinkParams,
    ) -> tower_lsp::jsonrpc::Result<Option<Vec<DocumentLink>>> {
        trace!("document_link");
        self.set_focus(params.text_document.uri).await;
        Ok(None)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> tower_lsp::jsonrpc::Result<Option<DocumentSymbolResponse>> {
        trace!("document_symbol");
        self.set_focus(params.text_document.uri).await;
        Ok(None)
    }

    async fn folding_range(
        &self,
        params: FoldingRangeParams,
    ) -> tower_lsp::jsonrpc::Result<Option<Vec<FoldingRange>>> {
        trace!("folding_range");
        self.set_focus(params.text_document.uri).await;
        Ok(None)
    }

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> tower_lsp::jsonrpc::Result<Option<Vec<TextEdit>>> {
        trace!("formatting");
        self.set_focus(params.text_document.uri).await;
        Ok(None)
    }

    async fn goto_declaration(
        &self,
        params: GotoDeclarationParams,
    ) -> tower_lsp::jsonrpc::Result<Option<GotoDeclarationResponse>> {
        trace!("goto_declaration");
        self.set_focus(params.text_document_position_params.text_document.uri)
            .await;
        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<GotoDefinitionResponse>> {
        trace!("goto_definition");
        self.set_focus(params.text_document_position_params.text_document.uri)
            .await;
        Ok(None)
    }

    async fn goto_implementation(
        &self,
        params: GotoImplementationParams,
    ) -> tower_lsp::jsonrpc::Result<Option<GotoImplementationResponse>> {
        trace!("goto_implementation");
        self.set_focus(params.text_document_position_params.text_document.uri)
            .await;
        Ok(None)
    }

    async fn goto_type_definition(
        &self,
        params: GotoTypeDefinitionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<GotoTypeDefinitionResponse>> {
        trace!("goto_type_definition");
        self.set_focus(params.text_document_position_params.text_document.uri)
            .await;
        Ok(None)
    }

    async fn hover(&self, params: HoverParams) -> tower_lsp::jsonrpc::Result<Option<Hover>> {
        trace!("hover");
        self.set_focus(params.text_document_position_params.text_document.uri)
            .await;
        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String("hovering file".to_string())),
            range: None,
        }))
    }

    async fn initialized(&self, _params: InitializedParams) {
        trace!("initialized");
    }

    async fn inlay_hint(
        &self,
        params: InlayHintParams,
    ) -> tower_lsp::jsonrpc::Result<Option<Vec<InlayHint>>> {
        trace!("inlay_hint");
        self.set_focus(params.text_document.uri).await;
        Ok(None)
    }

    async fn inline_value(
        &self,
        params: InlineValueParams,
    ) -> tower_lsp::jsonrpc::Result<Option<Vec<InlineValue>>> {
        trace!("inline_value");
        self.set_focus(params.text_document.uri).await;
        Ok(None)
    }
}

impl LspTask {
    async fn set_focus(&self, active_document: Url) {
        let mut state = self.state.lock().await;
        state.active_document = Some(active_document);
        self.update_presence(&mut state).await;
    }

    async fn update_presence(&self, state: &mut MutexGuard<'_, State>) {
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
