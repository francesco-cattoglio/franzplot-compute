use std::path::Path;
use crate::compute_graph::globals::NameValuePair;

pub enum Action<'a> {
    ProcessUserState(),
    RenderScene(wgpu::Extent3d, &'a wgpu::TextureView),
    RenderUI(&'a winit::window::Window),
    WriteToFile(&'a Path),
    OpenFile(&'a Path),
    NewFile(),
    UpdateGlobals(Vec<NameValuePair>),
}
