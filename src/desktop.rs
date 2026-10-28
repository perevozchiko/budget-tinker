//! Native desktop operations. macOS paths are retained; unsupported dialogs fail explicitly.
use std::path::{Path, PathBuf};
use std::process::Command;

type Result<T> = std::result::Result<T, String>;

pub fn choose_file(save: bool, extension: &str, prompt: &str) -> Result<Option<String>> {
    #[cfg(target_os = "macos")]
    {
        let script = if save {
            format!(
                "POSIX path of (choose file name with prompt \"{prompt}\" default name \"BudgetTinker.{extension}\")"
            )
        } else {
            format!("POSIX path of (choose file with prompt \"{prompt}\")")
        };
        apple_dialog(&script)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (save, extension, prompt);
        Err("Native file dialogs are currently available on macOS only. See the platform limitations in the README.".into())
    }
}

pub fn choose_budget_folder() -> Result<Option<String>> {
    #[cfg(target_os = "macos")]
    {
        apple_dialog("POSIX path of (choose folder with prompt \"Choose a legacy budget folder\")")
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("The folder chooser is currently available on macOS only. Enter the legacy budget folder path directly.".into())
    }
}

#[cfg(target_os = "macos")]
fn apple_dialog(script: &str) -> Result<Option<String>> {
    let output = Command::new("/usr/bin/osascript")
        .args(["-e", script])
        .output()
        .map_err(|e| format!("Could not open the native dialog: {e}"))?;
    if output.status.success() {
        Ok(Some(String::from_utf8_lossy(&output.stdout).trim().into()))
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        if error.contains("(-128)") {
            Ok(None) // User cancellation is not an application error.
        } else {
            Err(format!("Native dialog failed: {error}"))
        }
    }
}

fn reveal_command(path: &Path) -> (String, Vec<PathBuf>) {
    #[cfg(target_os = "macos")]
    {
        ("open".into(), vec![PathBuf::from("-R"), path.into()])
    }
    #[cfg(target_os = "windows")]
    {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        ("explorer.exe".into(), vec![parent.into()])
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        ("xdg-open".into(), vec![parent.into()])
    }
}

pub fn reveal_budget(path: &Path) -> Result<()> {
    let (program, arguments) = reveal_command(path);
    Command::new(program)
        .args(arguments)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Could not open the budget folder: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reveal_passes_paths_as_arguments_without_a_shell() {
        let path = Path::new("budgets/Family Budget; $(keep).budgettinker");
        let (program, arguments) = reveal_command(path);
        #[cfg(target_os = "macos")]
        {
            assert_eq!(program, "open");
            assert_eq!(arguments, [PathBuf::from("-R"), path.into()]);
        }
        #[cfg(target_os = "windows")]
        {
            assert_eq!(program, "explorer.exe");
            assert_eq!(arguments, [PathBuf::from("budgets")]);
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            assert_eq!(program, "xdg-open");
            assert_eq!(arguments, [PathBuf::from("budgets")]);
        }
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn unsupported_dialogs_return_actionable_errors_without_launching_commands() {
        assert!(
            choose_file(true, "json", "Save file")
                .unwrap_err()
                .contains("macOS")
        );
        assert!(
            choose_budget_folder()
                .unwrap_err()
                .contains("Enter the legacy budget folder path")
        );
    }
}
