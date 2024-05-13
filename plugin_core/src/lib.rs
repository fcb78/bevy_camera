use std::sync::Arc;

use bevy_reflect::{Reflect, TypeRegistry, reflect_trait};
use pluginator::plugin_trait;

pub trait PluginTrait:Sync + Send {
    fn reg(&self, reg:&mut TypeRegistry);
    fn create_obj(&self, name: &str, config:&str)->Arc<dyn Reflect>;
    // fn create_core_obj(&self, name: &str, config:&str)->Box<dyn Reflect>;
}

plugin_trait!(PluginTrait);

#[reflect_trait]
pub trait CoreTrait{
    fn name(&self) -> String;
    fn type_name(&self) ->String;
    fn log_prefix(&self, fun_name: &str) -> String{
        format!("{}-{}-{fun_name}: ", self.type_name(), self.name())
    }
}