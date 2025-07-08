pub use dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize, Pixel};



// Yes, this enum is taken from the winit
#[derive(Clone, Debug)]
pub enum Size {
    Logical(LogicalSize<f64>),
    Physical(PhysicalSize<u32>)
}

#[derive(Clone, Debug)]
pub enum Pos {
    Logical(LogicalPosition<f64>),
    Physical(PhysicalPosition<i32>),
}

impl Size {
    pub fn to_physical<T: Pixel>(&self, scale_factor: f64) -> PhysicalSize<T> {
        match self {
            Self::Physical(p) => p.cast(),
            Self::Logical(l) => l.to_physical::<T>(scale_factor)
        }
    }
}

impl Pos {
    pub fn to_physical<T: Pixel>(&self, scale_factor: f64) -> PhysicalPosition<T> {
        match self {
            Self::Physical(p) => p.cast(),
            Self::Logical(l) => l.to_physical::<T>(scale_factor)
        }
    }
}
