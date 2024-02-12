use lsp_types::InitializeParams;

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
