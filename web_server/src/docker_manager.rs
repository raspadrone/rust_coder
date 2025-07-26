// use anyhow::{Context, Result};

// use anyhow::anyhow;
// use bollard::Docker;
// use bollard::query_parameters::{
//     CreateContainerOptions, CreateImageOptions, CreateImageOptionsBuilder, ListContainersOptions,
//     StartContainerOptions,
// };
// use bollard::secret::{Config, ContainerCreateBody, ContainerSummaryStateEnum, PortBinding};
// use futures_util::TryStreamExt;
// use futures_util::stream::StreamExt;
// use std::collections::HashMap;
// use std::default::Default;

// const QDRANT_IMAGE: &str = "qdrant/qdrant:latest";
// const CONTAINER_NAME: &str = "rust-coder-qdrant";

// /// Ensures the Qdrant Docker container is running, creating and starting it if necessary.
// pub async fn ensure_qdrant_running() -> Result<()> {
//     let docker = Docker::connect_with_socket_defaults()?;

//     if let Some(container) = find_container(&docker, CONTAINER_NAME).await? {
//         // Correctly check against the enum variant for the container state
//         if container.state != Some(ContainerSummaryStateEnum::RUNNING) {
//             println!("INFO: Qdrant container found but not running. Starting...");
//             docker
//                 .start_container(CONTAINER_NAME, None::<StartContainerOptions>)
//                 .await
//                 .context("Failed to start existing Qdrant container")?;
//             println!("INFO: Qdrant container started.");
//         } else {
//             println!("INFO: Qdrant container is already running.");
//         }
//     } else {
//         println!("INFO: Qdrant container not found. Creating and starting a new one...");
//         create_and_start_qdrant(&docker).await?;
//         println!("INFO: New Qdrant container created and started successfully.");
//     }

//     Ok(())
// }

// /// Creates and starts a new Qdrant container using the modern bollard API.
// async fn create_and_start_qdrant(docker: &Docker) -> Result<()> {
//     println!("INFO: Pulling Qdrant image: '{}'...", QDRANT_IMAGE);
//     let mut stream = docker.create_image(
//         Some(
//             bollard::query_parameters::CreateImageOptionsBuilder::default()
//                 .from_image(QDRANT_IMAGE)
//                 .build(),
//         ),
//         None,
//         None,
//     );
//     while let Some(result) = stream.next().await {
//         result.context("Failed to pull Qdrant image")?;
//     }
//     println!("INFO: Qdrant image pulled successfully.");

//     let options = Some(CreateContainerOptions {
//         name: Some(CONTAINER_NAME.to_string()),
//         ..Default::default()
//     });

//     let mut port_bindings = HashMap::new();
//     port_bindings.insert(
//         "6333/tcp".to_string(),
//         Some(vec![PortBinding {
//             host_ip: Some("127.0.0.1".to_string()),
//             host_port: Some("6333".to_string()),
//         }]),
//     );
//     port_bindings.insert(
//         "6334/tcp".to_string(),
//         Some(vec![PortBinding {
//             host_ip: Some("127.0.0.1".to_string()),
//             host_port: Some("6334".to_string()),
//         }]),
//     );

//     // // Use the non-deprecated `Config` struct for the container body
//     // let config = Config {
//     //     image: Some(QDRANT_IMAGE),
//     //     host_config: Some(HostConfig {
//     //         port_bindings: Some(port_bindings),
//     //         ..Default::default()
//     //     }),
//     //     ..Default::default()
//     // };

//     // docker.create_container(options, config).await?;

//     // docker
//     //     .start_container(CONTAINER_NAME, None::<StartContainerOptions>)
//     //     .await?;

//     let alpine_config = ContainerCreateBody {
//         image: Some(QDRANT_IMAGE.to_string()),
//         tty: Some(true),
//         attach_stdin: Some(true),
//         attach_stdout: Some(true),
//         attach_stderr: Some(true),
//         open_stdin: Some(true),
//         ..Default::default()
//     };

//     let id = docker
//         .create_container(
//             None::<bollard::query_parameters::CreateContainerOptions>,
//             alpine_config,
//         )
//         .await?
//         .id;
//     docker
//         .start_container(
//             &id,
//             None::<bollard::query_parameters::StartContainerOptions>,
//         )
//         .await?;

//     Ok(())
// }

// /// Finds a container by name using the modern bollard API.
// async fn find_container(
//     docker: &Docker,
//     name: &str,
// ) -> Result<Option<bollard::models::ContainerSummary>> {
//     // Use the correct non-deprecated `ListContainersOptions`
//     let options = Some(ListContainersOptions {
//         all: true,
//         ..Default::default()
//     });
//     let containers = docker.list_containers(options).await?;
//     Ok(containers.into_iter().find(|c| {
//         c.names
//             .as_ref()
//             .map_or(false, |names| names.contains(&format!("/{}", name)))
//     }))
// }


use anyhow::{Context, Result};
use bollard::{
    models::{ContainerCreateBody, ContainerSummaryStateEnum, HostConfig, PortBinding}, query_parameters::{CreateContainerOptions, CreateImageOptions, ListContainersOptions, StartContainerOptions}, Docker
};
use futures_util::stream::StreamExt;
use std::collections::HashMap;
use std::default::Default;

const QDRANT_IMAGE: &str = "qdrant/qdrant:latest";
const CONTAINER_NAME: &str = "rust-coder-qdrant";

/// Ensures the Qdrant Docker container is running, creating and starting it if necessary.
pub async fn ensure_qdrant_running() -> Result<()> {
    // This function is correct and needs no changes.
    let docker = Docker::connect_with_socket_defaults()?;

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

    let host_config = HostConfig {
        port_bindings: Some(port_bindings),
        ..Default::default()
    };

    // Create the final container configuration for Qdrant.
    let qdrant_config = ContainerCreateBody {
        image: Some(QDRANT_IMAGE.to_string()),
        host_config: Some(host_config), // Add the host config here
        ..Default::default()
    };

    let id = docker
        .create_container(options, qdrant_config)
        .await?
        .id;

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
    // This function is correct and needs no changes.
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