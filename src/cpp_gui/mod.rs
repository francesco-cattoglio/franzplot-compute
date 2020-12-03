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
    unsafe extern "C++" {
        include!("franzplot-compute/src/cpp_gui/imnodes-5959729/imnodes.h");
        include!("franzplot-compute/src/cpp_gui/imnodes_shims.h");
        type PinShape = super::PinShape;
        fn Initialize();
        fn Shutdown();
        fn BeginNodeEditor();
        fn EndNodeEditor();
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
        fn GetSelectedNodes() -> Vec<i32>;
    }
}

