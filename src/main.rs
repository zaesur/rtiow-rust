extern crate nalgebra_glm as glm;

mod camera;
mod geometry;
mod material;
mod math;

use camera::camera::Camera;
use geometry::{sphere::Sphere, world::World};
use glm::Vec3;
use material::{dielectric::Dielectric, lambertian::Lambertian, metal::Metal};

const ASPECT_RATIO: f32 = 16.0 / 9.0;
const IMAGE_WIDTH: u32 = 400;
const FOCAL_LENGTH: f32 = 1.0;
const SAMPLES_PER_PIXEL: u32 = 30;
const MAX_DEPTH: u32 = 10;

fn main() {
    let ground = Lambertian::new(Vec3::new(0.8, 0.8, 0.0));
    let material_center = Lambertian::new(Vec3::new(0.1, 0.2, 0.5));
    let material_left = Dielectric::new(1.0 / 1.33);
    let material_right = Metal::new(Vec3::new(0.8, 0.6, 0.2), 1.0);

    let world = World::new(vec![
        Box::new(Sphere::new(Vec3::new(0.0, -100.5, -1.0), 100.0, ground)),
        Box::new(Sphere::new(Vec3::new(0.0, 0.0, -1.2), 0.5, material_center)),
        Box::new(Sphere::new(Vec3::new(-1.0, 0.0, -1.0), 0.5, material_left)),
        Box::new(Sphere::new(Vec3::new(1.0, 0.0, -1.0), 0.5, material_right)),
    ]);

    let camera = Camera::new(
        ASPECT_RATIO,
        IMAGE_WIDTH,
        FOCAL_LENGTH,
        SAMPLES_PER_PIXEL,
        MAX_DEPTH,
    );

    camera.render(&world)
}
