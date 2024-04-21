use crate::{
    image_editing::EditState,
    scrubber::Scrubber,
    settings::PersistentSettings,
    utils::{ExtendedImageInfo, Frame, Player}, ImageExt,
};
use egui_notify::Toasts;
use image::RgbaImage;
use image::imageops;
use nalgebra::Vector2;
use notan::{egui::epaint::ahash::HashMap, prelude::Texture, AppState};
use notan::prelude::{Graphics, TextureFilter};
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
};

#[derive(Debug, Clone)]
pub struct ImageGeometry {
    /// The scale of the displayed image
    pub scale: f32,
    /// Image offset on canvas
    pub offset: Vector2<f32>,
    pub dimensions: (u32, u32),
}

#[derive(Debug, Clone)]
pub enum Message {
    Info(String),
    Warning(String),
    Error(String),
    LoadError(String),
    Saved(PathBuf),
}

impl Message {
    pub fn info(m: &str) -> Self {
        Self::Info(m.into())
    }
    pub fn warn(m: &str) -> Self {
        Self::Warning(m.into())
    }
    pub fn err(m: &str) -> Self {
        Self::Error(m.into())
    }
}

pub struct TexWrap{
    //pub tex:Texture,
    pub texor:Vec<Texture>,
    //pub asbet:Option<Texture>,
    pub col_count:u32,
    pub row_count:u32,
    pub col_translation:u32,
    pub row_translation:u32,
    pub size_vec:(f32,f32) // Will be the whole Texture Array size soon
}

impl TexWrap{
    /*pub fn new(texture: Texture) -> Self{
        let s = texture.size();
        TexWrap { tex: texture, size_vec:s }        
    }*/

    pub fn from_rgbaimage(gfx: &mut Graphics, linear_mag_filter: bool, image: &RgbaImage) -> Option<TexWrap>{
        let im_w = image.width();
        let im_h = image.height();
        let s = (im_w as f32, im_h as f32);
        let max_texture_size = gfx.limits().max_texture_size;
        let col_count = (im_w as f32/max_texture_size as f32).ceil() as u32;       
        let row_count = (im_h as f32/max_texture_size as f32).ceil() as u32;

        

        let mut a:Vec<Texture> = Vec::new();
        let row_increment = std::cmp::min(max_texture_size, im_h);
        let col_increment = std::cmp::min(max_texture_size, im_w);
        let mut fine = true;
        
        for row_index in  0..row_count {
            let tex_start_y = row_index*row_increment;
            let tex_height = std::cmp::min(
                row_increment,
                im_h-tex_start_y
            );
            for col_index in  0..col_count {
                let tex_start_x = col_index*col_increment;
                let tex_width = std::cmp::min(
                    col_increment,
                    im_w-tex_start_x
                );
                print!("{0},{1},{2},{3}",tex_start_y, tex_height, tex_start_x, tex_width);
                let sub_img = imageops::crop_imm(image, tex_start_x, tex_start_y, tex_width, tex_height);
                let my_img = sub_img.to_image();
                let tex = gfx.create_texture()
                    .from_bytes(my_img.as_ref(), my_img.width(), my_img.height())
                    .with_mipmaps(true)
                    // .with_format(notan::prelude::TextureFormat::SRgba8)
                    // .with_premultiplied_alpha()
                    .with_filter(
                        TextureFilter::Linear,
                        if linear_mag_filter {
                            TextureFilter::Linear
                        } else {
                            TextureFilter::Nearest
                        },
                    )
                    // .with_wrap(TextureWrap::Clamp, TextureWrap::Clamp)
                    .build()
                    .ok();

                    
                    if let Some(t) = tex {
                        a.push(t);                        
                    }
                    else{
                        fine = false;
                        break;
                    }                  
            }
            if(fine == false){
                break;
            }
        }
        
        /*let sub_img = imageops::crop_imm(image, 0, 0, max_texture_size, image.height());
        let my_img = sub_img.to_image();
        let tex = gfx.create_texture()
            .from_bytes(my_img.as_ref(), my_img.width(), my_img.height())
            .with_mipmaps(true)
            // .with_format(notan::prelude::TextureFormat::SRgba8)
            // .with_premultiplied_alpha()
            .with_filter(
                TextureFilter::Linear,
                if linear_mag_filter {
                    TextureFilter::Linear
                } else {
                    TextureFilter::Nearest
                },
            )
            // .with_wrap(TextureWrap::Clamp, TextureWrap::Clamp)
            .build()
            .ok();
        
        match tex {
            None => None,
            Some(t) => Some(TexWrap { tex: t, size_vec:s, col_count:col_count, row_count:row_count }),
        }*/
        
        if(fine){
        Some(TexWrap {size_vec:s, col_count:col_count, row_count:row_count,texor:a, col_translation:col_increment, row_translation:row_increment })
    }
    else {
        None
    }

    }

    pub fn size(&self)->(f32,f32){
        return self.size_vec;
    }

    pub fn width(&self)-> f32 {
        return self.size_vec.0;
    }

    pub fn height(&self)-> f32 {
        return self.size_vec.1;
    }
}

/// The state of the application
#[derive(AppState)]
pub struct OculanteState {
    pub image_geometry: ImageGeometry,
    pub compare_list: HashMap<PathBuf, ImageGeometry>,
    pub drag_enabled: bool,
    pub reset_image: bool,
    /// Is the image fully loaded?
    pub is_loaded: bool,
    pub window_size: Vector2<f32>,
    pub cursor: Vector2<f32>,
    pub cursor_relative: Vector2<f32>,
    pub sampled_color: [f32; 4],
    pub mouse_delta: Vector2<f32>,
    pub texture_channel: (Sender<Frame>, Receiver<Frame>),
    pub message_channel: (Sender<Message>, Receiver<Message>),
    /// Channel to load images from
    pub load_channel: (Sender<PathBuf>, Receiver<PathBuf>),
    pub extended_info_channel: (Sender<ExtendedImageInfo>, Receiver<ExtendedImageInfo>),
    pub extended_info_loading: bool,
    /// The Player, responsible for loading and sending Frames
    pub player: Player,
    pub current_texture: Option<TexWrap>,
    pub current_path: Option<PathBuf>,
    pub current_image: Option<RgbaImage>,
    pub settings_enabled: bool,
    pub image_info: Option<ExtendedImageInfo>,
    pub tiling: usize,
    pub mouse_grab: bool,
    pub key_grab: bool,
    pub edit_state: EditState,
    pub pointer_over_ui: bool,
    /// Things that perisist between launches
    pub persistent_settings: PersistentSettings,
    pub always_on_top: bool,
    pub network_mode: bool,
    /// how long the toast message appears
    /// data to transform image once fullscreen is entered/left
    pub fullscreen_offset: Option<(i32, i32)>,
    /// List of images to cycle through. Usually the current dir or dropped files
    pub scrubber: Scrubber,
    pub checker_texture: Option<Texture>,
    pub redraw: bool,
    pub first_start: bool,
    pub toasts: Toasts,
    pub filebrowser_id: Option<String>,
}

impl OculanteState {
    pub fn send_message_info(&self, msg: &str) {
        _ = self.message_channel.0.send(Message::info(msg));
    }

    pub fn send_message_err(&self, msg: &str) {
        _ = self.message_channel.0.send(Message::err(msg));
    }

    pub fn send_message_warn(&self, msg: &str) {
        _ = self.message_channel.0.send(Message::warn(msg));
    }
}

impl Default for OculanteState {
    fn default() -> OculanteState {
        let tx_channel = mpsc::channel();
        OculanteState {
            image_geometry: ImageGeometry {
                scale: 1.0,
                offset: Default::default(),
                dimensions: Default::default(),
            },
            compare_list: Default::default(),
            drag_enabled: Default::default(),
            reset_image: Default::default(),
            is_loaded: Default::default(),
            cursor: Default::default(),
            cursor_relative: Default::default(),
            sampled_color: [0., 0., 0., 0.],
            player: Player::new(tx_channel.0.clone(), 20, 16384),
            texture_channel: tx_channel,
            message_channel: mpsc::channel(),
            load_channel: mpsc::channel(),
            extended_info_channel: mpsc::channel(),
            extended_info_loading: Default::default(),
            mouse_delta: Default::default(),
            current_texture: Default::default(),
            current_image: Default::default(),
            current_path: Default::default(),
            settings_enabled: Default::default(),
            image_info: Default::default(),
            tiling: 1,
            mouse_grab: Default::default(),
            key_grab: Default::default(),
            edit_state: Default::default(),
            pointer_over_ui: Default::default(),
            persistent_settings: Default::default(),
            always_on_top: Default::default(),
            network_mode: Default::default(),
            window_size: Default::default(),
            fullscreen_offset: Default::default(),
            scrubber: Default::default(),
            checker_texture: Default::default(),
            redraw: Default::default(),
            first_start: true,
            toasts: Toasts::default().with_anchor(egui_notify::Anchor::BottomLeft),
            filebrowser_id: None,
        }
    }
}
