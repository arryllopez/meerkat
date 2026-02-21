use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::sync::Arc;
use dashmap::DashMap;

pub type ServerState = Arc<DashMap<String, Session>>;



#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Transform {
    position: [f64; 3], // initializing an array of 3 float64 values (xyz)
    rotation: [f64; 3],
    scale: [f64; 3],
}

#[derive(Serialize, Deserialize, Clone, Debug)]
// enum for object type
pub enum ObjectType {
    Cube,
    Sphere,
    Cylinder,
    Camera,     // needs struct
    PointLight, // needs struct
    SpotLight,  // needs struct
    AreaLight,  // needs struct
    SunLight,   // needs struct
    AssetRef,   // asset ref being a reference to an asset inside of a library
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum LensType {
    Perspective,
    Orthographic,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum SensorFit {
    Auto,
    Horizontal,
    Vertical,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CameraProperties {
    // Lens
    lens_type: LensType,
    focal_length: f64,       // mm, used when perspective
    orthographic_scale: f64, // used when orthographic
    shift_x: f64,
    shift_y: f64,
    clip_start: f64,
    clip_end: f64,
    // Depth of Field
    focal_distance: f64,
    aperture_fstop: f64,
    aperture_blades: u32,
    aperture_rotation: f64,
    aperture_ratio: f64,
    // Sensor
    sensor_fit: SensorFit,
    sensor_width: f64,
    sensor_height: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PointLightProperties {
    color: [f32; 3],
    temperature: f32, // Kelvin
    exposure: f32,
    power: f32,       // watts
    radius: f32,      // sphere radius for soft shadows
    soft_falloff: bool,
    normalize : bool, 
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SpotLightProperties { 
    color : [f32; 3], 
    temperature : f32,
    exposure : f32, 
    normalize : bool, 
    power : f32, 
    radius : f32, 
    soft_falloff : bool, 
    angle : f32, 
    blend : f32, 
    show_cone : bool, 
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AreaLightProperties {
    // Common
    color: [f32; 3],
    temperature: f32,
    exposure: f32,
    normalize: bool,
    // Area specific
    power: f32,
    shape: AreaLightShape,
    size_x: f32, // used by Rectangle and Ellipse
    size_y: f32, // used by Rectangle and Ellipse
    size: f32,   // used by Square and Disk
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum AreaLightShape { // different shapes that an area light can have since blender has fixed options for the shape of an area light
    Rectangle, 
    Square, 
    Disk, 
    Ellipse, 
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SunLightProperties {
    // Common
    color: [f32; 3],
    temperature: f32,
    exposure: f32,
    normalize: bool,
    // Sun specific
    strength: f32,
    angle: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ObjectProperties {
    Camera(CameraProperties),
    PointLight(PointLightProperties),
    SpotLight(SpotLightProperties),
    AreaLight(AreaLightProperties),
    SunLight(SunLightProperties),
}


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SceneObject {
    object_id: Uuid,
    name: String,
    object_type: ObjectType,
    asset_id: Option<String>,      // None for non-asset objects
    asset_library: Option<String>, // None for non-asset objects
    transform: Transform,
    properties: Option<ObjectProperties>, // None for primitives/asset refs
    created_by: Uuid,
    last_updated_by: Uuid,
    last_updated_at: u64,          // unix timestamp ms
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct User {
    display_name : String, 
    color : [u8;3],
    selected_object : Option<Uuid>, 
    connected_at : u64, // timestamp 
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogEntry { 
    timestamp : u64, 
    event_type : String, 
    payload : serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Session { 
    session_id : String, 
    objects : DashMap<Uuid, SceneObject>, 
    users : DashMap<Uuid, User>,
    event_log : Vec<LogEntry>, 
}


