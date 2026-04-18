#[allow(dead_code)]
pub struct User {
    username: String,
    is_admin: bool,
}

impl User {
    pub fn new(username: impl Into<String>) -> Self {
        Self::builder().username(username).build()
    }

    pub fn builder() -> UserBuilder {
        UserBuilder::default()
    }

    fn from_parts(username: String, is_admin: bool) -> Self {
        Self { username, is_admin }
    }
}

#[derive(Default)]
pub struct UserBuilder {
    username: Option<String>,
    is_admin: bool,
}

impl UserBuilder {
    pub fn username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    pub fn build(self) -> User {
        User::from_parts(
            self.username.unwrap_or_else(|| "anonymous".to_string()),
            self.is_admin,
        )
    }
}

pub fn sample_direct_user() -> User {
    User {
        username: "literal-user".to_string(),
        is_admin: false,
    }
}
