/**------------------------------------------------------------------------
*!  Orthographic camera for 2D rendering with pixel-space coordinates.
*------------------------------------------------------------------------**/
use bytemuck::{Pod, Zeroable};
use glam::Mat4;

//? Camera uniform buffer data sent to GPU shaders.
//* #[repr(C)] ensures the struct's memory layout matches C conventions.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
}

//? Convert a glam::Mat4 (Rust math type) into a plain 2D array, what the GPU expects.
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

    //* Horizontal pan offset in pixels. Allows to follow the player.
    pub offset_x: f32,

    //* Cached view-projection matrix and uniform data for GPU upload.
    view_proj: Mat4,
    uniform: CameraUniform,
}

impl Camera {
    //* Create a new camera with given size, no offset.
    //* Computes the initial projection matrix and uniform.
    pub fn new(width: f32, height: f32) -> Self {
        let view_proj = Self::build_projection(width, height, 0.0);
        let uniform = CameraUniform::new(view_proj);

        Self {
            width,
            height,
            offset_x: 0.0,
            view_proj,
            uniform,
        }
    }

    //* Update camera size and recomputes projection and uniform.
    //? &mut self - can change self.width and self.height.
    pub fn resize(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
        self.view_proj = Self::build_projection(width, height, self.offset_x);
        self.uniform = CameraUniform::new(self.view_proj);
    }

    //* Update horizontal offset to follow a player and recomputes projection and uniform.
    //? &mut self - can change self.offset_x.
    pub fn set_offset(&mut self, offset_x: f32) {
        self.offset_x = offset_x;
        self.view_proj = Self::build_projection(self.width, self.height, self.offset_x);
        self.uniform = CameraUniform::new(self.view_proj);
    }

    //* Get the uniform data for uploading to GPU.
    //? &self - only reads self.uniform, returns reference to it.
    pub fn uniform(&self) -> &CameraUniform {
        &self.uniform
    }

    //* Build orthographic projection matrix.
    //* Maps (0, 0) at top-left to (width, height) at bottom-right → NDC [-1, 1].
    //? Shift the view by offset_x to follow the player
    fn build_projection(width: f32, height: f32, offset_x: f32) -> Mat4 {
        Mat4::orthographic_rh(offset_x, width + offset_x, height, 0.0, -1.0, 1.0)
    }
}
