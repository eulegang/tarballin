use lsp_types::{
    DiagnosticOptions, DiagnosticServerCapabilities, InitializeParams, SaveOptions,
    ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncOptions,
    WorkDoneProgressOptions,
};
use tracing::error;

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    Workspace,
    Single,
    Adhoc,
}

#[derive(thiserror::Error, Debug)]
#[error("client does not have the requisite capabilities for this lsp")]
pub struct InvalidClient;

impl TryFrom<&InitializeParams> for Mode {
    type Error = InvalidClient;

    fn try_from(value: &InitializeParams) -> Result<Self, Self::Error> {
        if let Some(text) = &value.capabilities.text_document {
            if let Some(sync) = &text.synchronization {
                if sync.did_save != Some(true) {
                    error!("text didsave not supported");

                    return Err(InvalidClient);
                }
            }
        }

        if let Some(workspace) = &value.capabilities.workspace {
            if let Some(diagnostic) = &workspace.diagnostic {
                if diagnostic.refresh_support == Some(true) {
                    return Ok(Mode::Workspace);
                }
            }
        }

        if let Some(text) = &value.capabilities.text_document {
            if let Some(diagnostic) = &text.diagnostic {
                if diagnostic.related_document_support == Some(true) {
                    return Ok(Mode::Single);
                }
            }

            if text.publish_diagnostics.is_some() {
                return Ok(Mode::Adhoc);
            }
        }

        Err(InvalidClient)
    }
}

impl Mode {
    pub fn capabilities(&self) -> ServerCapabilities {
        match self {
            Mode::Workspace => ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: None,
                        change: None,
                        will_save: None,
                        will_save_wait_until: None,
                        save: Some(lsp_types::TextDocumentSyncSaveOptions::SaveOptions(
                            SaveOptions {
                                include_text: Some(false),
                            },
                        )),
                    },
                )),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        identifier: Some("tarpaulin".to_string()),
                        inter_file_dependencies: true,
                        workspace_diagnostics: true,
                        work_done_progress_options: WorkDoneProgressOptions {
                            work_done_progress: Some(false),
                        },
                    },
                )),

                ..ServerCapabilities::default()
            },

            Mode::Single => ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: None,
                        change: None,
                        will_save: None,
                        will_save_wait_until: None,
                        save: Some(lsp_types::TextDocumentSyncSaveOptions::SaveOptions(
                            SaveOptions {
                                include_text: Some(false),
                            },
                        )),
                    },
                )),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        identifier: Some("tarpaulin".to_string()),
                        inter_file_dependencies: true,
                        workspace_diagnostics: false,
                        work_done_progress_options: WorkDoneProgressOptions {
                            work_done_progress: Some(false),
                        },
                    },
                )),

                ..ServerCapabilities::default()
            },

            Mode::Adhoc => ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: None,
                        will_save: None,
                        will_save_wait_until: None,
                        save: Some(lsp_types::TextDocumentSyncSaveOptions::SaveOptions(
                            SaveOptions {
                                include_text: Some(false),
                            },
                        )),
                    },
                )),

                ..ServerCapabilities::default()
            },
        }
    }
}
