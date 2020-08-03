pub mod docker;
use bollard::image::ListImagesOptions;
use bollard::{
    container::{ListContainersOptions, StatsOptions},
    Docker,
};
use docker::DockerBroker;
use dotenv;
use futures_util::stream::*;
use log::info;

#[tokio::main]
async fn main() -> Result<(), ()> {
    dotenv::dotenv().ok();
    env_logger::init();
    async move {
        let docker = Docker::connect_with_unix_defaults().unwrap();
        let version = docker.version().await;
        println!("{:?}", version);
    }
    .await;

    use std::default::Default;

    // Use a connection function described above
    // let docker = Docker::connect_...;

    async move {
        let docker = Docker::connect_with_unix_defaults().unwrap();
        let images = &docker
            .list_images(Some(ListImagesOptions::<String> {
                all: true,
                ..Default::default()
            }))
            .await
            .unwrap();

        for image in images {
            println!("-> {:?}", image);
        }
    }
    .await;

    async move {
        let docker = Docker::connect_with_unix_defaults().unwrap();
        let stats = &docker
            .stats(
                "rab",
                Some(StatsOptions {
                    stream: false,
                    ..Default::default()
                }),
            )
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        for stat in stats {
            println!(
                "{} - mem total: {:?} | mem usage: {:?}",
                stat.name, stat.memory_stats.max_usage, stat.memory_stats.usage
            );
        }
    }
    .await;

    async move {
        let docker = Docker::connect_with_unix_defaults().unwrap();
        let images = &docker
            .list_containers(Some(ListContainersOptions::<String> {
                ..Default::default()
            }))
            .await
            .unwrap();

        for image in images {
            println!("-> {:?}", image);
        }
    }
    .await;

    // Prune old images
    async move {
        let docker = DockerBroker::new().await;
        if let Some(docker) = docker {
            docker.prune_images(None).await;
        }
    }
    .await;

    // Build an image
    let mut image_id = String::from("");
    async {
        let docker = DockerBroker::new().await;
        if let Some(docker) = docker {
            let res = docker.build_image("scapegoat").await;
            if let Ok(r) = res {
                info!("----- Docker Build Results for {} -----", r.image_id);
                info!("{:?}", r.log);
                image_id = r.image_id;
            }
        }
    }
    .await;

    // List images
    async move {
        let docker = DockerBroker::new().await;
        if let Some(docker) = docker {
            let ids = docker.get_image_ids().await;

            for id in ids {
                println!("{}", id);
            }
        }
    }
    .await;

    // Start a container
    async {
        let docker = DockerBroker::new().await;
        if let Some(docker) = docker {
            let ids = docker.start_container(&image_id, 9000).await;

            if let Ok(id) = ids {
                info!("Spawned {}", id);
            }
        }
    }
    .await;

    std::thread::sleep(std::time::Duration::new(15, 0));

    // kill the started container
    async move {
        let docker = DockerBroker::new().await;
        if let Some(docker) = docker {
            docker.stop_container(&image_id).await;
        }
    }
    .await;

    Ok(())
}
