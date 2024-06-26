use glm::Mat3;
use glm::Mat4;
use glm::Vec3;
use glm::Vec4;
use indicatif::ProgressIterator;
use itertools::Itertools;
use rand::rngs::ThreadRng;
use rand::Rng;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;

use crate::geometry::geometry::Geometry;
use crate::math::interval::Interval;
use crate::ray::ray::Ray;

pub struct Camera {
    image_width: u32,
    image_height: u32,
    position: Vec3,
    raster_to_camera: Mat3,
    camera_to_world: Mat4,
}

impl Camera {
    pub fn new(image_width: u32, image_height: u32, fov: f32, position: Vec3) -> Self {
        let scale_y = 1.0 / image_height as f32;
        let scale_x = 1.0 / image_width as f32;
        let aspect_ratio = image_width as f32 / image_height as f32;
        let theta = fov.to_radians();
        let h = (theta / 2.0).tan();

        #[rustfmt::skip]
        // NDC: Normalized Device Coordinates.
        // X axis: (0..X) remapped to (0..1).
        // Y axis: (0..Y) remapped to (0..1).
        let raster_to_ndc = Mat3::new(
            scale_x, 0.0,     0.0,
            0.0,     scale_y, 0.0,
            0.0,     0.0,     1.0,
        );

        #[rustfmt::skip]
        // X axis: (0..1) remapped to (-1..1).
        // Y axis: (0..1) remapped to (1..-1).
        let ndc_to_screen = Mat3::new(
            2.0,  0.0, -1.0,
            0.0, -2.0,  1.0,
            0.0,  0.0,  1.0,
        );

        #[rustfmt::skip]
        // AR: width / height.
        // H: tan(fov / 2).
        // X axis: (-1..1) remapped to (-AR*H..AR*H).
        // Y axis: (1..-1) remapped to (H..-H).
        let screen_to_camera = Mat3::new(
            aspect_ratio * h, 0.0, 0.0,
            0.0,              h,   0.0,
            0.0,              0.0, 1.0,
        );

        let raster_to_camera = screen_to_camera * ndc_to_screen * raster_to_ndc;
        let camera_to_world = glm::translation(&position);

        Camera {
            image_width,
            image_height,
            position,
            raster_to_camera,
            camera_to_world,
        }
    }

    pub fn lookat(&mut self, lookat: Vec3) {
        let arbitrary_up = Vec3::new(0.0, 1.0, 0.0);
        let forward = (self.position - lookat).normalize();

        if glm::are_collinear(&forward, &arbitrary_up, glm::epsilon()) {
            panic!("The view direction and up vector are collinear")
        }

        let right = glm::cross(&arbitrary_up, &forward).normalize();
        let up = glm::cross(&forward, &right);
        let camera_to_world = Mat4::from_columns(&[
            glm::vec3_to_vec4(&right),
            glm::vec3_to_vec4(&up),
            glm::vec3_to_vec4(&forward),
            Vec4::new(self.position.x, self.position.y, self.position.z, 1.0),
        ]);
        self.camera_to_world = camera_to_world;
    }

    pub fn render<T: Geometry>(&self, world: &T, max_depth: u32, samples_per_pixel: u32) {
        // Print metadata
        println!("P3\n{} {}\n255", self.image_width, self.image_height);

        let pixels = (0..self.image_height)
            .progress()
            .cartesian_product(0..self.image_width)
            .map(|(y, x)| {
                let pixel_color: Vec3 = (0..samples_per_pixel)
                    .into_par_iter()
                    .map(|_| {
                        let mut rng = ThreadRng::default();
                        let ray = self.get_ray(&mut rng, x, y);
                        Camera::ray_color(&ray, world, max_depth)
                    })
                    .sum();
                pixel_color / samples_per_pixel as f32
            });

        for pixel_color in pixels {
            Camera::write_color(pixel_color);
        }
    }

    fn ray_color<T: Geometry>(ray: &Ray, world: &T, depth: u32) -> Vec3 {
        if depth <= 0 {
            Vec3::repeat(0.0)
        } else if let Some(hit_record) = world.hit(&ray, &Interval::new(0.001, f32::MAX)) {
            if let Some((scattered_ray, attenuation)) =
                hit_record.material.scatter(ray, &hit_record)
            {
                attenuation.component_mul(&Camera::ray_color(&scattered_ray, world, depth - 1))
            } else {
                Vec3::repeat(0.0)
            }
        } else {
            let unit_direction = ray.direction;
            let a = 0.5 * (unit_direction.y + 1.0);
            glm::lerp(&Vec3::repeat(1.0), &Vec3::new(0.5, 0.7, 1.0), a)
        }
    }

    fn get_ray<T: Rng>(&self, rng: &mut T, x: u32, y: u32) -> Ray {
        let offset_x: f32 = rng.gen();
        let offset_y: f32 = rng.gen();
        let p_screen = Vec3::new(x as f32 + offset_x, y as f32 + offset_y, 1.0);
        let p_camera = self.raster_to_camera * p_screen;
        let direction = self.camera_to_world * Vec4::new(p_camera.x, p_camera.y, -1.0, 0.0);
        Ray::new(self.position, direction.xyz().normalize())
    }

    fn write_color(color: Vec3) {
        let gamma_corrected = glm::sqrt(&color);
        let rgb_color = glm::clamp(&gamma_corrected, 0.0, 1.0) * 255.0;
        println!(
            "{} {} {}",
            rgb_color.x as u32, rgb_color.y as u32, rgb_color.z as u32
        );
    }
}

#[cfg(test)]
mod tests {
    use rand::rngs::mock::StepRng;

    use super::*;

    #[test]
    fn square_camera_test_00() {
        let mut rng = StepRng::new(0, 0);
        let camera = Camera::new(10, 10, 90.0, Vec3::repeat(0.0));
        let ray = camera.get_ray(&mut rng, 0, 0);

        let expected = Vec3::new(-1.0, 1.0, -1.0).normalize();
        let given = ray.direction;

        assert!(
            glm::equal_eps(&expected, &given, glm::epsilon())
                .iter()
                .all(|&x| x),
            "expected {:?}, given {:?}",
            expected,
            given
        )
    }

    #[test]
    fn square_camera_test99() {
        let mut rng = StepRng::new(0, 0);
        let camera = Camera::new(10, 10, 90.0, Vec3::repeat(0.0));
        let ray = camera.get_ray(&mut rng, 9, 9);

        let expected = Vec3::new(0.8, -0.8, -1.0).normalize();
        let given = ray.direction;

        assert!(
            glm::equal_eps(&expected, &given, glm::epsilon())
                .iter()
                .all(|&x| x),
            "expected {:?}, given {:?}",
            expected,
            given
        )
    }

    #[test]
    fn rectangular_camera_test00() {
        let mut rng = StepRng::new(0, 0);
        let camera = Camera::new(20, 10, 90.0, Vec3::repeat(0.0));
        let ray = camera.get_ray(&mut rng, 0, 0);

        let expected = Vec3::new(-2.0, 1.0, -1.0).normalize();
        let given = ray.direction;

        assert!(
            glm::equal_eps(&expected, &given, glm::epsilon())
                .iter()
                .all(|&x| x),
            "expected {:?}, given {:?}",
            expected,
            given
        )
    }

    #[test]
    fn rectangular_camera_test99() {
        let mut rng = StepRng::new(0, 0);
        let camera = Camera::new(20, 10, 90.0, Vec3::repeat(0.0));
        let ray = camera.get_ray(&mut rng, 19, 9);

        let expected = Vec3::new(1.8, -0.8, -1.0).normalize();
        let given = ray.direction;

        assert!(
            glm::equal_eps(&expected, &given, glm::epsilon())
                .iter()
                .all(|&x| x),
            "expected {:?}, given {:?}",
            expected,
            given
        )
    }
}
