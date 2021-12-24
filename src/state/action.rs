use std::path::PathBuf;
use crate::compute_graph::globals::NameValuePair;

pub enum Action<'a> {
    ProcessUserState(),
    RenderScene(wgpu::Extent3d, &'a wgpu::TextureView),
    WriteToFile(PathBuf),
    OpenFile(PathBuf),
    NewFile(),
    UpdateGlobals(Vec<NameValuePair>),
}
