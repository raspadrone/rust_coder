use anyhow::{Context, Result};
use bollard::{
    Docker,
    models::{ContainerCreateBody, ContainerSummaryStateEnum, HostConfig, PortBinding},
    query_parameters::{
        CreateContainerOptions, CreateImageOptions, ListContainersOptions, RemoveContainerOptions,
        StartContainerOptions, StopContainerOptions,
    },
};
use futures_util::stream::StreamExt;
use std::{collections::HashMap, env};
use std::default::Default;

const QDRANT_IMAGE: &str = "qdrant/qdrant:latest";
const CONTAINER_NAME: &str = "rust-coder-qdrant";

/// Ensures the Qdrant Docker container is running, creating and starting it if necessary.
pub async fn ensure_qdrant_running(host: String) -> Result<()> {
    // let docker = Docker::connect_with_socket_defaults()?;
    let docker = Docker::connect_with_socket(&host, 120, bollard::API_DEFAULT_VERSION)?;

    if let Some(container) = find_container(&docker, CONTAINER_NAME).await? {
        if container.state != Some(ContainerSummaryStateEnum::RUNNING) {
            println!("INFO: Qdrant container found but not running. Starting...");
            docker
                .start_container(CONTAINER_NAME, None::<StartContainerOptions>)
                .await
                .context("Failed to start existing Qdrant container")?;
            println!("INFO: Qdrant container started.");
        } else {
            println!("INFO: Qdrant container is already running.");
        }
    } else {
        println!("INFO: Qdrant container not found. Creating and starting a new one...");
        create_and_start_qdrant(&docker).await?;
        println!("INFO: New Qdrant container created and started successfully.");
    }

    Ok(())
}

/// Creates and starts a new Qdrant container using the modern bollard API.
async fn create_and_start_qdrant(docker: &Docker) -> Result<()> {
    println!("INFO: Pulling Qdrant image: '{}'...", QDRANT_IMAGE);
    let mut stream = docker.create_image(
        Some(CreateImageOptions {
            from_image: Some(QDRANT_IMAGE.to_string()),
            ..Default::default()
        }),
        None,
        None,
    );
    while let Some(result) = stream.next().await {
        result.context("Failed to pull Qdrant image")?;
    }
    println!("INFO: Qdrant image pulled successfully.");

    let options = Some(CreateContainerOptions {
        name: Some(CONTAINER_NAME.to_string()),
        ..Default::default()
    });

    let mut port_bindings = HashMap::new();
    port_bindings.insert(
        "6333/tcp".to_string(),
        Some(vec![PortBinding {
            host_ip: Some("127.0.0.1".to_string()),
            host_port: Some("6333".to_string()),
        }]),
    );
    port_bindings.insert(
        "6334/tcp".to_string(),
        Some(vec![PortBinding {
            host_ip: Some("127.0.0.1".to_string()),
            host_port: Some("6334".to_string()),
        }]),
    );


    // 1. Get the current directory where the app is running.
    let current_dir = env::current_dir().context("Failed to get current directory")?;

    // 2. Create the full, absolute path for our storage folder.
    let host_storage_path = current_dir.join("qdrant_storage");

    // 3. Format the bind mount string with the absolute path.
    let bind_mount = format!(
        "{}:/qdrant/storage",
        host_storage_path
            .to_str()
            .context("Storage path is not valid UTF-8")?
    );
    let host_config = HostConfig {
        port_bindings: Some(port_bindings),
        binds: Some(vec![bind_mount]),
        ..Default::default()
    };

    // Create the final container configuration for Qdrant.
    let qdrant_config = ContainerCreateBody {
        image: Some(QDRANT_IMAGE.to_string()),
        host_config: Some(host_config),
        ..Default::default()
    };

    let id = docker.create_container(options, qdrant_config).await?.id;

    docker
        .start_container(&id, None::<StartContainerOptions>)
        .await?;

    Ok(())
}

/// Finds a container by name using the modern bollard API.
async fn find_container(
    docker: &Docker,
    name: &str,
) -> Result<Option<bollard::models::ContainerSummary>> {
    // This function is correct and requires no changes.
    let options = Some(ListContainersOptions {
        all: true,
        ..Default::default()
    });
    let containers = docker.list_containers(options).await?;
    Ok(containers.into_iter().find(|c| {
        c.names
            .as_ref()
            .map_or(false, |names| names.contains(&format!("/{}", name)))
    }))
}

pub async fn stop_and_remove_qdrant(host: String) -> Result<()> {
    let docker = Docker::connect_with_socket(&host, 120, bollard::API_DEFAULT_VERSION)?;

    if find_container(&docker, CONTAINER_NAME).await?.is_some() {
        println!("INFO: Stopping container '{}'...", CONTAINER_NAME);

        // Create options to wait up to 10 seconds for a graceful shutdown.
        let options = Some(StopContainerOptions {
            signal: Some("SIGTERM".to_string()), // Specify the signal to send
            t: Some(10),                         // Optional timeout in seconds
        });

        docker
            .stop_container(CONTAINER_NAME, options)
            .await
            .context("Failed to stop Qdrant container")?;

        println!("INFO: Container stopped successfully.");

        // The container is now stopped. Uncomment the following lines
        // if you want to also remove it entirely.
        /*
        println!("INFO: Removing container...");
        docker
            .remove_container(CONTAINER_NAME, None::<RemoveContainerOptions>)
            .await
            .context("Failed to remove Qdrant container")?;
        println!("INFO: Container removed successfully.");
        */
    } else {
        println!("INFO: Qdrant container not found, nothing to do.");
    }

    Ok(())
}
