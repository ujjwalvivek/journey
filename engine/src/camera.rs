/**------------------------------------------------------------------------
*!  Orthographic camera for 2D rendering with pixel-space coordinates.
*------------------------------------------------------------------------**/
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec2};

//? Y-axis frequency multiplier: creates a Lissajous-like orbit that never
//? repeats cleanly, giving screen shakes an organic, non-mechanical feel.
const SHAKE_Y_FREQUENCY_RATIO: f32 = 1.3;

//? Decaying sinusoidal screen shake triggered by impacts.
#[derive(Debug, Clone)]
pub struct ScreenShake {
    pub intensity: f32,
    pub duration: f32,
    pub frequency: f32,
    pub decay: f32,
    pub elapsed: f32,
}

impl ScreenShake {
    pub fn new(intensity: f32, duration: f32) -> Self {
        Self {
            intensity,
            duration,
            frequency: 40.0,
            decay: 8.0,
            elapsed: 0.0,
        }
    }

    //? Sample the current shake offset. Returns (0,0) when finished.
    pub fn sample(&self) -> Vec2 {
        if self.elapsed >= self.duration {
            return Vec2::ZERO;
        }
        let t = self.elapsed;
        let envelope = self.intensity * (-self.decay * t).exp();
        Vec2::new(
            envelope * (self.frequency * t).sin(),
            envelope * (self.frequency * t * SHAKE_Y_FREQUENCY_RATIO).cos(),
        )
    }

    //? Advance the shake by dt. Returns true if still active.
    pub fn update(&mut self, dt: f32) -> bool {
        self.elapsed += dt;
        self.elapsed < self.duration
    }

    pub fn is_active(&self) -> bool {
        self.elapsed < self.duration
    }
}

//? Camera uniform buffer data sent to GPU shaders.
//* #[repr(C)] ensures the struct's memory layout matches C conventions.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
}

//? Convert a glam::Mat4 into a plain 2D array, what the GPU expects.
impl CameraUniform {
    pub fn new(view_proj: Mat4) -> Self {
        Self {
            view_proj: view_proj.to_cols_array_2d(),
        }
    }
}

//? Orthographic camera mapping pixel coordinates to Normalized Device Coordinates space.
//? (NDC: -1 to 1 in X and Y). The camera can pan horizontally by adjusting offset_x.
pub struct Camera {
    //* Camera's viewport in pixels.
    pub width: f32,
    pub height: f32,

    //* Pan offsets in pixels. Allows camera to follow the player.
    pub offset_x: f32,
    pub offset_y: f32,

    //* Active screen shakes.
    pub shakes: Vec<ScreenShake>,

    //* Cached view-projection matrix and uniform data for GPU upload.
    view_proj: Mat4,
    uniform: CameraUniform,
}

impl Camera {
    //* Create a new camera with given size, no offset.
    //* Computes the initial projection matrix and uniform.
    pub fn new(width: f32, height: f32) -> Self {
        let view_proj = Self::build_projection(width, height, 0.0, 0.0);
        let uniform = CameraUniform::new(view_proj);

        Self {
            width,
            height,
            offset_x: 0.0,
            offset_y: 0.0,
            shakes: Vec::new(),
            view_proj,
            uniform,
        }
    }

    //* Update camera size and recomputes projection and uniform.
    //? &mut self - can change self.width and self.height.
    pub fn resize(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
        let shake = self.total_shake();
        self.view_proj = Self::build_projection(
            width,
            height,
            self.offset_x + shake.x,
            self.offset_y + shake.y,
        );
        self.uniform = CameraUniform::new(self.view_proj);
    }

    //* Update horizontal offset to follow a player and recomputes projection and uniform.
    //? &mut self - can change self.offset_x.
    pub fn set_offset(&mut self, offset_x: f32, offset_y: f32) {
        self.offset_x = offset_x;
        self.offset_y = offset_y;
        let shake = self.total_shake();
        self.view_proj = Self::build_projection(
            self.width,
            self.height,
            self.offset_x + shake.x,
            self.offset_y + shake.y,
        );
        self.uniform = CameraUniform::new(self.view_proj);
    }

    //? Add a new screen shake. Multiple shakes stack additively.
    pub fn add_shake(&mut self, intensity: f32, duration: f32) {
        self.shakes.push(ScreenShake::new(intensity, duration));
    }

    //? Advance all active shakes by dt, remove finished ones.
    pub fn update_shakes(&mut self, dt: f32) {
        for shake in &mut self.shakes {
            shake.update(dt);
        }
        self.shakes.retain(|s| s.is_active());
    }

    //? Sum of all active shake offsets.
    fn total_shake(&self) -> Vec2 {
        self.shakes.iter().map(|s| s.sample()).sum()
    }

    //* Get the uniform data for uploading to GPU.
    //? &self - only reads self.uniform, returns reference to it.
    pub fn uniform(&self) -> &CameraUniform {
        &self.uniform
    }

    //* Build orthographic projection matrix.
    //* Maps (0, 0) at top-left to (width, height) at bottom-right → NDC [-1, 1].
    //? Shift the view by offset_x to follow the player
    fn build_projection(width: f32, height: f32, offset_x: f32, offset_y: f32) -> Mat4 {
        Mat4::orthographic_rh(
            offset_x,
            width + offset_x,
            height + offset_y,
            offset_y,
            -1.0,
            1.0,
        )
    }
}
