use  bevy_reflect::reflect_trait;
// use pluginator::plugin_implementation;
use opencv::{prelude::*,  error::Result};
use ndarray::{ArcArray,Ix3};
pub use plugin_core:: *;

#[reflect_trait]
pub trait CameraCorePluginTrait:CameraPluginTrait{
    fn read_into(&self, img:&mut Mat)->Result<bool>;
    fn grab(&self)->Result<bool>;
    
}


#[reflect_trait]
pub trait CameraPluginTrait:CoreTrait{
    fn read(&self)->ArcArray<u8, Ix3>;
    fn receiver(&self) ->Option<tokio::sync::watch::Receiver<ArcArray<u8, Ix3>>>{
        None
    }

}

#[reflect_trait]
pub trait CameraWithRcvPluginTrait:CoreTrait{
    fn receiver(&self) ->tokio::sync::watch::Receiver<ArcArray<u8, Ix3>>;
}