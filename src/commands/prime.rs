use crate::GritsError;

const PRIMER: &str = r#"# Grits — File Coordination

Before modifying any file, check for conflicts:
  grits check <file>:<symbol>

If clear, claim it:
  grits claim <file>:<symbol>

When done, release it:
  grits release <id> --commit <sha>

Use --json for structured output. Run `grits status` to see all active claims."#;

pub fn run() -> Result<(), GritsError> {
    println!("{PRIMER}");
    Ok(())
}
