use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct HookPayload {
    pub hook_event_name: Option<String>,
    pub session_id: Option<String>,
    pub event: Option<String>,

    // StopFailure sub-fields
    pub error: Option<String>,

    // Notification sub-fields
    #[serde(rename = "notification_type")]
    pub notification_type: Option<String>,

    // PostToolUseFailure sub-fields
    #[serde(rename = "is_interrupt")]
    pub is_interrupt: Option<bool>,

    // SessionStart sub-fields
    pub source: Option<String>,
}

/// 将 Claude Code hook payload 映射为内部事件字符串。
///
/// 支持两种格式：
/// - 旧格式：直接 "event" 字段（向后兼容 / 手动测试）
/// - 新格式：hook_event_name + 各子字段
pub fn map_hook_event(payload: &HookPayload) -> &'static str {
    // 旧格式：直接 event 字段
    if let Some(ref event) = payload.event {
        return map_legacy_event(event);
    }

    match payload.hook_event_name.as_deref() {
        Some("SessionStart") => {
            // compact 已完成 → 完成态 session_start
            match payload.source.as_deref() {
                Some("compact") => "session_start",
                _ => "session_start",
            }
        }

        Some("PreCompact") => "running",

        Some("UserPromptSubmit") => "running",
        Some("PreToolUse") => "running",
        Some("PostToolUse") => "running",

        Some("PermissionRequest") => "need_confirm",

        Some("Notification") => match payload.notification_type.as_deref() {
            Some("permission_prompt" | "elicitation_dialog") => "need_confirm",
            _ => "idle",
        },

        Some("Stop") => "stop",

        Some("StopFailure") => match payload.error.as_deref() {
            Some("rate_limit" | "server_error") => "tool_error",
            Some(
                "authentication_failed"
                | "oauth_org_not_allowed"
                | "billing_error"
                | "invalid_request"
                | "model_not_found"
                | "max_output_tokens"
                | "unknown",
            ) => "error_final",
            _ => "tool_error",
        },

        Some("PostToolUseFailure") => {
            if payload.is_interrupt.unwrap_or(false) {
                "stop"
            } else {
                "tool_error"
            }
        }

        Some("SessionEnd") => "session_end",

        _ => "idle",
    }
}

/// 旧格式（直接 event 字段）的映射，用于向后兼容和手动测试。
fn map_legacy_event(event: &str) -> &'static str {
    match event {
        "session_start" => "session_start",
        "session_end" => "session_end",
        "running" => "running",
        "need_confirm" => "need_confirm",
        "tool_error" => "tool_error",
        "error_final" => "error_final",
        "stop" => "stop",
        "idle" => "idle",
        _ => "idle",
    }
}
