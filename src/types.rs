use std::sync::{Arc, RwLock};
use nalgebra::Vector3;

pub type Color = Vector3<f32>;
pub type Shared<T> = Arc<RwLock<T>>;
pub type RGB = [f32; 3];

pub fn create_shared_mut<T>(t: T) -> Shared<T> {
    Arc::new(RwLock::new(t))
}