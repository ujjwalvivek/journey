/**----------------------------------------------------------------------------
*!  Texture asset management.
*?  Manages all textures in the game. Loads textures using Texture,
*?  stores them, and provides handles for safe access.
*?  Manages bind groups for shader access for loading/creating a single GPU texture.
*-----------------------------------------------------------------------------**/
use crate::texture::Texture;

//? Handle to a loaded texture. Encapsulates the index into the
//? TextureManager's texture and bind group arrays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureHandle(pub(crate) usize);

//? Texture asset manager.
pub struct TextureManager {
    pub(crate) textures: Vec<Texture>,
    pub(crate) bind_groups: Vec<wgpu::BindGroup>,
}

//? Methods for loading textures and retrieving them by handle.
impl TextureManager {
    #[allow(dead_code)]
    pub(crate) fn new() -> Self {
        Self {
            textures: Vec::new(),
            bind_groups: Vec::new(),
        }
    }

    //? Load a texture from image bytes and create a corresponding bind group.
    //* Returns a TextureHandle that can be used to retrieve the texture and bind group later.
    pub fn load_from_bytes(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bind_group_layout: &wgpu::BindGroupLayout,
        bytes: &[u8],
        label: Option<&str>,
    ) -> Result<TextureHandle, image::ImageError> {
        let texture = Texture::from_bytes(device, queue, bytes, label)?;

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label,
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
        });

        let handle = TextureHandle(self.textures.len());
        self.textures.push(texture);
        self.bind_groups.push(bind_group);

        Ok(handle)
    }

    pub fn get_texture(&self, handle: TextureHandle) -> Option<&Texture> {
        self.textures.get(handle.0)
    }

    #[allow(dead_code)]
    pub(crate) fn get_bind_group(&self, handle: TextureHandle) -> Option<&wgpu::BindGroup> {
        self.bind_groups.get(handle.0)
    }
}
