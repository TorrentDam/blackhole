use k8s_openapi::api::core::v1::{
    Container,
    EnvVar,
    PodSpec,
    PodTemplateSpec,
    Volume,
    VolumeMount,
    PersistentVolumeClaimVolumeSource,
};
use k8s_openapi::api::batch::v1::{Job, JobSpec};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::{Api, Client};
use kube::api::{ListParams, PostParams};
use hightorrent::{MagnetLink, TorrentFile};
use std::string::String;
use std::fs::DirEntry;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), kube::Error> {
    // TODO:
    // 1. Move downloaded files to the "complete" directory

    let client = Client::try_default().await?;
    let job_api: Api<Job> = Api::namespaced(client, "media-server");
    loop {
        run(&job_api).await?;
        sleep(Duration::from_secs(10)).await;
    }
}

async fn run(job_api: &Api<Job>) -> Result<(), kube::Error> {
    let files: Vec<DirEntry> = std::fs::read_dir("/data/torrents").unwrap()
        .map(|entry| entry.unwrap())
        .filter(|entry| entry.file_type().unwrap().is_file())
        .collect();

    let torrent_files: Vec<TorrentFile> = files.iter()
        .filter(|entry| entry.path().extension().unwrap() == "torrent")
        .map(|entry| entry.to_owned())
        .map(|entry| std::fs::read(entry.path()).unwrap())
        .map(|slice| TorrentFile::from_slice(&slice).unwrap())
        .collect();

    println!("Discovered torrent files: {:?}", torrent_files.len());

    let magnet_links: Vec<MagnetLink> = files.iter()
        .filter(|entry| entry.path().extension().unwrap() == "magnet")
        .map(|entry| std::fs::read(entry.path()).unwrap())
        .map(|bytes| String::from_utf8(bytes).unwrap())
        .map(|string| url::Url::parse(&string).unwrap())
        .map(|url| MagnetLink::from_url(&url).unwrap())
        .collect();

    println!("Discovered magnet links: {:?}", magnet_links.len());

    let info_hashes: Vec<String> = magnet_links.iter()
        .map(|magnet_link| magnet_link.hash().as_str().to_owned())
        .chain(
            torrent_files.iter()
                .map(|torrent_file| torrent_file.hash().to_owned())
        )
        .collect();

    let running_jobs: Vec<String> =
        job_api
            .list(&ListParams::default()).await?
            .items.into_iter()
            .map(|job| job.metadata.name.unwrap())
            .collect();

    for info_hash in info_hashes {
        let job_name: String = "blackhole-torrent-".to_owned() + &info_hash;
        if running_jobs.contains(&job_name) {
            continue;
        }
        println!("Creating job for downloading {}", info_hash);
        let pod_spec: PodSpec = PodSpec {
            restart_policy: Some("Never".to_owned()),
            containers: vec![Container {
                name: "echo".to_owned(),
                image: Some("ghcr.io/torrentdam/cmd:latest".to_owned()),
                args: Some(vec!["download".to_owned(), "--info-hash".to_owned(), info_hash.clone()]),
                working_dir: Some("/data".to_owned()),
                env: Some(vec![EnvVar {
                    name: "INFO_HASH".to_owned(),
                    value: Some(info_hash.clone()),
                    ..EnvVar::default()
                }]),
                volume_mounts: Some(vec![VolumeMount {
                    name: "data".to_owned(),
                    sub_path: Some("downloading".to_owned()),
                    mount_path: "/data".to_owned(),
                    ..VolumeMount::default()
                }]),
                ..Container::default()
            }],
            volumes: Some(vec![Volume {
                name: "data".to_owned(),
                persistent_volume_claim: Some(PersistentVolumeClaimVolumeSource {
                    claim_name: "movies".to_owned(),
                    ..PersistentVolumeClaimVolumeSource::default()
                }),
                ..Volume::default()
            }]),
            ..PodSpec::default()
        };
        let job: Job = Job {
            metadata: ObjectMeta {
                name: Some(job_name.clone()),
                namespace: Some("media-server".to_owned()),
                ..ObjectMeta::default()
            },
            spec: Some(JobSpec {
                template: PodTemplateSpec {
                    metadata: None,
                    spec: Some(pod_spec.clone()),
                },
                ..JobSpec::default()
            }),
            ..Job::default()
        };
        job_api.create(&PostParams::default(), &job).await?;
    }
    Ok(())
}
