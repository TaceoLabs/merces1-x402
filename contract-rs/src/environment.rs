/// The environment the service is running in.
///
/// Main usage for the `Environment` is to call
/// [`Environment::assert_is_dev`]. Services that are intended
/// for `dev` only (like local secret-manager,...)
/// shall assert that they are called from the `dev` environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(
    clippy::exhaustive_enums,
    reason = "We only expect those four environments at the moment. Changing that is a breaking change."
)]
pub enum Environment {
    /// Production environment.
    Prod,
    /// Staging environment.
    Stage,
    /// Test environment. Used for deployed test nets not for local testing. Use `Dev` instead for local testing.
    Test,
    /// Local dev environment.
    Dev,
}

impl core::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Environment::Prod => "prod",
            Environment::Stage => "stage",
            Environment::Test => "test",
            Environment::Dev => "dev",
        };
        f.write_str(str)
    }
}

impl Environment {
    /// Asserts that the environment is the dev environment.
    ///
    /// # Panics
    ///
    /// Panics with `"Is not dev environment"` if `self` is not `Environment::Dev`.
    pub fn assert_is_dev(&self) {
        assert!(self.is_dev(), "Is not dev environment");
    }

    /// Returns `true` if the environment is the test environment.
    #[must_use]
    pub fn is_dev(&self) -> bool {
        matches!(self, Environment::Dev)
    }

    /// Returns `true` if the environment is not the test environment.
    #[must_use]
    pub fn is_not_dev(&self) -> bool {
        !self.is_dev()
    }
}
