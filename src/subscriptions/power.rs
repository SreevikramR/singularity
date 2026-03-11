// SPDX-License-Identifier: GPL-3.0-only
//
// Power actions using DBus (org.freedesktop.login1).
// Provides lock, logout, suspend, and shutdown capabilities.

use zbus::{proxy, Connection, Result};

#[proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait LoginManager {
    async fn lock_sessions(&self) -> Result<()>;
    async fn suspend(&self, interactive: bool) -> Result<()>;
    async fn power_off(&self, interactive: bool) -> Result<()>;
    async fn terminate_user(&self, uid: u32) -> Result<()>;
}

/// Lock the screen.
pub async fn lock_screen() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let connection = Connection::system().await?;
    let proxy = LoginManagerProxy::new(&connection).await?;
    proxy.lock_sessions().await?;
    Ok(())
}

/// Suspend the system.
pub async fn suspend() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let connection = Connection::system().await?;
    let proxy = LoginManagerProxy::new(&connection).await?;
    proxy.suspend(true).await?;
    Ok(())
}

/// Power off the system.
pub async fn power_off() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let connection = Connection::system().await?;
    let proxy = LoginManagerProxy::new(&connection).await?;
    proxy.power_off(true).await?;
    Ok(())
}

/// Log out by exiting the cosmic session.
pub async fn log_out() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Try cosmic-session exit first, then fallback
    let status = tokio::process::Command::new("cosmic-session")
        .arg("exit")
        .status()
        .await;

    match status {
        Ok(s) if s.success() => Ok(()),
        _ => {
            // Fallback: terminate user session via DBus
            let uid_output = tokio::process::Command::new("id")
                .arg("-u")
                .output()
                .await?;

            if !uid_output.status.success() {
                return Err(format!("failed to get current uid: {}", uid_output.status).into());
            }

            let uid_str = String::from_utf8(uid_output.stdout)?
                .trim()
                .to_string();

            let uid: u32 = uid_str.parse()?;

            let connection = Connection::system().await?;
            let proxy = LoginManagerProxy::new(&connection).await?;
            proxy.terminate_user(uid).await?;
            Ok(())
        }
    }
}