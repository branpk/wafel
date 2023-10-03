#[derive(Debug, Default)]
pub struct CachedTexture(Option<(wgpu::TextureDescriptor<'static>, wgpu::Texture)>);

impl CachedTexture {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn matches(&self, descriptor: &wgpu::TextureDescriptor<'static>) -> bool {
        if let Some((cached_descriptor, _)) = &self.0 {
            if cached_descriptor == descriptor {
                return true;
            }
        }
        false
    }

    pub fn get(
        &mut self,
        device: &wgpu::Device,
        descriptor: &wgpu::TextureDescriptor<'static>,
    ) -> &wgpu::Texture {
        if !self.matches(descriptor) {
            let texture = device.create_texture(descriptor);
            self.0 = Some((descriptor.clone(), texture));
        }
        &self.0.as_ref().unwrap().1
    }
}
