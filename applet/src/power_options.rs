use zbus::Connection;

use crate::{cosmic_session::CosmicSessionProxy, session_manager::SessionManagerProxy};

/// The actions going through `logind-zbus`, which is still on zbus 5.
mod logind {
    use logind_zbus::{
        manager::{IsSupported, ManagerProxy},
        session::{SessionClass, SessionProxy, SessionType},
        user::UserProxy,
    };
    use rustix::process::getuid;
    use zbus5 as zbus;
    use zbus5::Connection;

    pub async fn restart() -> zbus::Result<()> {
        let connection = Connection::system().await?;
        let manager_proxy = ManagerProxy::new(&connection).await?;
        manager_proxy.reboot(true).await
    }

    pub async fn shutdown() -> zbus::Result<()> {
        let connection = Connection::system().await?;
        let manager_proxy = ManagerProxy::new(&connection).await?;
        manager_proxy.power_off(true).await
    }

    pub async fn suspend() -> zbus::Result<()> {
        let connection = Connection::system().await?;
        let manager_proxy = ManagerProxy::new(&connection).await?;
        manager_proxy.suspend(true).await
    }

    pub async fn hibernate() -> zbus::Result<()> {
        let connection = Connection::system().await?;
        let manager_proxy = ManagerProxy::new(&connection).await?;
        manager_proxy.hibernate(true).await
    }

    /// Whether logind can hibernate, perhaps after asking for permission.
    pub async fn can_hibernate() -> zbus::Result<bool> {
        let connection = Connection::system().await?;
        let manager_proxy = ManagerProxy::new(&connection).await?;
        Ok(matches!(
            manager_proxy.can_hibernate().await?,
            IsSupported::Yes | IsSupported::Challenge
        ))
    }

    pub async fn lock() -> zbus::Result<()> {
        let connection = Connection::system().await?;
        let manager_proxy = ManagerProxy::new(&connection).await?;
        // Get the session this current process is running in
        let our_uid = getuid().as_raw() as u32;
        let user_path = manager_proxy.get_user(our_uid).await?;
        let user = UserProxy::builder(&connection)
            .path(user_path)?
            .build()
            .await?;
        // Lock all non-TTY sessions of this user
        let sessions = user.sessions().await?;
        let mut locked_successfully = false;
        for (_, session_path) in sessions {
            let Ok(session) = SessionProxy::builder(&connection)
                .path(session_path)?
                .build()
                .await
            else {
                continue;
            };

            if session.class().await == Ok(SessionClass::User)
                && session.type_().await? != SessionType::TTY
                && session.lock().await.is_ok()
            {
                locked_successfully = true;
            }
        }

        if locked_successfully {
            Ok(())
        } else {
            Err(zbus::Error::Failure("locking session failed".to_string()))
        }
    }
}

fn from_zbus5(error: zbus5::Error) -> zbus::Error {
    zbus::Error::Failure(error.to_string())
}

pub async fn restart() -> zbus::Result<()> {
    logind::restart().await.map_err(from_zbus5)
}

pub async fn shutdown() -> zbus::Result<()> {
    logind::shutdown().await.map_err(from_zbus5)
}

pub async fn suspend() -> zbus::Result<()> {
    logind::suspend().await.map_err(from_zbus5)
}

pub async fn hibernate() -> zbus::Result<()> {
    logind::hibernate().await.map_err(from_zbus5)
}

/// Whether the system can hibernate: it has the swap for it and the user
/// may, perhaps after authenticating. Unknown, such as without logind, it
/// cannot.
pub async fn can_hibernate() -> bool {
    match logind::can_hibernate().await {
        Ok(can) => can,
        Err(err) => {
            log::warn!("Failed to check whether the system can hibernate: {err}");
            false
        }
    }
}

pub async fn lock() -> zbus::Result<()> {
    logind::lock().await.map_err(from_zbus5)
}

pub async fn log_out() -> zbus::Result<()> {
    // TODO: Fix imports
    let session_type = std::env::var("XDG_CURRENT_DESKTOP").ok();
    let connection = Connection::session().await?;
    match session_type.as_ref().map(|s| s.trim()) {
        Some("pop:GNOME") => {
            let manager_proxy = SessionManagerProxy::new(&connection).await?;
            manager_proxy.logout(0).await?;
        }
        // By default assume COSMIC
        _ => {
            let cosmic_session = CosmicSessionProxy::new(&connection).await?;
            cosmic_session.exit().await?;
        }
    }
    Ok(())
}