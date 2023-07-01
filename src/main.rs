use k8s_openapi::api::core::v1::{Container, ContainerPort, Pod, PodSpec};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::{Api, Client};
use kube::api::PostParams;

#[tokio::main]
async fn main() -> Result<(), kube::Error> {

    // TODO:
    // 1. Watch for files with magnet links in a directory
    // 2. Create a pod with a container that downloads the magnet link

    let client = Client::try_default().await?;
    let api: Api<Pod> = Api::namespaced(client, "media-server");
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
                ..Container::default()
            }],
            ..PodSpec::default()
        }),
        ..Pod::default()
    };
    api.create(&PostParams::default(), &pod).await?;
    Ok(())
}
