use std::process::Command;

pub fn get_git_user_info() -> Option<String> {
    let name = Command::new("git")
        .args(["config", "user.name"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        });

    let email = Command::new("git")
        .args(["config", "user.email"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        });

    match (name, email) {
        (Some(n), Some(e)) => Some(format!("{} <{}>", n, e)),
        (Some(n), None) => Some(n),
        (None, Some(e)) => Some(format!("<{}>", e)),
        (None, None) => None,
    }
}
