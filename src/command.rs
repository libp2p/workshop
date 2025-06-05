use crate::{
    evt,
    ui::tui::{
        events::Event as TuiEvent,
        screens::{self, Screens},
    },
};
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
    ) -> Result<CommandResult, Box<dyn std::error::Error + Send + Sync>> {
        // Show Log screen when command starts
        self.event_sender
            .send(evt!(Screens::Log, TuiEvent::CommandStarted).into())
            .await?;

        // Build command
        let mut command = Command::new(cmd);
        command.args(args);

        if let Some(dir) = working_dir {
            command.current_dir(dir);
        }

        // Send command info to log
        let cmd_info = format!("Running: {} {}", cmd, args.join(" "));
        self.event_sender
            .send(evt!(Screens::Log, TuiEvent::CommandOutput(cmd_info)).into())
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

        let exit_status = loop {
            tokio::select! {
                // Handle cancellation
                _ = token.cancelled() => {
                    let _ = child.kill().await;
                    return Err("Command cancelled".into());
                }

                // Read stdout line by line
                line = stdout_lines.next_line(), if !stdout_finished => {
                    match line {
                        Ok(Some(line)) => {
                            // Send each stdout line directly to Log screen (bypassing env filter)
                            self.event_sender
                                .send(evt!(
                                    Screens::Log,
                                    TuiEvent::CommandOutput(line)
                                ).into())
                                .await?;
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
                            // Log each stderr line using error!() macro
                            error!("{}", line);
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

        let result = CommandResult { success, exit_code };

        // Send completion event
        self.event_sender
            .send(evt!(Screens::Log, TuiEvent::CommandCompleted { success }).into())
            .await?;

        Ok(result)
    }

    /// Run docker-compose up -d followed by python check.py
    /// This is a convenience method for lesson solution checking
    pub async fn check_solution(
        &self,
        lesson_dir: &std::path::Path,
        token: &CancellationToken,
    ) -> Result<CommandResult, Box<dyn std::error::Error + Send + Sync>> {
        // Run docker-compose up -d
        let docker_result = self
            .run_command("docker-compose", &["up", "-d"], Some(lesson_dir), token)
            .await?;

        if !docker_result.success {
            return Ok(docker_result);
        }

        // Run python check.py
        self.run_command("python", &["check.py"], Some(lesson_dir), token)
            .await
    }

    /// Run deps.py script for dependency checking
    pub async fn check_dependencies(
        &self,
        deps_script: &std::path::Path,
        token: &CancellationToken,
    ) -> Result<CommandResult, Box<dyn std::error::Error + Send + Sync>> {
        let script_dir = deps_script
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));

        self.run_command(
            "python",
            &[deps_script.to_str().unwrap()],
            Some(script_dir),
            token,
        )
        .await
    }
}
