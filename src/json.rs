use std::sync::OnceLock;

use serde::Serialize;

/// Whether to pretty-print JSON output. Set once from `main()`.
static PRETTY: OnceLock<bool> = OnceLock::new();

/// Configure pretty-printing. Call before any JSON output.
pub fn set_pretty(pretty: bool) {
    PRETTY.set(pretty).ok();
}

/// Serialize a value to JSON, respecting the pretty-print setting.
fn to_json<T: Serialize>(value: &T) -> String {
    if PRETTY.get().copied().unwrap_or(false) {
        serde_json::to_string_pretty(value).unwrap()
    } else {
        serde_json::to_string(value).unwrap()
    }
}

/// Standard JSON response envelope.
#[derive(Serialize)]
#[serde(untagged)]
pub enum JsonResponse<T: Serialize> {
    Success {
        success: bool,
        data: T,
    },
    ActionRequired {
        success: bool,
        action_required: &'static str,
        data: T,
    },
    Error {
        success: bool,
        error: String,
    },
}

/// Print a success response and exit 0.
pub fn print_success<T: Serialize>(data: T) {
    let resp = JsonResponse::Success::<T> {
        success: true,
        data,
    };
    println!("{}", to_json(&resp));
}

/// Print an action-required response and exit 2.
pub fn print_action_required<T: Serialize>(action: &'static str, data: T) -> ! {
    let resp = JsonResponse::ActionRequired::<T> {
        success: false,
        action_required: action,
        data,
    };
    println!("{}", to_json(&resp));
    std::process::exit(2);
}

/// Print a JSON error and exit 1.
pub fn print_error(msg: &str) -> ! {
    let resp = JsonResponse::Error::<()> {
        success: false,
        error: msg.to_string(),
    };
    println!("{}", to_json(&resp));
    std::process::exit(1);
}
