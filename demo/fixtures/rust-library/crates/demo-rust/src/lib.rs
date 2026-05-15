// @claim demo.rust.greeting
// @claim demo.rust.empty-name
pub fn greeting(name: &str) -> String {
    let trimmed = name.trim();
    let who = if trimmed.is_empty() { "friend" } else { trimmed };
    format!("hello, {who}")
}
