use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use tonic::{Request, Response, Status};
use crate::cri::runtime::v1::{
    PodSandboxConfig, PodSandboxMetadata, PortMapping, Protocol,
    ContainerConfig, ContainerMetadata, ImageSpec, LinuxPodSandboxConfig,
    LinuxContainerConfig, Namespace, NamespaceMode,
    RunPodSandboxRequest, CreateContainerRequest, StartContainerRequest,
    RunPodSandboxResponse, CreateContainerResponse, StartContainerResponse,
};

// 模拟 Kubernetes Pod 的元数据
#[derive(Debug, Serialize, Deserialize)]
struct TypeMeta {
    #[serde(rename = "apiVersion")]
    api_version: String,
    #[serde(rename = "kind")]
    kind: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ObjectMeta {
    name: String,
    namespace: String,
    #[serde(default)]
    labels: std::collections::HashMap<String, String>,
    #[serde(default)]
    annotations: std::collections::HashMap<String, String>,
}

// 模拟 Kubernetes PodSpec
#[derive(Debug, Serialize, Deserialize)]
struct PodSpec {
    #[serde(default)]
    containers: Vec<ContainerSpec>,
    #[serde(default)]
    init_containers: Vec<ContainerSpec>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ContainerSpec {
    name: String,
    image: String,
    #[serde(default)]
    ports: Vec<Port>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Port {
    #[serde(rename = "containerPort")]
    container_port: i32,
}

// 任务运行器，基于 Kubernetes Pod 模型
pub struct TaskRunner {
    pub task: PodTask,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PodTask {
    #[serde(rename = "apiVersion")]
    api_version: String,
    #[serde(rename = "kind")]
    kind: String,
    metadata: ObjectMeta,
    spec: PodSpec,
}

impl TaskRunner {
    /// 从文件加载并解析 YAML
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let task: PodTask = serde_yaml::from_str(&contents)?;
        Ok(TaskRunner { task })
    }

    /// 创建 PodSandboxConfig，包含 Namespace 配置
    fn create_pod_sandbox_config(&self) -> PodSandboxConfig {
        let metadata = PodSandboxMetadata {
            name: self.task.metadata.name.clone(),
            namespace: self.task.metadata.namespace.clone(),
            uid: "12345".to_string(),
            attempt: 0,
        };

        let port_mappings = self.task.spec.containers
            .iter()
            .flat_map(|c| c.ports.iter().map(|p| PortMapping {
                protocol: Protocol::Tcp as i32,
                container_port: p.container_port,
                host_port: 0,
                host_ip: "".to_string(),
            }))
            .collect();

        PodSandboxConfig {
            metadata: Some(metadata),
            hostname: self.task.metadata.name.clone(),
            log_directory: format!("/var/log/pods/{}_{}/", self.task.metadata.namespace, self.task.metadata.name),
            dns_config: None,
            port_mappings,
            labels: self.task.metadata.labels.clone(),
            annotations: self.task.metadata.annotations.clone(),
            linux: Some(LinuxPodSandboxConfig {
                namespaces: vec![
                    Namespace {
                        r#type: "network".to_string(),
                        mode: NamespaceMode::Pod as i32,
                        path: "".to_string(),
                    },
                    Namespace {
                        r#type: "pid".to_string(),
                        mode: NamespaceMode::Pod as i32,
                        path: "".to_string(),
                    },
                    Namespace {
                        r#type: "ipc".to_string(),
                        mode: NamespaceMode::Pod as i32,
                        path: "".to_string(),
                    },
                    Namespace {
                        r#type: "mount".to_string(),
                        mode: NamespaceMode::Pod as i32,
                        path: "".to_string(),
                    },
                ],
                ..Default::default()
            }),
            windows: None,
        }
    }

    /// 创建 ContainerConfig，包含 Namespace 配置
    fn create_container_config(&self, pod_sandbox_id: &str, container: &ContainerSpec) -> ContainerConfig {
        ContainerConfig {
            metadata: Some(ContainerMetadata {
                name: container.name.clone(),
                attempt: 0,
            }),
            image: Some(ImageSpec {
                image: container.image.clone(),
                annotations: std::collections::HashMap::new(),
                user_specified_image: container.image.clone(),
                runtime_handler: "".to_string(),
            }),
            command: vec![],
            args: vec![],
            working_dir: "".to_string(),
            envs: vec![],
            mounts: vec![],
            devices: vec![],
            labels: self.task.metadata.labels.clone(),
            annotations: self.task.metadata.annotations.clone(),
            log_path: format!("{}/0.log", container.name),
            stdin: false,
            stdin_once: false,
            tty: false,
            linux: Some(LinuxContainerConfig {
                namespaces: vec![
                    Namespace {
                        r#type: "network".to_string(),
                        mode: NamespaceMode::Container as i32,
                        path: pod_sandbox_id.to_string(),
                    },
                    Namespace {
                        r#type: "pid".to_string(),
                        mode: NamespaceMode::Container as i32,
                        path: pod_sandbox_id.to_string(),
                    },
                    Namespace {
                        r#type: "ipc".to_string(),
                        mode: NamespaceMode::Container as i32,
                        path: pod_sandbox_id.to_string(),
                    },
                    Namespace {
                        r#type: "mount".to_string(),
                        mode: NamespaceMode::Container as i32,
                        path: pod_sandbox_id.to_string(),
                    },
                ],
                ..Default::default()
            }),
            windows: None,
        }
    }

    /// 构造 RunPodSandboxRequest
    pub fn build_run_pod_sandbox_request(&self) -> RunPodSandboxRequest {
        RunPodSandboxRequest {
            config: Some(self.create_pod_sandbox_config()),
            runtime_handler: "".to_string(),
        }
    }

    /// 构造 CreateContainerRequest
    pub fn build_create_container_request(&self, pod_sandbox_id: &str, container: &ContainerSpec) -> CreateContainerRequest {
        CreateContainerRequest {
            pod_sandbox_id: pod_sandbox_id.to_string(),
            config: Some(self.create_container_config(pod_sandbox_id, container)),
            sandbox_config: Some(self.create_pod_sandbox_config()),
        }
    }

    /// 构造 StartContainerRequest
    pub fn build_start_container_request(&self, container_id: &str) -> StartContainerRequest {
        StartContainerRequest {
            container_id: container_id.to_string(),
        }
    }

    /// 运行任务：启动 PodSandbox 和多个容器
    pub async fn run<T: cri::runtime::v1::runtime_service_server::RuntimeService>(
        &self,
        runtime: &T,
    ) -> Result<(String, Vec<String>), Status> {
        // 显式日志：镜像拉取和 bundle 准备
        println!("Pulling image(s): {:?}", self.task.spec.containers.iter().map(|c| &c.image).collect::<Vec<&String>>());
        println!("Getting bundle for PodSandbox...");

        // 启动 PodSandbox
        let pod_request = self.build_run_pod_sandbox_request();
        let pod_response = runtime.run_pod_sandbox(Request::new(pod_request)).await?;
        let pod_sandbox_id = pod_response.into_inner().pod_sandbox_id;
        println!("PodSandbox started: {}", pod_sandbox_id);

        // 创建并启动容器（跳过 init_containers，假设由用户手动处理）
        let mut container_ids = Vec::new();
        for container in &self.task.spec.containers {
            let container_request = self.build_create_container_request(&pod_sandbox_id, container);
            let container_response = runtime.create_container(Request::new(container_request)).await?;
            let container_id = container_response.into_inner().container_id;
            println!("Container created: {}", container_id);

            let start_request = self.build_start_container_request(&container_id);
            runtime.start_container(Request::new(start_request)).await?;
            println!("Container started: {}", container_id);
            container_ids.push(container_id);
        }

        Ok((pod_sandbox_id, container_ids))
    }
}