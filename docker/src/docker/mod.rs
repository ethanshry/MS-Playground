use bollard::image::ListImagesOptions;
use bollard::{
    container::{
        Config, CreateContainerOptions, ListContainersOptions, PruneContainersOptions,
        StartContainerOptions, StopContainerOptions,
    },
    image::{BuildImageOptions, PruneImagesOptions},
    service::{HostConfig, PortBinding},
    Docker,
};
use flate2::write::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use futures_util::stream::StreamExt;
use log::{error, info};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Read;
use tar::Archive;
use uuid::Uuid;

pub mod docker_container;

use docker_container::DockerContainer;

/// The interface between Kraken and Docker
pub struct DockerBroker {
    /// Connection to the Rabbit Instance (Should be one per device)
    pub conn: bollard::Docker,
}

impl DockerBroker {
    pub async fn new() -> Option<DockerBroker> {
        let conn = Docker::connect_with_unix_defaults();
        match conn {
            Ok(c) => {
                let version = c.version().await;
                info!("Docker {} connection established", version.unwrap().version);
                Some(DockerBroker { conn: c })
            }
            Err(e) => {
                error!("Error establishing conn: {:?}", e);
                None
            }
        }
    }

    pub async fn get_image_ids(&self) -> Vec<String> {
        let images = self
            .conn
            .list_images(Some(ListImagesOptions::<String> {
                all: true,
                ..Default::default()
            }))
            .await
            .unwrap();

        let mut ids = vec![];

        for image in images {
            println!("-> {:?}", image);
            ids.push(image.id);
        }
        ids
    }

    pub async fn get_running_containers(&self) -> Vec<DockerContainer> {
        let cs = self
            .conn
            .list_containers(Some(ListContainersOptions::<String> {
                ..Default::default()
            }))
            .await
            .unwrap();

        let mut containers = vec![];

        for c in cs {
            let id = c.id.unwrap();
            let name = match c.names {
                // Get first name from vector and drop first character (they all start with /?)
                Some(names) => String::from(&names[0][1..]),
                None => String::from(""),
            };
            let ports = match c.ports {
                Some(ps) => {
                    let mut v = vec![];
                    for p in ps {
                        if let Some(port) = p.public_port {
                            v.push(port);
                        }
                    }
                    Some(v)
                }
                None => None,
            };
            containers.push(DockerContainer::new(
                id, name, c.image, c.image_id, c.created, ports, c.state, c.status,
            ));
        }

        containers
    }

    pub async fn build_image(&self, source_path: &str) -> Result<DockerImageBuildResult, String> {
        let container_guid = Uuid::new_v4().to_hyphenated().to_string();
        // tar the directory
        let make_tar = || -> Result<(), std::io::Error> {
            // Create directory tree if it doesn't exist
            fs::create_dir_all("./tmp/containers")?;
            let tar_gz = File::create(format!("./tmp/containers/{}.tar.gz", &container_guid))?;
            let enc = GzEncoder::new(tar_gz, Compression::default());
            let mut tar = tar::Builder::new(enc);
            tar.append_dir_all(".", source_path)?;
            tar.into_inner()?;
            Ok(())
        };
        match make_tar() {
            Ok(_) => {
                info!("Tar for {} completed succesfully", source_path);
                let mut log = vec![];
                let build_result: Result<(), String> = async {
                    let mut file =
                        File::open(format!("./tmp/containers/{}.tar.gz", &container_guid))
                            .expect("Could not find tarball");
                    let mut contents = Vec::new();
                    file.read_to_end(&mut contents)
                        .expect("Failed to read tarball");

                    info!("Building docker image [{}]", &container_guid);

                    let mut build_results = self.conn.build_image(
                        BuildImageOptions {
                            dockerfile: "Dockerfile",
                            t: &container_guid,
                            rm: true,
                            ..Default::default()
                        },
                        None,
                        Some(contents.into()),
                    );

                    while let Some(result) = build_results.next().await {
                        if let Ok(stage) = result {
                            match stage {
                                bollard::image::BuildImageResults::BuildImageStream { stream } => {
                                    let data = str::replace(&stream, "\n", "");
                                    if data.len() > 0 {
                                        log.push(data);
                                    }
                                }
                                _ => {
                                    // TODO figure out what to do with other results
                                    // BuildImageAux is called right before the final Stream message
                                }
                            }
                        } else {
                            // TODO figure out what to do with Err
                        }
                    }
                    Ok(())
                }
                .await;
                match build_result {
                    Ok(_) => Ok(DockerImageBuildResult {
                        log: log,
                        image_id: container_guid.clone(),
                    }),
                    Err(e) => {
                        error!("Error building container {}: {}", &container_guid, &e);
                        Err(String::from("Failed to build image"))
                    }
                }
            }
            Err(e) => {
                error!("Failed to tar source from path {}", source_path);
                Err(format!("{:?}", e))
            }
        }
    }

    /// Both creates and starts a docker container
    pub async fn start_container(&self, image_id: &str, port: i64) -> Result<String, ()> {
        let mut ports = HashMap::new();

        let p = format!("{}/tcp", port);

        // TODO this is so dumb there must be a better way
        // but &port makes the 'exposed_ports' unhappy
        ports.insert(&p[0..p.len()], HashMap::new());

        let mut port_bindings = HashMap::new();
        port_bindings.insert(
            p.clone(),
            Some(vec![PortBinding {
                host_ip: Some(String::from("0.0.0.0")),
                host_port: Some(String::from(format!("{}", port))),
            }]),
        );

        let config = Config {
            image: Some(image_id),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            exposed_ports: Some(ports),
            host_config: Some(HostConfig {
                port_bindings: Some(port_bindings),
                ..Default::default()
            }),
            ..Default::default()
        };

        println!("{:?}", config);

        let res = self
            .conn
            .create_container(Some(CreateContainerOptions { name: image_id }), config)
            .await;

        println!("{:?}", res);

        if let Ok(response) = res {
            info!("Docker built container {}", response.id);
            // TODO why is this not Ok? Ok(String::from(response.id))
            let res = self
                .conn
                .start_container(&response.id, None::<StartContainerOptions<String>>)
                .await;
            if let Ok(_) = res {
                info!("Docker started container {}", response.id);
                return Ok(String::from(response.id));
            }
            return Err(());
        }
        Err(())
    }

    pub async fn stop_container(&self, container_id: &str) -> () {
        self.conn
            .stop_container(container_id, Some(StopContainerOptions { t: 10 }))
            .await
            .unwrap();
        ()
    }

    pub async fn get_container_stats(&self, container_id: &str) -> () {
        // TODO container stats
    }

    /// Remove unused images from docker
    ///
    /// # Arguments
    ///
    /// * `keep_if_created_before_time` - A time string indicating a duration since now.
    /// Images created before them will be deleted. Defaults to 1 hour ago.
    ///
    /// # Examples
    ///
    /// ```
    /// let docker = DockerBroker::new();
    /// docker.prune_images("10m"); // prune images more than 10 min old
    /// ```
    pub async fn prune_images(&self, keep_if_created_before_time: Option<&str>) -> () {
        let mut filters = HashMap::new();
        filters.insert("until", vec![keep_if_created_before_time.unwrap_or("1h")]); // keep images created < until ago
        filters.insert("dangling", vec!["false"]); // remove all images that are not running

        let out = self
            .conn
            .prune_images(Some(PruneImagesOptions { filters: filters }))
            .await
            .unwrap();

        info!(
            "Docker prune removed {} images, reclaimed {} bytes",
            out.images_deleted.unwrap_or(vec![]).len(), // TODO verify if this is actually correct
            out.space_reclaimed
        );
    }

    /// Remove unused containers from docker
    ///
    /// # Arguments
    ///
    /// * `keep_if_created_before_time` - A time string indicating a duration since now.
    /// Containers created before them will be deleted. Defaults to 1 hour ago.
    ///
    /// # Examples
    ///
    /// ```
    /// let docker = DockerBroker::new();
    /// docker.prune_containers("10m"); // prune containers more than 10 min old
    /// ```
    pub async fn prune_containers(&self, keep_if_created_before_time: Option<&str>) -> () {
        let mut filters = HashMap::new();
        filters.insert("until", vec![keep_if_created_before_time.unwrap_or("1h")]); // keep images created < until ago
        filters.insert("dangling", vec!["false"]); // remove all images that are not running

        let out = self
            .conn
            .prune_containers(Some(PruneContainersOptions { filters: filters }))
            .await
            .unwrap();

        info!(
            "Docker prune removed {} images, reclaimed {} bytes",
            out.containers_deleted.unwrap_or(vec![]).len(), // TODO verify if this is actually correct
            out.space_reclaimed.unwrap_or(0)
        );
        ()
    }

    /// Removes unused containers and images from docker
    /// Uses `docker::DockerBroker::prune_containers` and `docker::DockerBroker::prune_images` default `keep_if_created_before_time`.
    pub async fn prune(&self) {
        self.prune_containers(None).await;
        self.prune_images(None).await;
    }
}

pub struct DockerImageBuildResult {
    pub log: Vec<String>,
    pub image_id: String,
}
