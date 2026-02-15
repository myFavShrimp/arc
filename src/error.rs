#[derive(thiserror::Error)]
pub struct ErrorReport(Box<dyn std::error::Error>);

impl ErrorReport {
    pub fn boxed_from<E>(value: E) -> Self
    where
        E: std::error::Error + 'static,
    {
        Self(Box::new(value))
    }

    pub fn build_report(&self) -> String {
        let error = &self.0;

        let mut message = error.to_string();
        let mut curr_err = error.source();

        while let Some(current_error) = curr_err {
            message.push_str("\nCaused by:");
            message.push_str(&format!("\n    {}", current_error));
            curr_err = current_error.source();
        }

        message
    }
}

impl std::fmt::Display for ErrorReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.build_report())
    }
}

impl std::fmt::Debug for ErrorReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.build_report())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to lock mutex")]
pub struct MutexLockError;
