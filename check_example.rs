use std::process::ExitStatus;
use tokio::process::Command;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

// Message types for communication
#[derive(Debug)]
enum Message {
    StdOut(String),
    StdErr(String),
    Success,
    Failed(i32),
}

// Function to spawn the task
fn spawn_docker_check_task() -> (JoinHandle<()>, Receiver<Message>, CancellationToken) {
    let (tx, rx) = mpsc::channel::<Message>(100);
    let cancel_token = CancellationToken::new();
    let token_clone = cancel_token.clone();
    
    let handle = tokio::spawn(async move {
        if let Err(e) = run_process(tx, token_clone).await {
            eprintln!("Task error: {}", e);
        }
    });
    
    (handle, rx, cancel_token)
}

async fn run_process(tx: Sender<Message>, token: CancellationToken) -> Result<(), Box<dyn std::error::Error>> {
    // Run docker-compose
    let status = run_command("docker-compose", &["up", "-d"], &tx, &token).await?;
    
    if !status.success() {
        let code = status.code().unwrap_or(-1);
        tx.send(Message::Failed(code)).await?;
        return Ok(());
    }
    
    // Run check.py
    let status = run_command("python", &["check.py"], &tx, &token).await?;
    
    if status.success() {
        tx.send(Message::Success).await?;
    } else {
        let code = status.code().unwrap_or(-1);
        tx.send(Message::Failed(code)).await?;
    }
    
    Ok(())
}

async fn run_command(cmd: &str, args: &[&str], tx: &Sender<Message>, token: &CancellationToken) -> Result<ExitStatus, Box<dyn std::error::Error>> {
    let mut command = Command::new(cmd);
    command.args(args);
    
    let mut child = command.stdout(std::process::Stdio::piped())
                           .stderr(std::process::Stdio::piped())
                           .spawn()?;
    
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    
    let tx_clone = tx.clone();
    let stdout_task = tokio::spawn(async move {
        let mut reader = tokio::io::BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            if tx_clone.send(Message::StdOut(line)).await.is_err() {
                break;
            }
        }
    });
    
    let tx_clone = tx.clone();
    let stderr_task = tokio::spawn(async move {
        let mut reader = tokio::io::BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            if tx_clone.send(Message::StdErr(line)).await.is_err() {
                break;
            }
        }
    });
    
    // Create a task to wait for the child process
    let mut status_task = tokio::spawn(async move { child.wait().await });
    
    // Wait for either cancellation or completion
    tokio::select! {
        _ = token.cancelled() => {
            // Try to kill the process
            if let Ok(Some(mut child)) = status_task.await {
                let _ = child.kill().await;
                child.wait().await?
            } else {
                return Err("Failed to get child process".into());
            }
        }
        status = &mut status_task => {
            status?
        }
    };
    
    // Clean up stdout/stderr tasks
    stdout_task.abort();
    stderr_task.abort();
    
    Ok(status?)
}

// Example usage
async fn main_example() {
    let (handle, mut rx, cancel_token) = spawn_docker_check_task();
    
    // Process messages
    while let Some(msg) = rx.recv().await {
        match msg {
            Message::StdOut(line) => println!("STDOUT: {}", line),
            Message::StdErr(line) => eprintln!("STDERR: {}", line),
            Message::Success => {
                println!("Task completed successfully!");
                cancel_token.cancel();
                break;
            }
            Message::Failed(code) => {
                eprintln!("Task failed with exit code: {}", code);
                cancel_token.cancel();
                break;
            }
        }
    }
    
    // Wait for the task to complete
    if let Err(e) = handle.await {
        eprintln!("Error joining task: {}", e);
    }
}