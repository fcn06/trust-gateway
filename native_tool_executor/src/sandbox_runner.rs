use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerSandboxConfig {
    pub image: String,
    pub read_only_rootfs: bool,
    pub drop_capabilities: Vec<String>,
    pub seccomp_profile: String,
    pub cpu_limit: f32,
    pub memory_limit_mb: u64,
}

impl Default for ContainerSandboxConfig {
    fn default() -> Self {
        Self {
            image: "docker.io/library/alpine:latest".to_string(),
            read_only_rootfs: true,
            drop_capabilities: vec!["ALL".to_string()],
            seccomp_profile: "default.json".to_string(),
            cpu_limit: 0.5,
            memory_limit_mb: 256,
        }
    }
}

pub struct ContainerSandboxRunner {
    config: ContainerSandboxConfig,
}

impl ContainerSandboxRunner {
    pub fn new(config: ContainerSandboxConfig) -> Self {
        Self { config }
    }

    /// Prepares OCI container execution command vector (Podman / Docker rootless mode).
    pub fn build_container_args(&self, script_path: &str, input_json: &str) -> Vec<String> {
        let mut cmd = vec![
            "podman".to_string(),
            "run".to_string(),
            "--rm".to_string(),
            "--network=none".to_string(),
            format!("--memory={}m", self.config.memory_limit_mb),
            format!("--cpus={:.2}", self.config.cpu_limit),
        ];

        if self.config.read_only_rootfs {
            cmd.push("--read-only".to_string());
        }

        for cap in &self.config.drop_capabilities {
            cmd.push(format!("--cap-drop={}", cap));
        }

        cmd.push(self.config.image.clone());
        cmd.push("/bin/sh".to_string());
        cmd.push("-c".to_string());
        cmd.push(format!("echo '{}' | {}", input_json.replace('\'', "'\\''"), script_path));

        cmd
    }
}
