mod crd;

use k8s_openapi::api::core::v1::{Container, EnvVar, PodSpec, PodTemplateSpec, Volume, VolumeMount, PersistentVolumeClaimVolumeSource};
use k8s_openapi::api::batch::v1::{Job, JobSpec};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::{Api, Client};
use kube::api::{ListParams, PostParams};
use hightorrent::{MagnetLink, TorrentFile};
use std::string::String;
use std::fs::{DirEntry};
use std::path::PathBuf;
use tokio::time::{sleep, Duration};
use log::{info, LevelFilter};
use kube::runtime::{reflector, watcher, watcher::Config, WatchStreamExt};
use futures::{future, StreamExt};
use kube::runtime::reflector::ObjectRef;
use crate::crd::Blackhole;

#[tokio::main]
async fn main() -> Result<(), kube::Error> {
    env_logger::builder().filter_level(LevelFilter::Info).init();

    let client = Client::try_default().await?;

    let namespace = "media-server";

    let blackhole_api: Api<Blackhole> = Api::namespaced(client.clone(), namespace);
    let (reader, writer) = reflector::store::<Blackhole>();

    let rf = reflector(writer, watcher(blackhole_api, Config::default()));
    tokio::spawn(async move {
        rf.applied_objects().for_each(|_| future::ready(())).await;
    });

    let job_api: Api<Job> = Api::namespaced(client, namespace);
    loop {
        sleep(Duration::from_secs(10)).await;

        let blackhole = reader.get(&ObjectRef::new("blackhole").within(namespace));
        let Some(blackhole) = blackhole else {
            info!("Blackhole CRD not found");
            continue;
        };
        run(&job_api, &blackhole).await?;
    }
}

async fn run(job_api: &Api<Job>, blackhole: &Blackhole) -> Result<(), kube::Error> {
    let mut files: Vec<DirEntry> = std::fs::read_dir("torrents").unwrap()
        .map(|entry| entry.unwrap())
        .filter(|entry| entry.file_type().unwrap().is_file())
        .collect();

    let info_hashes: Vec<InfoHashSource> = files.iter_mut().filter_map(move |entry| {
        InfoHashSource::from_file(entry)
    }).collect();

    let running_jobs: Vec<Job> = job_api.list(&ListParams::default()).await?.items;

    for source in info_hashes {
        let info_hash = &source.info_hash;
        let job_name: String = format!("blackhole-torrent-{}", info_hash[0..6].to_owned());
        let downloading_dir = format!("downloading/{}", source.file_name);
        if let Some(job) = running_jobs.iter().find(|job| job.metadata.name.as_ref() == Some(&job_name)) {
            if job.status.as_ref().is_some_and(|status| status.succeeded == Some(1)) {
                let complete_dir = format!("complete/{}", source.file_name);
                info!("Job {} succeeded, moving files to {}", job_name, complete_dir);
                std::fs::rename(&downloading_dir, &complete_dir).unwrap();
                std::fs::remove_file(&source.path).unwrap();
            }
            continue;
        }
        info!("Creating job for downloading \"{}\" ({})", source.file_name, info_hash);
        let pod_spec: PodSpec = PodSpec {
            restart_policy: Some("Never".to_owned()),
            containers: vec![Container {
                name: "echo".to_owned(),
                image: Some("ghcr.io/torrentdam/cmd:latest".to_owned()),
                args: Some(vec!["download".to_owned(), "--info-hash".to_owned(), info_hash.clone()]),
                working_dir: Some("/data".to_owned()),
                resources: blackhole.spec.resources.clone(),
                env: Some(vec![EnvVar {
                    name: "INFO_HASH".to_owned(),
                    value: Some(info_hash.clone()),
                    ..EnvVar::default()
                }]),
                volume_mounts: Some(vec![VolumeMount {
                    name: "movies".to_owned(),
                    sub_path: Some(downloading_dir.clone()),
                    mount_path: "/data".to_owned(),
                    ..VolumeMount::default()
                }]),
                ..Container::default()
            }],
            volumes: Some(vec![Volume {
                name: "movies".to_owned(),
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
                ttl_seconds_after_finished: Some(60),
                active_deadline_seconds: Some(60 * 360), // 6 hours
                template: PodTemplateSpec {
                    spec: Some(pod_spec.clone()),
                    ..PodTemplateSpec::default()
                },
                ..JobSpec::default()
            }),
            ..Job::default()
        };
        job_api.create(&PostParams::default(), &job).await?;
    }
    Ok(())
}

struct InfoHashSource {
    info_hash: String,
    file_name: String,
    path: PathBuf,
}

impl InfoHashSource {
    fn from_file(file: &DirEntry) -> Option<InfoHashSource> {
        let file_name = file.file_name().to_str()?.to_owned();
        let path = file.path();
        let info_hash =
            match file.path().extension()?.to_str()? {
                "torrent" => {
                    let slice = std::fs::read(&path).ok()?;
                    let torrent_file = TorrentFile::from_slice(&slice).ok()?;
                    torrent_file.hash().to_owned()
                }
                "magnet" => {
                    let bytes = std::fs::read(&path).ok()?;
                    let utf8_string = String::from_utf8(bytes).ok()?;
                    let url = url::Url::parse(&utf8_string).ok()?;
                    let magnet_link = MagnetLink::from_url(&url).ok()?;
                    magnet_link.hash().as_str().to_owned()
                }
                _ => return None,
            };
        Some(InfoHashSource { info_hash, file_name, path })
    }
}