use k8s_openapi::api::core::v1::{Container, ContainerPort, EnvVar, Pod, PodSpec};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::{Api, Client};
use kube::api::PostParams;
use hightorrent::{InfoHash, MagnetLink};
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

    let magnet_links: Vec<MagnetLink> = files.iter()
        .filter(|entry| entry.path().extension().unwrap() == "magnet")
        .map(|entry| fs::read(entry.path()).unwrap())
        .map(|bytes| String::from_utf8(bytes).unwrap())
        .map(|string| url::Url::parse(&string).unwrap())
        .map(|url| MagnetLink::from_url(&url).unwrap())
        .collect();

    let info_hashes: Vec<InfoHash> = magnet_links.iter()
        .map(|magnet_link| magnet_link.hash().to_owned())
        .collect();

    let client = Client::try_default().await?;
    let api: Api<Pod> = Api::namespaced(client, "media-server");

    for info_hash in info_hashes {
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
                        name: "INFO_HASH".to_owned(),
                        value: Some(info_hash.to_string()),
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
