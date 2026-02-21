use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Transform {
    position: [f64; 3], // initializing an array of 3 float64 values (xyz)
    rotation: [f64; 3],
    scale: [f64; 3],
}

#[derive(Serialize, Deserialize, Clone, Debug)]
// enum for object type
enum ObjectType {
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
enum LensType {
    Perspective,
    Orthographic,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
enum SensorFit {
    Auto,
    Horizontal,
    Vertical,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct CameraProperties {
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
struct PointLightProperties {
    // Common to all lights
    color: [f32; 3],
    temperature: f32, // Kelvin
    exposure: f32,
    // Specific to point lights
    power: f32,       // watts
    radius: f32,      // sphere radius for soft shadows
    soft_falloff: bool,
}
