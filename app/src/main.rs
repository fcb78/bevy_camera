use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
};
use bevy_editor_pls::prelude::*;
use plugin_core::{load_plugin, PluginTrait};
use pluginator::LoadedPlugin;
use serde::Deserialize;
use std::{
    collections::HashMap,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Arc,
    },
};

use camera_core::{CameraPluginTrait, ReflectCameraPluginTrait, ReflectCameraWithRcvPluginTrait};
use serde_json::{Map, Value};

fn main() {
    // env_logger::init();
    log::info!("Hello, world!");
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EditorPlugin::default())
        .add_plugins(bevy_tokio_tasks::TokioTasksPlugin::default())
        .insert_resource(PluginList::default())
        .insert_resource(DriverList::default())
        .insert_resource(Assets::<Image>::default())
        .add_systems(
            Startup,
            (load_plugins, load_drivers, steup_image).chain(),
        )
        // .add_systems(Update, update_image)
        .run();
}

#[derive(Resource, Default)]
struct PluginList(pub HashMap<String, LoadedPlugin<dyn PluginTrait>>);

#[derive(Resource, Default)]
struct DriverList(pub HashMap<String, Arc<dyn Reflect>>);

#[derive(Deserialize)]
struct DriverInfo {
    pub name: String,
    pub config: Map<String, Value>,
    pub plugin: String,
}

fn load_drivers(plugins: ResMut<PluginList>, mut drivers: ResMut<DriverList>) {
    let drvs_info: Vec<DriverInfo> = serde_json::from_str(
        &std::fs::read_to_string("./config.d/drivers.json")
            .expect("drivers list file cannot be read!"),
    )
    .expect("drivers list file is ill-formed!");
    for drv_info in drvs_info {
        match plugins.0.get(&drv_info.plugin) {
            Some(plugin) => {
                let config = serde_json::to_string(&drv_info.config).unwrap();
                let drv = plugin.create_obj(&drv_info.name, &config);
                drivers.0.insert(drv_info.name.clone(), drv);
            }
            None => {}
        };
    }
}

#[derive(Deserialize)]
struct PluginInfo {
    path: PathBuf,
    name: String,
}

fn load_plugins(
    // reg: &mut TypeRegistry,
    // path:&str
    reg: Res<AppTypeRegistry>,
    mut plugins: ResMut<PluginList>,
) {
    let plugins_info: Vec<PluginInfo> = serde_json::from_str(
        &std::fs::read_to_string("./config.d/plugins.json")
            .expect("plugins list file cannot be read!"),
    )
    .expect("plugins list file is ill-formed!");
    // let mut plugins = HashMap::new();
    for plugin_info in plugins_info {
        let plugin = unsafe { load_plugin(Path::new(&plugin_info.path)) }
            .unwrap_or_else(|_err| panic!("Plugin file {:?} cannot be read!", plugin_info.path));
        plugin.reg(&mut reg.write());
        plugins.0.insert(plugin_info.name, plugin);
    }
}

fn steup_image(
    reg_: Res<AppTypeRegistry>,
    mut image_handle: ResMut<Assets<Image>>,
    mut commands: Commands,
    drivers: Res<DriverList>,
    runtime: ResMut<bevy_tokio_tasks::TokioTasksRuntime>,
) {
    let cap_obj = drivers.0.get("camera").unwrap().clone();
    let reg = reg_.read();
    let mut rx = {
        let rcv_do = reg
        .get_type_data::<ReflectCameraWithRcvPluginTrait>(cap_obj.type_id())
        .unwrap();
    let cap = rcv_do.get(&*cap_obj).unwrap();
        cap.receiver().clone()
        
    };
    let camera_do = reg
        .get_type_data::<ReflectCameraPluginTrait>(cap_obj.type_id())
        .unwrap();
    let cap = camera_do.get(&*cap_obj).unwrap();
    // let mut rx = cap.receiver().unwrap();
    let array = cap.read();
    // let sh = (500, 500, 4);
    // let array = ArcArray::<u8, Ix3>::ones(sh)*255;
    // use ndarray::s;
    //array.slice_mut(s![.., .., 3]).fill(255);
    commands.spawn(Camera2dBundle::default());

    let ext = Extent3d {
        width: array.shape()[1] as u32,
        height: array.shape()[0] as u32,
        depth_or_array_layers: 1,
    };
    // let array2 = array;
    let n = array.shape()[0] * array.shape()[1] * array.shape()[2];
    log::debug!(
        "{}, {}, {} =? {}, {}, {}",
        array.shape()[0],
        array.shape()[1],
        array.shape()[2],
        ext.width,
        ext.height,
        ext.depth_or_array_layers
    );
    let vec = array.into_shape(n).unwrap().to_vec();

    let image = Image::new(
        ext,
        TextureDimension::D2,
        vec,
        TextureFormat::Rgba8Unorm,
         RenderAssetUsages::RENDER_WORLD|RenderAssetUsages::MAIN_WORLD,
    );
    let handle = image_handle.add(image);
    commands.spawn(SpriteBundle {
        texture: handle,
        transform: Transform::from_xyz(0., 0., 0.),
        ..Default::default()
    });

    runtime.spawn_background_task(|mut ctx| async move {
        tokio::spawn(async move {
            loop {
                let _ = rx.changed().await;
                let array = rx.borrow().clone();
                let ext = Extent3d {
                    width: array.shape()[1] as u32,
                    height: array.shape()[0] as u32,
                    depth_or_array_layers: 1,
                };
                // let array2 = array;
                let n = array.shape()[0] * array.shape()[1] * array.shape()[2];
                log::debug!(
                    "{}, {}, {} =? {}, {}, {}",
                    array.shape()[0],
                    array.shape()[1],
                    array.shape()[2],
                    ext.width,
                    ext.height,
                    ext.depth_or_array_layers
                );
                let vec = array.into_shape(n).unwrap().to_vec();

                let image = Image::new(
                    ext,
                    TextureDimension::D2,
                    vec,
                    TextureFormat::Rgba8Unorm,
                    RenderAssetUsages::RENDER_WORLD|RenderAssetUsages::MAIN_WORLD,
                );
                ctx.run_on_main_thread(move |cx| {
                    let world = cx.world;
                    let mut image_handle = world.get_resource_mut::<Assets<Image>>().unwrap();
                    for (idx, img) in image_handle.iter_mut() {
                        *img = image.clone()
                    }
                })
                .await;
            }
        });
    });
}
