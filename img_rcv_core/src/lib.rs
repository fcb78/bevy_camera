use bevy_reflect::reflect_trait;
use ndarray::{ArcArray, Ix3};
use plugin_core::CoreTrait;
use tokio::sync::watch;

#[reflect_trait]
pub trait ImgRcvPluginTrait:CoreTrait{
    fn start(&self, rx: watch::Receiver<ArcArray<u8, Ix3>>);

}

