#![feature(trivial_bounds)]
extern crate camera_core;
pub mod camera_opencv;

use std::sync::Arc;

use bevy_reflect::{Reflect, TypeRegistry};
use camera_opencv::CameraOpencv;
use pluginator::plugin_implementation;

use camera_core::*;

pub struct CameraOpencvPlugin;

impl PluginTrait for CameraOpencvPlugin{
    fn reg(&self, reg:&mut TypeRegistry) {
        reg.register::<CameraOpencv>();
    }

    fn create_obj(&self, name: &str, config:&str)->Arc<dyn Reflect> {
        Arc::new(CameraOpencv::new(name, config))
    }

}

plugin_implementation!(PluginTrait, CameraOpencvPlugin);
#[cfg(test)]
mod tests {
    

    
}
