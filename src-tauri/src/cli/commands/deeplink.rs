use clap::Args;
use inquire::Text;

use crate::app_config::AppType;
use crate::cli::i18n::texts;
use crate::cli::ui::{info, success};
use crate::error::AppError;
use crate::store::AppState;

#[derive(Args, Debug, Clone)]
pub struct DeeplinkCommand {
    /// The ccswitch://v1/import?... URL to import; when omitted you are
    /// prompted to paste the deep link interactively
    pub url: Option<String>,
}

pub fn execute(cmd: DeeplinkCommand, app: Option<AppType>) -> Result<(), AppError> {
    if app.is_some() {
        return Err(AppError::InvalidInput(
            "`--app` cannot be used with `deeplink`; target app(s) must be encoded in the URL via `app` or `apps`."
                .to_string(),
        ));
    }

    let url = resolve_deeplink_url(cmd.url)?;
    let request = crate::parse_deeplink_url(&url)?;
    let state = AppState::try_new()?;

    match request.resource.as_str() {
        "provider" => import_provider(&state, request),
        "mcp" => import_mcp(&state, request),
        "prompt" => import_prompt(&state, request),
        "skill" => import_skill(&state, request),
        other => Err(AppError::InvalidInput(
            texts::deeplink_unsupported_resource_error(other),
        )),
    }
}

/// Resolve the deep link URL from the positional argument, or prompt for an
/// interactive paste when it was omitted. Surrounding whitespace (including
/// the trailing newline a terminal paste appends) is trimmed.
fn resolve_deeplink_url(url: Option<String>) -> Result<String, AppError> {
    let url = match url {
        Some(url) => url,
        None => Text::new(texts::deeplink_paste_prompt())
            .with_help_message(texts::deeplink_paste_help())
            .prompt()
            .map_err(|e| AppError::Message(texts::input_failed_error(&e.to_string())))?,
    };
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidInput(
            texts::deeplink_url_empty_error().to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

fn import_provider(
    state: &AppState,
    request: crate::DeepLinkImportRequest,
) -> Result<(), AppError> {
    let app_label = request.app.clone().unwrap_or_default();
    let name = request.name.clone().unwrap_or_default();
    let switched = request.enabled == Some(true);

    let provider_id = crate::import_provider_from_deeplink(state, request)?;

    println!(
        "{}",
        success(&format!(
            "✓ Imported provider '{name}' (id: {provider_id}) for {app_label}"
        ))
    );
    if switched {
        println!("{}", info(&format!("  Switched to '{provider_id}'")));
    }
    Ok(())
}

fn import_mcp(state: &AppState, request: crate::DeepLinkImportRequest) -> Result<(), AppError> {
    let apps_label = request.apps.clone().unwrap_or_default();
    let result = crate::import_mcp_from_deeplink(state, request)?;

    println!(
        "{}",
        success(&format!(
            "✓ Imported {} MCP server(s) for {apps_label}",
            result.imported_count
        ))
    );
    for id in &result.imported_ids {
        println!("{}", info(&format!("  • {id}")));
    }
    for failure in &result.failed {
        println!(
            "{}",
            crate::cli::ui::warning(&format!("  ✗ {}: {}", failure.id, failure.error))
        );
    }
    Ok(())
}

fn import_prompt(state: &AppState, request: crate::DeepLinkImportRequest) -> Result<(), AppError> {
    let app_label = request.app.clone().unwrap_or_default();
    let name = request.name.clone().unwrap_or_default();
    let enabled = request.enabled == Some(true);

    let prompt_id = crate::import_prompt_from_deeplink(state, request)?;

    println!(
        "{}",
        success(&format!(
            "✓ Imported prompt '{name}' (id: {prompt_id}) for {app_label}"
        ))
    );
    if enabled {
        println!("{}", info(&format!("  Enabled '{prompt_id}'")));
    }
    Ok(())
}

fn import_skill(state: &AppState, request: crate::DeepLinkImportRequest) -> Result<(), AppError> {
    let repo_id = crate::import_skill_from_deeplink(state, request)?;

    println!("{}", success(&format!("✓ Added skill repo '{repo_id}'")));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command() -> DeeplinkCommand {
        DeeplinkCommand {
            url: Some("ccswitch://v1/import?resource=provider&app=claude&name=Demo".to_string()),
        }
    }

    #[test]
    fn rejects_global_app_flag() {
        let err = execute(command(), Some(AppType::Codex))
            .expect_err("`--app` should be rejected for deeplink");

        match err {
            AppError::InvalidInput(message) => {
                assert!(
                    message.contains("`--app` cannot be used with `deeplink`"),
                    "unexpected message: {message}"
                );
                assert!(
                    message.contains("`app` or `apps`"),
                    "message should point users to the URL parameters: {message}"
                );
            }
            other => panic!("expected InvalidInput error, got {other:?}"),
        }
    }

    #[test]
    fn app_flag_is_rejected_before_parsing_url() {
        // An invalid URL would normally fail during parsing; the `--app` guard
        // must short-circuit first so the error always points at the flag.
        let cmd = DeeplinkCommand {
            url: Some("not-a-valid-deeplink".to_string()),
        };

        let err = execute(cmd, Some(AppType::Claude))
            .expect_err("`--app` should be rejected before URL parsing");

        assert!(
            matches!(&err, AppError::InvalidInput(message) if message.contains("`--app` cannot be used with `deeplink`")),
            "expected the `--app` rejection, got {err:?}"
        );
    }

    #[test]
    fn resolve_deeplink_url_trims_surrounding_whitespace() {
        let url = resolve_deeplink_url(Some("  ccswitch://v1/import?resource=prompt  \n".into()))
            .expect("argument URLs should be accepted after trimming");
        assert_eq!(url, "ccswitch://v1/import?resource=prompt");
    }

    #[test]
    fn resolve_deeplink_url_rejects_whitespace_only_argument() {
        let err = resolve_deeplink_url(Some("   \n".into()))
            .expect_err("a whitespace-only URL argument should be rejected");
        assert!(
            matches!(err, AppError::InvalidInput(_)),
            "expected InvalidInput, got {err:?}"
        );
    }
}
