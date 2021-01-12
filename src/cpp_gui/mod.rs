#[derive(Clone, Copy)]
#[repr(i32)]
pub enum PinShape
{
    Circle,
    CircleFilled,
    Triangle,
    TriangleFilled,
    Quad,
    QuadFilled
}
unsafe impl cxx::ExternType for PinShape {
    type Id = cxx::type_id!("imnodes::PinShape");
    type Kind = cxx::kind::Trivial;
}

#[cxx::bridge(namespace = "imnodes")]
pub mod imnodes {
    struct StyleShim {
        pub grid_spacing: f32,
        pub node_padding_horizontal: f32,
        pub node_padding_vertical: f32,

        pub pin_circle_radius: f32,
        pub pin_quad_side_length: f32,
        pub pin_triangle_side_length: f32,
        pub pin_line_thickness: f32,
        pub pin_hover_radius: f32,
    }

    unsafe extern "C++" {
        include!("franzplot-compute/src/cpp_gui/imnodes-5959729/imnodes.h");
        include!("franzplot-compute/src/cpp_gui/imnodes_shims.h");
        type PinShape = super::PinShape;
        fn Initialize();
        fn Shutdown();
        fn BeginNodeEditor();
        fn EndNodeEditor();
        fn IsEditorHovered() -> bool;
        fn BeginNode(id: i32);
        fn EndNode();
        fn ClearNodeSelection();
        fn ClearLinkSelection();
        fn IsAnyAttributeActive(attribute_id: &mut i32) -> bool;
        fn BeginNodeTitleBar();
        fn EndNodeTitleBar();
        fn BeginInputAttribute(id: i32, shape: PinShape);
        fn EndInputAttribute();
        fn BeginStaticAttribute(id: i32);
        fn EndStaticAttribute();
        fn BeginOutputAttribute(id: i32, shape: PinShape);
        fn EndOutputAttribute();
        fn Link(link_id: i32, first_id: i32, second_id: i32);
        fn IsLinkCreated(first_id: &mut i32, second_id: &mut i32) -> bool;
        fn IsNodeHovered(id: &mut i32) -> bool;
        fn IsLinkHovered(id: &mut i32) -> bool;
        fn GetNodePosition(node_id: i32) -> [f32; 2];
        fn SetNodePosition(node_id: i32, position: [f32; 2]);
        fn GetEditorPanning() -> [f32; 2];
        fn SetEditorPanning(position: [f32; 2]);
        fn GetSelectedNodes() -> Vec<i32>;
        fn ApplyStyle(style: &StyleShim);
    }
}

#[cxx::bridge(namespace = "ImGui")]
pub mod ImGui {
    unsafe extern "C++" {
        include!("franzplot-compute/src/cpp_gui/imgui_shims.h");
        fn ClearActiveID();
    }
}
