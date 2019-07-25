#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use rocket_contrib::json::Json;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::{thread, time};
use visca::{Camera, PanTiltValue, Result};

#[derive(Serialize, Deserialize, Debug)]
struct CameraState {
    tilt: i32,
    pan: i32,
}

impl From<PanTiltValue> for CameraState {
    fn from(val: PanTiltValue) -> Self {
        CameraState {
            tilt: val.tilt as i32,
            pan: val.pan as i32,
        }
    }
}

impl Into<PanTiltValue> for CameraState {
    fn into(self) -> PanTiltValue {
        PanTiltValue {
            tilt: self.tilt as i16,
            pan: self.pan as i16,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct CameraStateResponse {
    camera: CameraState,
}

#[derive(Deserialize)]
struct CameraStateRequest {
    camera: CameraState,
}

#[get("/state")]
fn get_state(camera: rocket::State<Mutex<Camera>>) -> Result<Json<CameraStateResponse>> {
    let mut cam = camera.lock().unwrap();

    cam.pan_tilt().get().map(|state| {
        Json(CameraStateResponse {
            camera: state.into(),
        })
    })
}

#[patch("/state", format = "json", data = "<new_state>")]
fn patch_state(
    camera: rocket::State<Mutex<Camera>>,
    new_state: Json<CameraStateRequest>,
) -> Result<Json<CameraStateResponse>> {
    let mut cam = camera.lock().unwrap();

    let input = new_state.into_inner();
    let initial = cam.pan_tilt().get()?;

    let desired = PanTiltValue {
        pan: initial.pan + (input.camera.pan as i16),
        tilt: initial.tilt + (input.camera.tilt as i16),
    };

    if initial == desired {
        return Ok(Json(CameraStateResponse {
            camera: desired.into(),
        }));
    }

    cam.pan_tilt().set_absolute(desired)?;

    let mut current;

    let timeout = time::Duration::from_secs(5);
    let start = time::Instant::now();

    loop {
        current = cam.pan_tilt().get()?;

        if current != initial {
            break;
        }

        if start.elapsed() > timeout {
            break;
        }

        thread::sleep(time::Duration::from_millis(100));
    }

    Ok(Json(CameraStateResponse {
        camera: current.into(),
    }))
}

#[put("/state", format = "json", data = "<new_state>")]
fn put_state(
    camera: rocket::State<Mutex<Camera>>,
    new_state: Json<CameraStateRequest>,
) -> Result<Json<CameraStateResponse>> {
    let mut cam = camera.lock().unwrap();

    let input = new_state.into_inner();
    let initial = cam.pan_tilt().get()?;
    let desired = input.camera.into();

    if initial == desired {
        return Ok(Json(CameraStateResponse {
            camera: desired.into(),
        }));
    }

    cam.pan_tilt().set_absolute(desired)?;

    let mut current;

    let timeout = time::Duration::from_secs(5);
    let start = time::Instant::now();

    loop {
        current = cam.pan_tilt().get()?;

        if current != initial {
            break;
        }

        if start.elapsed() > timeout {
            break;
        }

        thread::sleep(time::Duration::from_millis(100));
    }

    Ok(Json(CameraStateResponse {
        camera: current.into(),
    }))
}

#[post("/presets/<id>/recall")]
fn recall_preset(camera: rocket::State<Mutex<Camera>>, id: u8) -> Result<()> {
    let mut camera = camera.lock().unwrap();
    camera.presets().recall(id)
}

fn main() -> Result<()> {
    let camera = Camera::open("/dev/cu.usbserial-AM00QCCD")?;

    rocket::ignite()
        .manage(Mutex::new(camera))
        .mount(
            "/",
            routes![get_state, put_state, patch_state, recall_preset],
        )
        .launch();

    Ok(())
}
