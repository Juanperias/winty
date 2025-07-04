use std::num::NonZero;

use glutin::{api::egl::{context::PossiblyCurrentContext, display::Display, surface::Surface}, config::{ConfigTemplate, ConfigTemplateBuilder, GetGlConfig, GlConfig}, context::{ContextApi, ContextAttributesBuilder, GlProfile, Version}, display::GetGlDisplay, prelude::{GlDisplay, NotCurrentGlContext, PossiblyCurrentGlContext}, surface::{GlSurface, SurfaceAttributes, SurfaceAttributesBuilder, WindowSurface}};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GlHandlerError {
    #[error("Glutin error {0}")]
    GlutinError(#[from] glutin::error::Error)
}

#[derive(Debug)]
pub struct GlHints {
    pub version: (u8, u8),
    pub profile: Profile,
}

#[derive(Debug)]
pub enum Profile {
    Core,
    Compatibility,
}

impl Into<GlProfile> for Profile {
    fn into(self) -> GlProfile {
        match self {
            Self::Core => GlProfile::Core,
            Self::Compatibility => GlProfile::Compatibility,
        }
    }
}

pub struct GlHandler {
    context: PossiblyCurrentContext,
    surface: Surface<WindowSurface>,
    display: Display
}

impl GlHandler {
    pub fn new(display_handle: RawDisplayHandle, window_handle: RawWindowHandle, gl_hints: GlHints, size: (u32, u32)) -> Result<Self, GlHandlerError> {
        let display = unsafe {
            Display::new(display_handle)?
        };

        
    let config = unsafe {
        let c = display.find_configs(config_template())
            .unwrap()
            .reduce(|config, acc| {
                if config.num_samples() > acc.num_samples() {
                    config
                } else {
                    acc
                }
            });
        c.unwrap() 
    };

        let context_attribute = ContextAttributesBuilder::new().with_profile(gl_hints.profile.into()).with_context_api(ContextApi::OpenGl(Some(Version::new(gl_hints.version.0, gl_hints.version.1)))).build(Some(window_handle)); 
        let not_current_context = unsafe {
            display.create_context(&config, &context_attribute)?
        };

        let gl_config = not_current_context.config();
        let surface_attrs: SurfaceAttributes<WindowSurface> = SurfaceAttributesBuilder::<WindowSurface>::default().build(window_handle, NonZero::new(size.0).unwrap(), NonZero::new(size.1).unwrap());
        let surface = unsafe {
            gl_config.display().create_window_surface(&config, &surface_attrs)?
        };
        let context = not_current_context.make_current(&surface)?;

        Ok(Self {
            display,
            context,
            surface
        })
    }
    pub fn swap_buffers(&self) -> Result<(), GlHandlerError> {
        self.surface.swap_buffers(&self.context)?;
        Ok(())
    }
}


fn config_template() -> ConfigTemplate {
        ConfigTemplateBuilder::default()
            .build()
}
