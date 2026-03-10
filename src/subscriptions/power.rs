// SPDX-License-Identifier: GPL-3.0-only
//
// Power actions using system commands.
// Provides lock, logout, suspend, and shutdown capabilities.

/// Lock the screen.
pub async fn lock_screen() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // loginctl lock-session works on systemd-based systems
    let _ = tokio::process::Command::new("loginctl")
        .arg("lock-session")
        .status()
        .await?;
    Ok(())
}

/// Suspend the system.
pub async fn suspend() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _ = tokio::process::Command::new("systemctl")
        .arg("suspend")
        .status()
        .await?;
    Ok(())
}

/// Power off the system.
pub async fn power_off() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _ = tokio::process::Command::new("systemctl")
        .arg("poweroff")
        .status()
        .await?;
    Ok(())
}

/// Log out by exiting the cosmic session.
pub async fn log_out() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Try cosmic-session exit first, then fallback
    let status = tokio::process::Command::new("cosmic-session")
        .arg("exit")
        .status()
        .await;

    match status {
        Ok(s) if s.success() => Ok(()),
        _ => {
            // Fallback: terminate user session via loginctl
            let uid = tokio::process::Command::new("id")
                .arg("-u")
                .output()
                .await?;

            if !uid.status.success() {
                return Err(format!("failed to get current uid: {}", uid.status).into());
            }

            let uid = String::from_utf8(uid.stdout)?
                .trim()
                .to_string();

            let _ = tokio::process::Command::new("loginctl")
                .arg("terminate-user")
                .arg(uid)
                .status()
                .await?;
            Ok(())
        }
    }
}
