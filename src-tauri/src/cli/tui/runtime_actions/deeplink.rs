use crate::cli::i18n::texts;
use crate::error::AppError;

use super::super::app::ToastKind;
use super::super::data::{load_state, UiData};
use super::RuntimeActionContext;

/// Import a resource (provider/mcp/prompt/skill) from a ccswitch:// deep
/// link URL. Mirrors the CLI `deeplink` command: the URL fully describes
/// the target app(s), so the current TUI app selection is irrelevant.
pub(super) fn import(ctx: &mut RuntimeActionContext<'_>, url: String) -> Result<(), AppError> {
    let request = crate::parse_deeplink_url(&url)?;
    let state = load_state()?;

    let message = match request.resource.as_str() {
        "provider" => {
            let app_label = request.app.clone().unwrap_or_default();
            let name = request.name.clone().unwrap_or_default();
            let switched = request.enabled == Some(true);
            let provider_id = crate::import_provider_from_deeplink(&state, request)?;
            texts::tui_toast_deeplink_provider_imported(&name, &app_label, &provider_id, switched)
        }
        "mcp" => {
            let apps_label = request.apps.clone().unwrap_or_default();
            let result = crate::import_mcp_from_deeplink(&state, request)?;
            texts::tui_toast_deeplink_mcp_imported(
                result.imported_count,
                &apps_label,
                result.failed.len(),
            )
        }
        "prompt" => {
            let app_label = request.app.clone().unwrap_or_default();
            let name = request.name.clone().unwrap_or_default();
            let enabled = request.enabled == Some(true);
            crate::import_prompt_from_deeplink(&state, request)?;
            texts::tui_toast_deeplink_prompt_imported(&name, &app_label, enabled)
        }
        "skill" => {
            let repo_id = crate::import_skill_from_deeplink(&state, request)?;
            texts::tui_toast_deeplink_skill_imported(&repo_id)
        }
        other => {
            return Err(AppError::InvalidInput(
                texts::deeplink_unsupported_resource_error(other),
            ));
        }
    };

    ctx.app.push_toast(message, ToastKind::Success);
    *ctx.data = UiData::load(&ctx.app.app_type)?;
    Ok(())
}
