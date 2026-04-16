use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;
use uuid::Uuid;

pub const COLOR_PALETTE: [[u8; 3]; 10] = [
    [231, 76, 60],   // red
    [46, 204, 113],  // green
    [52, 152, 219],  // blue
    [241, 196, 15],  // yellow
    [155, 89, 182],  // purple
    [230, 126, 34],  // orange
    [26, 188, 156],  // teal
    [236, 112, 160], // pink
    [149, 165, 166], // grey
    [52, 73, 94],    // dark blue
];

#[derive(Clone)]
pub struct AppState {
    pub sessions: Arc<DashMap<String, Arc<SessionHandle>>>,
    pub connections: Arc<DashMap<Uuid, mpsc::Sender<String>>>,
    /// Maps connection_id → (session_id, user_id) for session-scoped broadcast routing.
    pub connection_meta: Arc<DashMap<Uuid, (String, Uuid)>>,
    /// Per-connection lag tracking for bounded queue backpressure decisions.
    pub connection_backpressure: Arc<DashMap<Uuid, LagState>>,
    pub session_connections: Arc<DashMap<String, HashSet<Uuid>>>,
}

#[derive(Clone, Debug)]
pub struct LagState {
    pub strikes: u8,
    pub last_full_at_ms: u64,
}

// Session related logic is simplified by using a separate struct that contains RwLocks for interior mutability,
// which the AppState holds Arc references to for shared ownership across connections.
pub struct SessionHandle {
    pub objects: RwLock<HashMap<Uuid, SceneObject>>,
    pub users: RwLock<HashMap<Uuid, User>>,
    pub session_id: String,
    pub password_hash: String, 
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Session {
    pub session_id: String,
    pub objects: HashMap<Uuid, SceneObject>,
    pub users: HashMap<Uuid, User>,
}

// Helper function to handle poisoned locks by logging a warning and recovering with a new lock containing default data. This prevents the entire session from becoming inaccessible due to one poisoned lock, at the cost of potentially losing some data.
fn recover_read<T: Clone>(lock: &std::sync::RwLock<T>, name: &str) -> T {
    match lock.read() {
        Ok(guard) => guard.clone(),
        Err(poisoned) => {
            tracing::warn!(lock = name, "RwLock poisoned, recovering anyway");
            poisoned.into_inner().clone()
        }
    }
}

// Build a session by acquiring read locks on the SessionHandle's internal state and cloning the data into a new Session struct.
impl SessionHandle {
    pub fn session_snapshot(&self) -> Session {
        // If any of the locks are poisoned, we log a warning and recover by creating a new lock with empty data. This prevents the entire session from becoming inaccessible due to one poisoned lock, at the cost of potentially losing some data.
        let objects = recover_read(&self.objects, "objects");
        let users = recover_read(&self.users, "users");
        let session_id = self.session_id.clone();
        Session {
            session_id,
            objects,
            users,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Transform {
    pub position: [f64; 3], // initializing an array of 3 float64 values (xyz)
    pub rotation: [f64; 3],
    pub scale: [f64; 3],
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
#[serde(rename_all = "UPPERCASE")]
pub enum SensorFit {
    // blender sends the camera properties in all caps
    Auto,
    Horizontal,
    Vertical,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CameraProperties {
    // Lens
    pub lens_type: LensType,
    pub focal_length: f64,       // mm, used when perspective
    pub orthographic_scale: f64, // used when orthographic
    pub shift_x: f64,
    pub shift_y: f64,
    pub clip_start: f64,
    pub clip_end: f64,
    // Depth of Field
    pub focal_distance: f64,
    pub aperture_fstop: f64,
    pub aperture_blades: u32,
    pub aperture_rotation: f64,
    pub aperture_ratio: f64,
    // Sensor
    pub sensor_fit: SensorFit,
    pub sensor_width: f64,
    pub sensor_height: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PointLightProperties {
    pub color: [f32; 3],
    pub temperature: f32, // Kelvin
    pub exposure: f32,
    pub power: f32,  // watts
    pub radius: f32, // sphere radius for soft shadows
    pub soft_falloff: bool,
    pub normalize: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SpotLightProperties {
    pub color: [f32; 3],
    pub temperature: f32,
    pub exposure: f32,
    pub normalize: bool,
    pub power: f32,
    pub radius: f32,
    pub soft_falloff: bool,
    pub angle: f32,
    pub blend: f32,
    pub show_cone: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AreaLightProperties {
    // Common
    pub color: [f32; 3],
    pub temperature: f32,
    pub exposure: f32,
    pub normalize: bool,
    // Area specific
    pub power: f32,
    pub shape: AreaLightShape,
    pub size_x: f32, // used by Rectangle and Ellipse
    pub size_y: f32, // used by Rectangle and Ellipse
    pub size: f32,   // used by Square and Disk
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum AreaLightShape {
    // different shapes that an area light can have since blender has fixed options for the shape of an area light
    Rectangle,
    Square,
    Disk,
    Ellipse,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SunLightProperties {
    // Common
    pub color: [f32; 3],
    pub temperature: f32,
    pub exposure: f32,
    pub normalize: bool,
    // Sun specific
    pub strength: f32,
    pub angle: f32,
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
    pub object_id: Uuid,
    pub name: String,
    pub object_type: ObjectType,
    pub asset_id: Option<String>,      // None for non-asset objects
    pub asset_library: Option<String>, // None for non-asset objects
    pub transform: Transform,
    pub properties: Option<ObjectProperties>, // None for primitives/asset refs
    pub created_by: Uuid,
    pub last_updated_by: Uuid,
    pub last_updated_at: u64, // unix timestamp ms
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct User {
    pub display_name: String,
    pub color: [u8; 3],
    pub selected_object: Option<Uuid>,
    pub connected_at: u64, // timestamp
}
