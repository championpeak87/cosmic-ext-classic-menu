use serde::{Deserialize, Serialize};

pub async fn get_current_user() -> Result<User, zbus::Error> {
    let uid = users::get_current_uid() as u64;

    let conn = zbus::Connection::system().await?;
    let user = accounts_zbus::UserProxy::from_uid(&conn, uid).await?;

    // Fetch all fields concurrently
    let (username, user_realname, profile_picture, uid, user_home, user_shell) = tokio::join!(
        user.user_name(),
        user.real_name(),
        user.icon_file(),
        user.uid(),
        user.home_directory(),
        user.shell()
    );

    Ok(User {
        username: username?,
        user_realname: user_realname?,
        profile_picture: profile_picture?,
        uid: uid?,
        user_home: user_home?,
        user_shell: user_shell?,
    })
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub username: String,
    pub user_realname: String,
    pub profile_picture: String,
    pub uid: u64,
    pub user_home: String,
    pub user_shell: String,
}
