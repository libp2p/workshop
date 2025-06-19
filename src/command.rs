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
use tracing::{debug, error};

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
        trace: bool,
    ) -> Result<CommandResult, Error> {
        self.run_command_with_env(cmd, args, working_dir, &[], token, trace)
            .await
    }

    /// Run a command with environment variables and stream output to the Log screen
    ///
    /// This function:
    /// - Shows the Log screen when command starts
    /// - Streams stdout to Log screen (bypassing env filter)
    /// - Logs stderr using error!() macro
    /// - Hides Log screen on success, leaves visible on failure
    pub async fn run_command_with_env(
        &self,
        cmd: &str,
        args: &[&str],
        working_dir: Option<&std::path::Path>,
        env_vars: &[(&str, &str)],
        token: &CancellationToken,
        trace: bool,
    ) -> Result<CommandResult, Error> {
        // Build command
        let mut command = Command::new(cmd);
        command.args(args);

        // Set environment variables
        for (key, value) in env_vars {
            debug!("Setting environment variable: {key}={value}");
            command.env(key, value);
        }

        if let Some(dir) = working_dir {
            debug!("Setting working directory: {}", dir.display());
            command.current_dir(dir);
        } else {
            debug!(
                "No working directory specified, using current directory: {}",
                std::env::current_dir().unwrap().display()
            );
        }

        // Send command info to log screen
        let cmd_info = format!("{cmd} {}", args.join(" "));
        debug!("Running command: {cmd_info}");
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
        let mut child = match command
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(e) => {
                error!("Failed to spawn command '{cmd}': {e}");
                return Err(Error::Command(format!(
                    "Failed to spawn command '{cmd}': {e}"
                )));
            }
        };

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
                                if trace {
                                    self.event_sender
                                        .send((
                                            Some(screens::Screens::Log),
                                            tui::Event::CommandOutput(prev_line, None)
                                        ).into())
                                        .await?;
                                }
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
                                if trace {
                                    self.event_sender
                                        .send((
                                            Some(screens::Screens::Log),
                                            tui::Event::CommandOutput(prev_line, None)
                                        ).into())
                                        .await?;
                                }
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
        let last_line = stdout_line.unwrap_or_else(|| stderr_line.unwrap_or_default());

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
        docker_compose_executable: &str,
        python_executable: &str,
        lesson_dir: &Path,
        token: &CancellationToken,
    ) -> Result<CommandResult, Error> {
        // Calculate PROJECT_ROOT and LESSON_PATH for docker-compose environment
        let (project_root, lesson_path) = self.calculate_docker_env_paths(lesson_dir)?;

        // Set up environment variables for docker-compose
        let env_vars = [
            ("PROJECT_ROOT", project_root.as_str()),
            ("LESSON_PATH", lesson_path.as_str()),
        ];

        // Run docker compose up -d --build
        debug!(
            "Running '{} compose up -d --build' in '{}' with PROJECT_ROOT={project_root} LESSON_PATH={lesson_path}",
            docker_compose_executable,
            lesson_dir.display(),
        );
        let docker_result = self
            .run_command_with_env(
                docker_compose_executable.as_ref(),
                &["compose", "up", "-d", "--build"],
                Some(lesson_dir),
                &env_vars,
                token,
                false,
            )
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
            true,
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
            true,
        )
        .await
    }

    /// Run git to clone a repository to our application data directory
    pub async fn install_workshop(
        &self,
        git_executable: &str,
        repo_url: &str,
        data_dir: &Path,
        token: &CancellationToken,
    ) -> Result<CommandResult, Error> {
        debug!(
            "Running '{} clone {}' into '{}'",
            git_executable,
            repo_url,
            data_dir.display()
        );

        self.run_command(
            git_executable.as_ref(),
            &["clone", "--depth", "1", repo_url],
            Some(data_dir),
            token,
            true,
        )
        .await
    }

    /// Calculate PROJECT_ROOT and LESSON_PATH environment variables for docker-compose
    fn calculate_docker_env_paths(&self, lesson_dir: &Path) -> Result<(String, String), Error> {
        // Find the .workshops directory by going up from lesson_dir
        let mut current = lesson_dir;
        let workshops_dir = loop {
            if current
                .file_name()
                .map(|n| n == ".workshops")
                .unwrap_or(false)
            {
                break current;
            }
            if let Some(parent) = current.parent() {
                current = parent;
            } else {
                return Err(Error::Command(
                    "Could not find .workshops directory".to_string(),
                ));
            }
        };

        // PROJECT_ROOT is the parent of .workshops directory
        let project_root = workshops_dir
            .parent()
            .ok_or_else(|| Error::Command("Could not find PROJECT_ROOT directory".to_string()))?;

        // LESSON_PATH is the relative path from PROJECT_ROOT to lesson_dir
        let lesson_path = lesson_dir
            .strip_prefix(project_root)
            .map_err(|_| Error::Command("Could not calculate LESSON_PATH".to_string()))?;

        Ok((
            project_root.to_string_lossy().to_string(),
            lesson_path.to_string_lossy().to_string(),
        ))
    }
}
