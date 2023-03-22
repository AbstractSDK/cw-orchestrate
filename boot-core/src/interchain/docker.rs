use super::error::InterchainError;
use super::hermes::{Hermes, HERMES_ID};
use super::infrastructure::{ContainerId, Port};
use super::IcResult;

use bollard::models::ContainerSummary;
use bollard::Docker;
use bollard::{
    container::{InspectContainerOptions, ListContainersOptions},
    service::PortMap,
};
use futures_util::stream;
use futures_util::stream::StreamExt;

use std::collections::HashMap;
use std::default::Default;

/// Helper for interacting with the Docker environment
/// contains container information for the current environment
#[derive(Debug)]
pub struct DockerHelper {
    containers: Vec<ContainerSummary>,
    docker: Docker,
}

impl DockerHelper {
    pub async fn new() -> IcResult<Self> {
        // connect to docker
        let docker = Docker::connect_with_socket_defaults().unwrap();
        // get all running containers
        let mut list_container_filters = HashMap::new();
        list_container_filters.insert("status", vec!["running"]);
        let containers = docker
            .list_containers(Some(ListContainersOptions {
                all: true,
                filters: list_container_filters,
                ..Default::default()
            }))
            .await?;
        Ok(Self { containers, docker })
    }
    async fn ports(arg: (Docker, &ContainerSummary)) -> IcResult<(ContainerId, PortMap)> {
        let (docker, container) = arg;

        let port_map = docker
            .inspect_container(
                container.id.as_ref().unwrap(),
                None::<InspectContainerOptions>,
            )
            .await?
            .network_settings
            .unwrap()
            .ports
            .unwrap();

        let name = container
            .names
            .as_ref()
            .unwrap()
            .first()
            .unwrap()
            .to_owned();

        Ok((name, port_map))
    }

    fn map_validator_port(port_map: &PortMap, port: &str) -> String {
        for (container_port, bindings) in port_map {
            if container_port.contains(port) {
                return bindings
                    .as_ref()
                    .unwrap()
                    .first()
                    .unwrap()
                    .host_port
                    .as_ref()
                    .unwrap()
                    .to_owned();
            }
        }
        panic!("No port found for {} given portmap {:#?}", port, port_map);
    }

    /// Get the port mapping for all the running containers
    pub async fn container_ports(&self) -> IcResult<Vec<(ContainerId, PortMap)>> {
        let docker_stream = stream::repeat(self.docker.clone());
        let result_stream = docker_stream
            .zip(stream::iter(&self.containers))
            .map(|(docker_item, container_item)| {
                async move {
                    // Call your function on docker_item and container_item here
                    let result = Self::ports((docker_item, container_item)).await.unwrap();
                    result
                }
            })
            .buffer_unordered(2) // Run up to 2 tasks concurrently
            .boxed();
        let mut results = result_stream.collect::<Vec<_>>().await;
        // only return validator containers
        results = results
            .into_iter()
            .filter(|(id, _)| id.contains("val"))
            .collect();

        Ok(results)
    }

    /// Get the grpc ports for all the running validator containers
    pub async fn grpc_ports(&self) -> IcResult<Vec<(ContainerId, Port)>> {
        Self::container_ports(&self)
            .await
            .map(|container_binded_ports| {
                container_binded_ports
                    .into_iter()
                    .map(|(id, mapping)| (id, Self::map_validator_port(&mapping, "9090")))
                    .collect()
            })
    }

    pub fn get_hermes(&self) -> IcResult<Hermes> {
        self.containers
            .iter()
            .find(|container| {eprintln!("{:?}", container.names); container.names.as_ref().unwrap().first().unwrap().contains(HERMES_ID)})
            .ok_or(InterchainError::HermesContainerNotFound)
            .map(|cs| Hermes::new(cs.clone()))
    }
}
