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
struct CameraProperties {
    
}
