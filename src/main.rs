#![feature(proc_macro_hygiene, decl_macro)]

use rocket_contrib::json::Json;
use serde::{Deserialize, Serialize};
use std::io::Result;
use std::sync::Mutex;
use std::{thread, time};

#[macro_use]
extern crate rocket;

#[derive(Serialize, Deserialize, Debug)]
struct CameraState {
    tilt: i32,
    pan: i32,
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
fn get_state(camera: rocket::State<Mutex<visca::Camera>>) -> Result<Json<CameraStateResponse>> {
    let mut cam = camera.lock().unwrap();

    cam.pan_tilt().get().map(|state| {
        Json(CameraStateResponse {
            camera: CameraState {
                tilt: state.tilt as i32,
                pan: state.pan as i32,
            },
        })
    })
}

#[patch("/state", format = "json", data = "<new_state>")]
fn patch_state(
    camera: rocket::State<Mutex<visca::Camera>>,
    new_state: Json<CameraStateRequest>,
) -> Result<Json<CameraStateResponse>> {
    let mut cam = camera.lock().unwrap();

    let input = new_state.into_inner();

    let initial = cam.pan_tilt().get().unwrap();
    let desired = visca::PanTiltValue {
        pan: initial.pan + (input.camera.pan as i16),
        tilt: initial.tilt + (input.camera.tilt as i16),
    };

    if initial == desired {
        return Ok(Json(CameraStateResponse {
            camera: CameraState {
                tilt: desired.tilt as i32,
                pan: desired.pan as i32,
            },
        }));
    }

    cam.pan_tilt().set(desired)?;

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
        camera: CameraState {
            tilt: current.tilt as i32,
            pan: current.pan as i32,
        },
    }))
}

#[put("/state", format = "json", data = "<new_state>")]
fn put_state(
    camera: rocket::State<Mutex<visca::Camera>>,
    new_state: Json<CameraStateRequest>,
) -> Result<Json<CameraStateResponse>> {
    let mut cam = camera.lock().unwrap();

    let input = new_state.into_inner();

    let initial = cam.pan_tilt().get().unwrap();
    let desired = visca::PanTiltValue {
        pan: input.camera.pan as i16,
        tilt: input.camera.tilt as i16,
    };

    if initial == desired {
        return Ok(Json(CameraStateResponse {
            camera: CameraState {
                tilt: desired.tilt as i32,
                pan: desired.pan as i32,
            },
        }));
    }

    cam.pan_tilt().set(desired)?;

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
        camera: CameraState {
            tilt: current.tilt as i32,
            pan: current.pan as i32,
        },
    }))
}

fn main() -> Result<()> {
    let camera = visca::Camera::open("/dev/cu.usbserial-AM00QCCD")?;

    rocket::ignite()
        .manage(Mutex::new(camera))
        .mount("/", routes![get_state, put_state, patch_state])
        .launch();

    Ok(())
}
