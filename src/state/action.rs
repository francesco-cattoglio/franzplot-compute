use std::path::{Path, PathBuf};
use crate::compute_graph::globals::NameValuePair;

// TODO: is it really worth having BOTH a CustomEvent and an Action type?
#[derive(Debug)]
pub enum Action<'a> {
    ProcessUserState(),
    RenderScene(wgpu::Extent3d, &'a wgpu::TextureView),
    RenderUI(&'a winit::window::Window),
    WriteToFile(&'a Path),
    OpenFile(PathBuf),
    OpenPart(PathBuf),
    CameraMovement(egui::Vec2),
    NewFile(),
    UpdateGlobals(Vec<NameValuePair>),
}
