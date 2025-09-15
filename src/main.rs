use eframe::egui;
use image::{DynamicImage, GenericImageView};
use lru::LruCache;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use walkdir::WalkDir;

#[derive(Clone)]
struct CachedImage {
    display_image: DynamicImage,
    texture: Option<egui::TextureHandle>,
    rotation: u32,
}

struct ImageViewer {
    images: Vec<PathBuf>,
    current_index: usize,
    current_image: Option<DynamicImage>,
    loading_image: Option<tokio::task::JoinHandle<Option<DynamicImage>>>,
    image_cache: Arc<std::sync::Mutex<LruCache<PathBuf, CachedImage>>>,
    preload_handles: HashMap<PathBuf, tokio::task::JoinHandle<()>>,
    // Delete confirmation state
    delete_pending: bool,
    delete_timestamp: Option<std::time::Instant>,
    show_delete_confirm: bool,
    image_to_delete: Option<PathBuf>,
}

impl ImageViewer {
    fn new(path: PathBuf) -> Self {
        let images = Self::scan_images(&path);

        let mut viewer = Self {
            images,
            current_index: 0,
            current_image: None,
            loading_image: None,
            image_cache: Arc::new(Mutex::new(LruCache::new(std::num::NonZeroUsize::new(100).unwrap()))), // Cache up to 100 images
            preload_handles: HashMap::new(),
            // Initialize delete state
            delete_pending: false,
            delete_timestamp: None,
            show_delete_confirm: false,
            image_to_delete: None,
        };

        if !viewer.images.is_empty() {
            viewer.load_current_image();
            viewer.preload_adjacent_images();
        }
        viewer
    }

    fn scan_images(path: &PathBuf) -> Vec<PathBuf> {
        WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.path().to_path_buf())
            .filter(|p| {
                p.extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| matches!(ext.to_lowercase().as_str(), "jpg" | "jpeg" | "png" | "gif" | "bmp"))
                    .unwrap_or(false)
            })
            .collect()
    }

    fn load_current_image(&mut self) {
        if let Some(path) = self.images.get(self.current_index) {
            let cache = self.image_cache.clone();
            let path_clone = path.clone();

            // Check if image is cached first
            let is_cached = {
                let cache = cache.lock().unwrap();
                cache.contains(&path_clone)
            };

            if is_cached {
                self.current_image = {
                    let mut cache = cache.lock().unwrap();
                    cache.get(&path_clone).map(|cached| cached.display_image.clone())
                };
                self.loading_image = None; // Clear any pending load
            } else {
                // Start async loading if not cached
                if self.loading_image.is_none() || self.loading_image.as_ref().unwrap().is_finished() {
                    let cache_clone = cache.clone();
                    self.loading_image = Some(tokio::spawn(async move {
                        Self::load_and_cache_image_async(cache_clone, path_clone).await
                    }));
                }
            }
        }
    }

    async fn load_and_cache_image_async(cache: Arc<std::sync::Mutex<LruCache<PathBuf, CachedImage>>>, path: PathBuf) -> Option<DynamicImage> {
        if let Ok(img) = image::open(&path) {
            let display_img = Self::resize_for_display_static(&img);
let cached = CachedImage {
                    display_image: display_img.clone(),
                    texture: None,
                    rotation: 0,
                };


            let mut cache = cache.lock().unwrap();
            cache.put(path, cached);
            Self::manage_memory_usage_sync(&mut cache);
            Some(display_img)
        } else {
            None
        }
    }

    fn check_loading_complete(&mut self) {
        if let Some(handle) = self.loading_image.take() {
            if handle.is_finished() {
                self.current_image = futures::executor::block_on(handle).unwrap_or(None);
            } else {
                self.loading_image = Some(handle); // Put it back if not finished
            }
        }
    }

    fn resize_for_display_static(img: &DynamicImage) -> DynamicImage {
        let (w, h) = img.dimensions();
        let max_size = 1920.0f32;
        let scale = if w > h {
            max_size / w as f32
        } else {
            max_size / h as f32
        }.min(1.0);

        let new_w = (w as f32 * scale) as u32;
        let new_h = (h as f32 * scale) as u32;

        img.resize(new_w, new_h, image::imageops::FilterType::Lanczos3)
    }

    fn manage_memory_usage_sync(cache: &mut LruCache<PathBuf, CachedImage>) {
        // If cache is getting too large, remove oldest entries
        while cache.len() > 80 { // Keep only 80 most recently used
            if let Some((_, _)) = cache.pop_lru() {
                // Entry removed, continue
            } else {
                break;
            }
        }
    }





    fn cleanup_textures(&mut self, _ctx: &egui::Context) {
        // Remove textures for images that are no longer in cache
        // This is called periodically to free GPU memory
        let cache = self.image_cache.clone();
        {
            let cache = cache.lock().unwrap();
            let _current_paths: std::collections::HashSet<_> = cache.iter().map(|(path, _)| path.clone()).collect();
        }

        // Note: egui doesn't provide a direct way to free textures, but they get cleaned up automatically
        // when the context is recreated or when the application exits
    }



    fn preload_adjacent_images(&mut self) {
        if self.images.is_empty() {
            return;
        }

        // Preload more images for faster navigation
        let mut indices_to_preload = Vec::new();

        // Preload next 5 images
        for i in 1..=5 {
            indices_to_preload.push((self.current_index + i) % self.images.len());
        }

        // Preload previous 3 images
        for i in 1..=3 {
            if self.current_index >= i {
                indices_to_preload.push(self.current_index - i);
            } else {
                indices_to_preload.push(self.images.len() - i + self.current_index);
            }
        }

        let cache = self.image_cache.clone();

        for idx in indices_to_preload {
            if let Some(path) = self.images.get(idx) {
                let path_clone = path.clone();
                let cache_clone = cache.clone();

                // Check if already cached or being preloaded
                let already_cached = {
                    let cache = cache_clone.lock().unwrap();
                    cache.contains(&path_clone)
                };

                if !already_cached && !self.preload_handles.contains_key(&path_clone) {
                    let path_for_async = path.clone();
                    let handle = tokio::spawn(async move {
                        let cache = cache_clone;
                        if let Ok(img) = image::open(&path_for_async) {
                            // Pre-calculate display size
                            let (w, h) = img.dimensions();
                            let max_size = 1920.0f32;
                            let scale = if w > h {
                                max_size / w as f32
                            } else {
                                max_size / h as f32
                            }.min(1.0);

                            let new_w = (w as f32 * scale) as u32;
                            let new_h = (h as f32 * scale) as u32;

                            let display_img = img.resize(new_w, new_h, image::imageops::FilterType::Lanczos3);

                            // Actually cache the result
                            let mut cache = cache.lock().unwrap();
                            let cached = CachedImage {
                                display_image: display_img,
                                texture: None,
                                rotation: 0,
                            };
                            cache.put(path_for_async, cached);

                            // Manage memory usage
                            Self::manage_memory_usage_sync(&mut cache);
                        }
                    });
                    self.preload_handles.insert(path_clone, handle);
                }
            }
        }

        // Clean up old preload handles
        self.preload_handles.retain(|_, handle| !handle.is_finished());
    }



    fn next_image(&mut self) {
        if !self.images.is_empty() {
            // Cancel any pending load
            if let Some(handle) = self.loading_image.take() {
                handle.abort();
            }
            self.current_image = None;

            self.current_index = (self.current_index + 1) % self.images.len();
            self.load_current_image();
            self.preload_adjacent_images();
        }
    }

    fn prev_image(&mut self) {
        if !self.images.is_empty() {
            // Cancel any pending load
            if let Some(handle) = self.loading_image.take() {
                handle.abort();
            }
            self.current_image = None;

            self.current_index = if self.current_index == 0 {
                self.images.len() - 1
            } else {
                self.current_index - 1
            };
            self.load_current_image();
            self.preload_adjacent_images();
        }
    }

    fn delete_image(&mut self, path: &PathBuf) -> Result<(), std::io::Error> {
        std::fs::remove_file(path)?;
        // Remove from cache if present
        {
            let mut cache = self.image_cache.lock().unwrap();
            cache.pop(path);
        }
        // Remove from preload handles if present
        self.preload_handles.remove(path);
        Ok(())
    }

    fn update_image_list_after_delete(&mut self) {
        if self.images.is_empty() {
            return;
        }

        // Remove the deleted image from the list
        if let Some(pos) = self.images.iter().position(|p| Some(p) == self.image_to_delete.as_ref()) {
            self.images.remove(pos);

            // Adjust current_index if necessary
            if pos < self.current_index {
                self.current_index = self.current_index.saturating_sub(1);
            } else if pos == self.current_index {
                // If we deleted the current image, stay at the same index
                // (which now points to the next image)
                if self.current_index >= self.images.len() && self.current_index > 0 {
                    self.current_index = self.current_index.saturating_sub(1);
                }
            }

            // If no images left, reset state
            if self.images.is_empty() {
                self.current_index = 0;
                self.current_image = None;
                self.loading_image = None;
            } else {
                // Reload current image
                self.current_image = None;
                self.loading_image = None;
                self.load_current_image();
                self.preload_adjacent_images();
            }
        }

        // Reset delete state
        self.show_delete_confirm = false;
        self.image_to_delete = None;
    }

    fn rotate_current_image(&mut self) {
        if let Some(path) = self.images.get(self.current_index) {
            let cache = self.image_cache.clone();
            let path_clone = path.clone();

            let mut cache = cache.lock().unwrap();
            if let Some(cached) = cache.get_mut(&path_clone) {
                // Increment rotation by 90 degrees clockwise
                cached.rotation = (cached.rotation + 90) % 360;
                
                // Clear the texture so it gets recreated with the new rotation
                cached.texture = None;
            }
        }
    }
}

impl eframe::App for ImageViewer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check if any async loading has completed
        self.check_loading_complete();

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(img) = &self.current_image {
                let size = img.dimensions();

                // Get or create texture for current image
                let texture_id = if let Some(path) = self.images.get(self.current_index) {
                    let cache = self.image_cache.clone();
                    let path_clone = path.clone();

                    {
                        let mut cache = cache.lock().unwrap();
                        if let Some(cached) = cache.get_mut(&path_clone) {
                            // Apply rotation if necessary when creating texture
                            let mut display_img = cached.display_image.clone();
                            if cached.rotation != 0 {
                                display_img = match cached.rotation % 360 {
                                    90 => display_img.rotate90(),
                                    180 => display_img.rotate180(),
                                    270 => display_img.rotate270(),
                                    _ => display_img,
                                };
                            }

                            if let Some(texture) = &cached.texture {
                                texture.id()
                            } else {
                                // Create and cache texture
                                let texture = ctx.load_texture(
                                    &format!("image_{}", self.current_index),
                                    egui::ColorImage::from_rgba_unmultiplied(
                                        [display_img.width() as usize, display_img.height() as usize],
                                        &display_img.to_rgba8(),
                                    ),
                                    Default::default(),
                                );
                                let id = texture.id();
                                cached.texture = Some(texture);
                                id
                            }
                        } else {
                            // Fallback: create texture without caching
                            ctx.load_texture(
                                &format!("image_{}", self.current_index),
                                egui::ColorImage::from_rgba_unmultiplied(
                                    [size.0 as usize, size.1 as usize],
                                    &img.to_rgba8(),
                                ),
                                Default::default(),
                            ).id()
                        }
                    }
                } else {
                    // Fallback: create texture without caching
                    ctx.load_texture(
                        &format!("image_{}", self.current_index),
                        egui::ColorImage::from_rgba_unmultiplied(
                            [size.0 as usize, size.1 as usize],
                            &img.to_rgba8(),
                        ),
                        Default::default(),
                    ).id()
                };

                // Calculate aspect ratio and fit to available space
                let available_size = ui.available_size();
                
                // Get the actual display dimensions after rotation
                let (display_width, display_height) = if let Some(path) = self.images.get(self.current_index) {
                    let cache = self.image_cache.clone();
                    let path_clone = path.clone();
                    
                    {
                        let mut cache = cache.lock().unwrap();
                        if let Some(cached) = cache.get(&path_clone) {
                            // Apply rotation to get effective dimensions
                            let (w, h) = cached.display_image.dimensions();
                            match cached.rotation % 360 {
                                90 | 270 => (h, w), // Swap dimensions for 90° and 270° rotations
                                _ => (w, h), // Keep original dimensions for 0° and 180°
                            }
                        } else {
                            (size.0, size.1) // Fallback to original dimensions
                        }
                    }
                } else {
                    (size.0, size.1) // Fallback to original dimensions
                };
                
                let img_aspect = display_width as f32 / display_height as f32;
                let available_aspect = available_size.x / available_size.y;

                let display_size = if img_aspect > available_aspect {
                    // Image is wider, fit to width
                    egui::vec2(available_size.x, available_size.x / img_aspect)
                } else {
                    // Image is taller, fit to height
                    egui::vec2(available_size.y * img_aspect, available_size.y)
                };

                ui.centered_and_justified(|ui| {
                    ui.image((texture_id, display_size));
                });
            } else if self.loading_image.is_some() {
                ui.centered_and_justified(|ui| {
                    ui.label("Loading image...");
                    ui.add(egui::Spinner::new());
                });
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("No image loaded");
                });
            }
        });

        // Show delete confirmation dialog
        if self.show_delete_confirm {
            if let Some(path) = &self.image_to_delete {
                let path_clone = path.clone();
                let mut open = true;
                egui::Window::new("Confirm Delete")
                    .open(&mut open)
                    .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                    .show(ctx, |ui| {
                        ui.label(format!("Delete image: {}", path_clone.display()));
                        ui.label("This action cannot be undone.");
                        ui.separator();

                        ui.horizontal(|ui| {
                            if ui.button("Delete").clicked() {
                                if let Err(e) = self.delete_image(&path_clone) {
                                    eprintln!("Failed to delete image: {}", e);
                                } else {
                                    self.update_image_list_after_delete();
                                }
                            }

                            if ui.button("Cancel").clicked() {
                                self.show_delete_confirm = false;
                                self.image_to_delete = None;
                            }
                        });
                    });

                // Close dialog if user clicked outside or pressed escape
                if !open {
                    self.show_delete_confirm = false;
                    self.image_to_delete = None;
                }
            }
        }

        // Periodic cleanup (every 100 frames)
        static mut FRAME_COUNT: u64 = 0;
        unsafe {
            FRAME_COUNT += 1;
            if FRAME_COUNT % 100 == 0 {
                self.cleanup_textures(ctx);
            }
        }

        // Handle keyboard input
        if ctx.input(|i| i.key_pressed(egui::Key::J)) {
            self.next_image();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::K)) {
            self.prev_image();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::R)) {
            self.rotate_current_image();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Q)) {
            std::process::exit(0);
        }

        // Handle delete confirmation (dd like vim)
        if ctx.input(|i| i.key_pressed(egui::Key::D)) {
            let now = std::time::Instant::now();

            if self.delete_pending {
                // Check if second 'd' was pressed within 1 second
                if let Some(timestamp) = self.delete_timestamp {
                    if now.duration_since(timestamp).as_millis() < 1000 {
                        // Valid dd sequence - show confirmation
                        if let Some(path) = self.images.get(self.current_index) {
                            self.show_delete_confirm = true;
                            self.image_to_delete = Some(path.clone());
                        }
                    }
                }
                // Reset state
                self.delete_pending = false;
                self.delete_timestamp = None;
            } else {
                // First 'd' press
                self.delete_pending = true;
                self.delete_timestamp = Some(now);
            }
        }

        // Reset delete pending state if timeout (more than 1 second)
        if self.delete_pending {
            if let Some(timestamp) = self.delete_timestamp {
                if std::time::Instant::now().duration_since(timestamp).as_millis() >= 1000 {
                    self.delete_pending = false;
                    self.delete_timestamp = None;
                }
            }
        }
    }
}

fn main() -> Result<(), eframe::Error> {
    let args: Vec<String> = std::env::args().collect();
    let path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from(".")
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    // Block on the async runtime for eframe
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            eframe::run_native(
                "Image Viewer",
                options,
                Box::new(|_cc| Box::new(ImageViewer::new(path))),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::DynamicImage;

    #[test]
    fn test_rotation_initialization() {
        let img = DynamicImage::new_rgb8(100, 100);
        let cached = CachedImage {
            display_image: img,
            texture: None,
            rotation: 0,
        };
        
        assert_eq!(cached.rotation, 0);
    }

    #[test]
    fn test_rotation_math() {
        // Test rotation calculation logic
        let mut rotation = 0;
        
        // Test single rotation
        rotation = (rotation + 90) % 360;
        assert_eq!(rotation, 90);
        
        // Test multiple rotations
        rotation = (rotation + 90) % 360;
        assert_eq!(rotation, 180);
        
        rotation = (rotation + 90) % 360;
        assert_eq!(rotation, 270);
        
        // Test wrap-around
        rotation = (rotation + 90) % 360;
        assert_eq!(rotation, 0);
        
        // Test additional rotations
        rotation = (rotation + 90) % 360;
        assert_eq!(rotation, 90);
    }

    #[test]
    fn test_rotation_with_no_image() {
        let mut viewer = ImageViewer {
            images: Vec::new(),
            current_index: 0,
            current_image: None,
            loading_image: None,
            image_cache: Arc::new(Mutex::new(LruCache::new(std::num::NonZeroUsize::new(10).unwrap()))),
            preload_handles: HashMap::new(),
            delete_pending: false,
            delete_timestamp: None,
            show_delete_confirm: false,
            image_to_delete: None,
        };

        // This should not panic
        viewer.rotate_current_image();
    }

    #[test]
    fn test_aspect_ratio_calculation_after_rotation() {
        // Test that aspect ratio calculation correctly handles rotated dimensions
        let (orig_w, orig_h) = (1920, 1080); // Landscape image
        
        // Test 0° rotation (no change)
        let (display_w, display_h) = match 0 % 360 {
            90 | 270 => (orig_h, orig_w),
            _ => (orig_w, orig_h),
        };
        let aspect_0 = display_w as f32 / display_h as f32;
        assert!((aspect_0 - 1920.0/1080.0).abs() < 0.001);
        
        // Test 90° rotation (landscape becomes portrait)
        let (display_w, display_h) = match 90 % 360 {
            90 | 270 => (orig_h, orig_w),
            _ => (orig_w, orig_h),
        };
        let aspect_90 = display_w as f32 / display_h as f32;
        assert!((aspect_90 - 1080.0/1920.0).abs() < 0.001);
        
        // Test 180° rotation (no change to aspect ratio)
        let (display_w, display_h) = match 180 % 360 {
            90 | 270 => (orig_h, orig_w),
            _ => (orig_w, orig_h),
        };
        let aspect_180 = display_w as f32 / display_h as f32;
        assert!((aspect_180 - 1920.0/1080.0).abs() < 0.001);
        
        // Test 270° rotation (landscape becomes portrait)
        let (display_w, display_h) = match 270 % 360 {
            90 | 270 => (orig_h, orig_w),
            _ => (orig_w, orig_h),
        };
        let aspect_270 = display_w as f32 / display_h as f32;
        assert!((aspect_270 - 1080.0/1920.0).abs() < 0.001);
        
        // Verify that 90° and 270° rotations have the same aspect ratio
        assert!((aspect_90 - aspect_270).abs() < 0.001);
        
        // Verify that 0° and 180° rotations have the same aspect ratio
        assert!((aspect_0 - aspect_180).abs() < 0.001);
        
        // Verify that 90° rotation is different from 0° rotation
        assert!((aspect_0 - aspect_90).abs() > 0.001);
    }
}