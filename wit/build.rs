use std::io;
use std::process::{ExitStatus, Command};

fn main() {
    install_node_packages()
        .and_then(|_| {
            build_tailwind()
        })
        .expect("Failed to build tailwindcss...");
}

#[cfg(target_os = "windows")]
fn install_node_packages() -> io::Result<ExitStatus> {
    Command::new("yarn.cmd")
        .arg("install")
        .status()
}

#[cfg(target_os = "windows")]
fn build_tailwind() -> io::Result<ExitStatus> {
    Command::new("yarn.cmd")
        .arg("run")
        .arg("build")
        .status()
}

#[cfg(target_os = "linux")]
fn install_node_packages() -> io::Result<ExitStatus> {
    Command::new("yarn")
        .arg("install")
        .status()
}

#[cfg(target_os = "linux")]
fn build_tailwind() -> io::Result<ExitStatus> {
    Command::new("yarn")
        .arg("run")
        .arg("build")
        .status()
}
