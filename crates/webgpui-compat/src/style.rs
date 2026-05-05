//! Style mutation stubs — MUST-tier (api-mapping.md §13).

use crate::types::{CompatError, CompatResult, NodeId};

pub fn style_set(_node: NodeId, _key: &str, _value: &str) -> CompatResult<()> {
    Err(CompatError::NotSupported)
}

pub fn style_set_many(_node: NodeId, _styles: &[(&str, &str)]) -> CompatResult<()> {
    Err(CompatError::NotSupported)
}

pub fn style_position(_node: NodeId, _x: f32, _y: f32) -> CompatResult<()> {
    Err(CompatError::NotSupported)
}

pub fn style_size(_node: NodeId, _w: Option<f32>, _h: Option<f32>) -> CompatResult<()> {
    Err(CompatError::NotSupported)
}

pub fn style_margin(_node: NodeId, _l: f32, _t: f32, _r: f32, _b: f32) -> CompatResult<()> {
    Err(CompatError::NotSupported)
}

pub fn style_padding(_node: NodeId, _l: f32, _t: f32, _r: f32, _b: f32) -> CompatResult<()> {
    Err(CompatError::NotSupported)
}

pub fn style_background(_node: NodeId, _color: &str) -> CompatResult<()> {
    Err(CompatError::NotSupported)
}

pub fn style_border(_node: NodeId, _width: f32, _color: &str) -> CompatResult<()> {
    Err(CompatError::NotSupported)
}

pub fn style_opacity(_node: NodeId, _alpha: f32) -> CompatResult<()> {
    Err(CompatError::NotSupported)
}
