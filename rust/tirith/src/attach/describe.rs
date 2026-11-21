//! The attachments as the manager reads them in the owner's message.

/// The text appended to the owner's message so the manager can open the attached files (absolute paths).
pub fn describe(paths: &[String]) -> String {
    paths.is_empty().then(String::new).unwrap_or_else(|| {
        format!(
            "\n\nAttached files (open them with your file-reading tool):{}",
            paths.iter().map(|p| format!("\n- {p}")).collect::<String>()
        )
    })
}
