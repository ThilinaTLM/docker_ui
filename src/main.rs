use bollard::container::{ListContainersOptions, StartContainerOptions, StopContainerOptions};
use bollard::secret::ContainerSummary;
use bollard::Docker;
use tokio::runtime::Runtime;
use env_logger;

slint::include_modules!();


impl From<String> for ContainerStatus {
    fn from(status: String) -> Self {
        match status.as_str() {
            "running" => ContainerStatus::Running,
            "exited" => ContainerStatus::Exited,
            _ => ContainerStatus::Unknown,
        }
    }
}

impl From<ContainerSummary> for ContainerData {
    fn from(value: ContainerSummary) -> Self {
        let container_id = value.id.unwrap_or_default();
        let short_id = container_id.chars().take(12).collect::<String>();
        let name = value.names.unwrap_or_default().join(", ");
        let name = name.trim_start_matches('/');
        return ContainerData {
            id: container_id.into(),
            short_id: short_id.into(),
            name: name.into(),
            status: value.state.unwrap_or_default().into(),
        }
    }
}

async fn get_docker_containers() -> Vec<ContainerData> {
    let docker = Docker::connect_with_local_defaults().unwrap();
    let options = Some(ListContainersOptions::<String> {
        all: true,
        ..Default::default()
    });
    let containers = docker.list_containers(options).await.unwrap();
    containers.into_iter().map(ContainerData::from).collect()
}

async fn run_container(container_id: String) {
    let docker = Docker::connect_with_local_defaults().unwrap();
    let options = StartContainerOptions::<String>::default();
    if let Err(e) = docker.start_container(&container_id, Some(options)).await {
        log::error!("Failed to stop container {}: {}", container_id, e);
    }
}

async fn stop_container(container_id: String) {
    let docker = Docker::connect_with_local_defaults().unwrap();
    let options = StopContainerOptions { t: 10 };
    if let Err(e) = docker.stop_container(&container_id, Some(options)).await {
        log::error!("Failed to stop container {}: {}", container_id, e);
    }
}

fn main() -> Result<(), slint::PlatformError> {
    env_logger::init();
    let ui = AppWindow::new()?;
    
    let ui_weak = ui.as_weak();
    ui.on_refresh_containers(move || {
        let runtime = Runtime::new().unwrap();
        let containers: Vec<ContainerData> = runtime.block_on(get_docker_containers());
        let container_model = std::rc::Rc::new(slint::VecModel::from(containers));
        let ui = ui_weak.upgrade().unwrap();
        ui.set_containers(container_model.into());
    });

    ui.on_run_container(move |id| {
        let runtime = Runtime::new().unwrap();
        log::info!("Running container {}", id);
        runtime.block_on(run_container(id.into()));
    });

    ui.on_stop_container(move |id| {
        let runtime = Runtime::new().unwrap();
        log::info!("Stopping container {}", id);
        runtime.block_on(stop_container(id.into()));
    });

    let ui_weak = ui.as_weak();
    let timer = slint::Timer::default();
    timer.start(slint::TimerMode::Repeated, std::time::Duration::from_secs(1), move || {
        let ui = ui_weak.upgrade().unwrap();
        ui.invoke_refresh_containers();
        log::info!("refreshing containers complete.");
    });

    ui.run()
}
