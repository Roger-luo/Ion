use serde::Serialize;

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
    println!("{}", serde_json::to_string_pretty(&resp).unwrap());
}

/// Print an action-required response and exit 2.
pub fn print_action_required<T: Serialize>(action: &'static str, data: T) -> ! {
    let resp = JsonResponse::ActionRequired::<T> {
        success: false,
        action_required: action,
        data,
    };
    println!("{}", serde_json::to_string_pretty(&resp).unwrap());
    std::process::exit(2);
}

/// Print a JSON error and exit 1.
pub fn print_error(msg: &str) -> ! {
    let resp = JsonResponse::Error::<()> {
        success: false,
        error: msg.to_string(),
    };
    println!("{}", serde_json::to_string_pretty(&resp).unwrap());
    std::process::exit(1);
}
