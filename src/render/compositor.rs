use super::tile_cache::Tile;

/// Structural wgpu compositor. Full GPU pipeline creation is deferred to
/// Phase 6 (Windowing) since it requires a winit window/surface.
pub struct WgpuCompositor {
    viewport_width: u32,
    viewport_height: u32,
    initialized: bool,
}

impl WgpuCompositor {
    /// Create a new compositor for the given viewport dimensions.
    pub fn new(viewport_width: u32, viewport_height: u32) -> Self {
        Self {
            viewport_width,
            viewport_height,
            initialized: false,
        }
    }

    /// Initialize the GPU resources. In this structural implementation,
    /// just marks as initialized. Real wgpu setup happens in Phase 6.
    pub fn init(&mut self) {
        // Phase 6 will:
        // 1. Create wgpu instance, adapter, device, queue
        // 2. Create texture atlas for tile upload
        // 3. Create render pipeline with vertex/fragment shaders
        // 4. Create bind groups for tile textures
        self.initialized = true;
    }

    /// Check if the compositor has been initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Upload a tile's pixel data to the GPU texture atlas.
    /// Stub — actual GPU upload happens in Phase 6.
    pub fn upload_tile(&mut self, _col: u32, _row: u32, _tile: &Tile) {
        // Phase 6 will write tile.image data to GPU texture
    }

    /// Render a frame by compositing all visible tiles.
    /// Stub — actual GPU rendering happens in Phase 6.
    pub fn render_frame(&mut self) {
        // Phase 6 will:
        // 1. Begin render pass
        // 2. Draw quads for each visible tile
        // 3. Submit command buffer
    }

    /// Handle viewport resize.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.viewport_width = width;
        self.viewport_height = height;
        // Phase 6: recreate surface/swapchain
    }

    /// Get the viewport dimensions.
    pub fn viewport_size(&self) -> (u32, u32) {
        (self.viewport_width, self.viewport_height)
    }
}
