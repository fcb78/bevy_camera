use bevy_reflect::Reflect;
use json;
use log;

use opencv::error::{Error, Result};
use opencv::imgproc::{cvt_color, COLOR_BGR2RGBA};
use opencv::prelude::*;

use ndarray::{ArcArray, Ix3, s};
use opencv::videoio::VideoCapture;
// use std::borrow::{Borrow, BorrowMut};
use lazy_static::lazy_static;
use std::cell::RefCell;
use std::collections::HashMap;
use std::mem::size_of;
use std::ptr::copy_nonoverlapping;
use std::sync::{Arc, RwLock};
use std::thread;
use tokio::sync::watch::{channel, Receiver, Sender};

use camera_core::*;

pub struct CameraOpencvCore {
    name: String,
    cap: Result<RefCell<VideoCapture>>,
}

impl CameraOpencvCore {
    pub fn new(name: &str, config: &str) -> Self {
        let mut obj = Self {
            name: name.to_string(),
            cap: Err(Error::new(-9000, "not init")),
        };

        let con = match json::parse(config) {
            Ok(con) => con,
            Err(e) => {
                obj.cap = Err(Error::new(-9001, format!("config Error: {}", e)));
                log::error!("{}", format!("{} {:?}", obj.log_prefix("new"), obj.cap));
                return obj;
            }
        };

        obj.cap = match VideoCapture::new(con["index"].as_i32().unwrap(), 0) {
            Ok(cap) => Ok(RefCell::new(cap)),
            Err(e) => {
                log::error!("{}{:?}", obj.log_prefix("new"), e);
                Err(e)
            }
        };

        obj
    }
}

impl CoreTrait for CameraOpencvCore {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn type_name(&self) -> String {
        String::from("CameraOpencvCore")
    }
}

impl CameraCorePluginTrait for CameraOpencvCore {
    fn read_into(&self, img: &mut Mat) -> Result<bool> {
        match &self.cap {
            Ok(cap) => match cap.borrow_mut().read(img) {
                Ok(r) => Ok(r),
                Err(e) => {
                    log::error!("{}{:?}", self.log_prefix("read_into"), e);
                    Err(e)
                }
            },
            Err(e) => {
                log::error!("{}{:?}", self.log_prefix("read_into"), e);
                let ee = Error::new(e.code, e.message.clone());
                return Err(ee);
            }
        }
    }

    fn grab(&self) -> Result<bool> {
        match &self.cap {
            Ok(cap) => match cap.borrow_mut().grab() {
                Ok(r) => Ok(r),
                Err(e) => {
                    log::error!("{}{:?}", self.log_prefix("grab"), e);
                    Err(e)
                }
            },
            Err(e) => {
                log::error!("{}{:?}", self.log_prefix("grab"), e);
                let ee = Error::new(e.code, e.message.clone());
                return Err(ee);
            }
        }
    }
}

impl CameraPluginTrait for CameraOpencvCore {
    fn read(&self) -> ArcArray<u8, Ix3> {
        let mut frame = Mat::default();
        let _ = self.read_into(&mut frame);
        let mut frame2 = Mat::default();
        cvt_color(&frame, &mut frame2, COLOR_BGR2RGBA, 0).unwrap();

        let sh = (
            frame2.rows() as usize,
            frame2.cols() as usize,
            frame2.channels() as usize,
        );

        let mut r = ArcArray::<u8, Ix3>::zeros(sh);
        unsafe {
            copy_nonoverlapping(
                frame.data(),
                r.as_mut_ptr(),
                frame.total() * frame.channels() as usize * size_of::<u8>(),
            );
        }
        r.slice_mut(s![.., .., 3]).fill(255);
        r
    }
}

lazy_static! {
    static ref CAMERA_OPENCV_RECIVERS: Arc<RwLock<HashMap<String, Receiver<ArcArray::<u8, Ix3>>>>> =
        Arc::new(RwLock::new(HashMap::new()));
}

#[derive(Reflect)]
#[reflect(CoreTrait)]
#[reflect(CameraPluginTrait)]
#[reflect(CameraWithRcvPluginTrait)]
pub struct CameraOpencv {
    name: String,
    // rx: Receiver<ArcArray<u8, Ix3>>,
    //is_initialized: bool
}

impl CameraOpencv {
    pub fn new(name: &str, config: &str) -> Self {
        //let frame = Mat::default();
        //let ar = ArcArray::<u8, Dim<IxDynImpl>>::default(IxDyn);
        let frame = ArcArray::<u8, Ix3>::from_elem((720, 1280, 4), 0u8);
        let (tx, rx) = channel(frame);

        let nm = name.to_string();
        let name = nm.clone();
        let config = config.to_string();

        // let dict:Arc<RwLock<HashMap<String, Receiver<ArcArray::<u8, Ix3>>>>> = CAMERA_OPENCV_RECIVERS.clone();
        CAMERA_OPENCV_RECIVERS
            .write()
            .unwrap()
            .insert(nm.clone(), rx);
        thread::spawn(move || camera_worker(tx, name, config));

        Self {
            name: nm,
            // rx,
        }
    }
}

impl CoreTrait for CameraOpencv {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn type_name(&self) -> String {
        String::from("CameraOpencv")
    }
}

impl CameraPluginTrait for CameraOpencv {
    fn read(&self) -> ArcArray<u8, Ix3> {
        let a = CAMERA_OPENCV_RECIVERS.clone();
        let b = a.read().unwrap();
        let c = b.get(&self.name).unwrap();

        let d = (*c.borrow()).clone();
        d
    }
    fn receiver(&self) -> Option<tokio::sync::watch::Receiver<ArcArray<u8, Ix3>>> {
       Some(CAMERA_OPENCV_RECIVERS
            .read()
            .unwrap()
            .get(&self.name)
            .unwrap()
            .clone())
    }
}

impl CameraWithRcvPluginTrait for CameraOpencv {
    fn receiver(&self) -> tokio::sync::watch::Receiver<ArcArray<u8, Ix3>> {
        CAMERA_OPENCV_RECIVERS
            .read()
            .unwrap()
            .get(&self.name)
            .unwrap()
            .clone()
    }
}

fn camera_worker(tx: Sender<ArcArray<u8, Ix3>>, name: String, config: String) {
    let mut error_count = 0;
    let core = CameraOpencvCore::new(&name, &config);
    let mut frame_mat = Mat::default();

    tx.send_modify(|r| {
        *r = core.read();
    });
   
    loop {
        std::thread::sleep(std::time::Duration::from_secs_f64(1.0/24.0));
        if tx.is_closed() || tx.receiver_count() < 1 {
            log::info!("CameraOpencv-{}-camera_worker: Channel closed, exit", name);
            return;
        }
        if error_count > 10 {
            log::error!("CameraOpencv-{}-camera_worker: Too many errors, exit", name);
            return;
        }
        match core.read_into(&mut frame_mat) {
            Ok(_) => tx.send_modify(|r| {
                let mut frame2 = Mat::default();
                cvt_color(&frame_mat, &mut frame2, COLOR_BGR2RGBA, 0).unwrap();
                unsafe {
                    copy_nonoverlapping(
                        frame2.data(),
                        r.as_mut_ptr(),
                        frame2.total() * frame2.channels() as usize * size_of::<u8>(),
                    );
                }

                r.slice_mut(s![.., .., 3]).fill(255);
            }),
            Err(e) => {
                error_count += 1;
                log::error!("CameraOpencv-{}-camera_worker: {:?}", name, e);
            } // tx.send_modify(|r| *r = Err(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }
    #[test]
    fn test_opencv_core() {
        init();
        let cap = CameraOpencvCore::new("test_camera", r#"{"index":0}"#);
        let frame = cap.read();

        println!("shape={:?}, \n", frame.shape());
        let mut frame_mat = Mat::default();
        cap.read_into(&mut frame_mat).unwrap();
        println!(
            "total={:?}, row={}, cols={}, channel={}\n",
            frame_mat.total(),
            frame_mat.rows(),
            frame_mat.cols(),
            frame_mat.channels() as usize
        );
        let mut frame2 = Mat::default();
        cvt_color(&frame_mat, &mut frame2, COLOR_BGR2RGBA, 0).unwrap();
        println!("depth={}", frame2.depth());
        assert_eq!(
            frame_mat.total(),
            frame_mat.rows() as usize * frame_mat.cols() as usize
        );
    }
}
