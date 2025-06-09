use crate::{
    ui::tui::{self, screens, widgets::StatusMode},
    Error,
};
use std::path::Path;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    sync::mpsc::Sender,
};
use tokio_util::sync::CancellationToken;
use tracing::error;

/// Result of command execution
#[derive(Debug, Clone)]
pub struct CommandResult {
    pub success: bool,
    pub exit_code: i32,
    pub last_line: String,
}

/// Generic command runner that sends output to the Log screen
#[derive(Clone)]
pub struct CommandRunner {
    event_sender: Sender<screens::Event>,
}

impl CommandRunner {
    /// Create a new CommandRunner
    pub fn new(event_sender: Sender<screens::Event>) -> Self {
        Self { event_sender }
    }

    /// Run a command and stream output to the Log screen
    ///
    /// This function:
    /// - Shows the Log screen when command starts
    /// - Streams stdout to Log screen (bypassing env filter)
    /// - Logs stderr using error!() macro
    /// - Hides Log screen on success, leaves visible on failure
    pub async fn run_command(
        &self,
        cmd: &str,
        args: &[&str],
        working_dir: Option<&std::path::Path>,
        token: &CancellationToken,
    ) -> Result<CommandResult, Error> {
        // Build command
        let mut command = Command::new(cmd);
        command.args(args);

        if let Some(dir) = working_dir {
            command.current_dir(dir);
        }

        // Send command info to log screen
        let cmd_info = format!("{} {}", cmd, args.join(" "));
        self.event_sender
            .send(
                (
                    Some(screens::Screens::Log),
                    tui::Event::CommandStarted(StatusMode::Messages, cmd_info.clone()),
                )
                    .into(),
            )
            .await?;

        // Spawn process with piped stdout/stderr
        let mut child = command
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        // Handle stdout
        let stdout = child.stdout.take().unwrap();
        let stdout_reader = BufReader::new(stdout);
        let mut stdout_lines = stdout_reader.lines();

        // Handle stderr
        let stderr = child.stderr.take().unwrap();
        let stderr_reader = BufReader::new(stderr);
        let mut stderr_lines = stderr_reader.lines();

        // Stream output until process completes or is cancelled
        let mut stdout_finished = false;
        let mut stderr_finished = false;
        let mut stdout_line: Option<String> = None;
        let mut stderr_line: Option<String> = None;

        let exit_status = loop {
            tokio::select! {
                // Handle cancellation
                _ = token.cancelled() => {
                    let _ = child.kill().await;
                    return Err(Error::Command("Command cancelled".to_string()));
                }

                // Read stdout line by line
                line = stdout_lines.next_line(), if !stdout_finished => {
                    match line {
                        Ok(Some(line)) => {
                            if let Some(prev_line) = stdout_line.take() {
                                self.event_sender
                                    .send((
                                        Some(screens::Screens::Log),
                                        tui::Event::CommandOutput(prev_line, None)
                                    ).into())
                                    .await?;
                            }
                            stdout_line = Some(line);
                        }
                        Ok(None) => {
                            // EOF on stdout
                            stdout_finished = true;
                        },
                        Err(e) => {
                            error!("Error reading stdout: {}", e);
                            stdout_finished = true;
                        },
                    }
                }

                // Read stderr line by line
                line = stderr_lines.next_line(), if !stderr_finished => {
                    match line {
                        Ok(Some(line)) => {
                            if let Some(prev_line) = stderr_line.take() {
                                self.event_sender
                                    .send((
                                        Some(screens::Screens::Log),
                                        tui::Event::CommandOutput(prev_line, None)
                                    ).into())
                                    .await?;
                            }
                            stderr_line = Some(line);
                        }
                        Ok(None) => {
                            // EOF on stderr
                            stderr_finished = true;
                        },
                        Err(e) => {
                            error!("Error reading stderr: {}", e);
                            stderr_finished = true;
                        },
                    }
                }

                // Wait for process completion
                status = child.wait() => {
                    break status?;
                }
            }
        };

        let success = exit_status.success();
        let exit_code = exit_status.code().unwrap_or(-1);
        let last_line = stdout_line.unwrap_or_default();

        let result = CommandResult {
            success,
            exit_code,
            last_line: last_line.clone(),
        };

        Ok(result)
    }

    /// Run docker-compose up -d followed by python check.py
    /// This is a convenience method for lesson solution checking
    pub async fn check_solution(
        &self,
        python_executable: &str,
        lesson_dir: &Path,
        token: &CancellationToken,
    ) -> Result<CommandResult, Error> {
        // Run docker-compose up -d
        let docker_result = self
            .run_command("docker-compose", &["up", "-d"], Some(lesson_dir), token)
            .await?;

        if !docker_result.success {
            return Ok(docker_result);
        }

        // Run python check.py
        self.run_command(
            python_executable.as_ref(),
            &["check.py"],
            Some(lesson_dir),
            token,
        )
        .await
    }

    /// Run deps.py script for dependency checking
    pub async fn check_dependencies(
        &self,
        python_executable: &str,
        deps_script: &Path,
        token: &CancellationToken,
    ) -> Result<CommandResult, Error> {
        let script_dir = deps_script
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));

        self.run_command(
            python_executable.as_ref(),
            &[deps_script.to_str().unwrap()],
            Some(script_dir),
            token,
        )
        .await
    }
}
