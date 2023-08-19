mod crd;

use kube::CustomResourceExt;
use crd::Blackhole;

fn main() {
    print!("{}", serde_yaml::to_string(&Blackhole::crd()).unwrap())
}