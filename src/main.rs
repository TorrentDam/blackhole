use k8s_openapi::api::core::v1::{Container, ContainerPort, EnvVar, Pod, PodSpec};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::{Api, Client};
use kube::api::PostParams;
use std::string::String;
use std::fs;
use std::fs::DirEntry;

#[tokio::main]
async fn main() -> Result<(), kube::Error> {

    // TODO:
    // 1. Watch for files with magnet links in a directory
    // 2. Check if pod for magnet link already exists
    // 2. Create a pod with a container that downloads the magnet link if it doesn't exist

    let files: Vec<DirEntry> = fs::read_dir("/data/torrents").unwrap()
        .map(|entry| entry.unwrap())
        .filter(|entry| entry.file_type().unwrap().is_file())
        .collect();

    let magnet_links: Vec<String> = files.iter()
        .filter(|entry| entry.path().extension().unwrap() == "magnet")
        .map(|entry| fs::read(entry.path()).unwrap())
        .map(|bytes| String::from_utf8(bytes).unwrap())
        .collect();

    let client = Client::try_default().await?;
    let api: Api<Pod> = Api::namespaced(client, "media-server");

    for magnet_link in magnet_links {
        let pod: Pod = Pod {
            metadata: ObjectMeta {
                name: Some("test-echo-server".to_owned()),
                namespace: Some("media-server".to_owned()),
                ..ObjectMeta::default()
            },
            spec: Some(PodSpec {
                containers: vec![Container {
                    name: "echo".to_owned(),
                    image: Some("nginx:1.14.2".to_owned()),
                    ports: Some(vec![ContainerPort {
                        container_port: 80,
                        ..ContainerPort::default()
                    }]),
                    env: Some(vec![EnvVar {
                        name: "MAGNET_LINK".to_owned(),
                        value: Some(magnet_link),
                        ..EnvVar::default()
                    }]),
                    ..Container::default()
                }],
                ..PodSpec::default()
            }),
            ..Pod::default()
        };
        api.create(&PostParams::default(), &pod).await?;
    }
    Ok(())
}
