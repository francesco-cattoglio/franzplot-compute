use std::collections::{BTreeMap, HashMap};

use egui::TextureId;
use serde::{Serialize, Deserialize};

use crate::CustomEvent;
use crate::compute_graph::globals::NameValuePair;
use crate::node_graph::NodeID;
use crate::{util, file_io};
use crate::state::{UserState, AppState, user_to_app_state};

#[derive(Deserialize, Serialize)]
#[derive(Clone)]
pub enum GlobalVarUsage {
    Still(f32),
    Animated(f32, f32),
}

#[derive(Clone, Debug, PartialEq)]
enum VarUsageType {
    Still,
    Animated,
}

impl Default for GlobalVarUsage {
    fn default() -> Self {
        Self::Still(0.0)
    }
}

#[derive(Deserialize, Serialize)]
#[derive(Clone, Default)]
pub struct Step {
    comment: String,
    is_on: bool,
    global_vars_usage: HashMap<String, GlobalVarUsage>,
}

#[derive(Deserialize, Serialize)]
#[derive(Clone, Default)]
pub struct FerreData {
    steps: BTreeMap<NodeID, Step>,
}

pub struct FerreGui {
    selected_step: NodeID,
    step_edit: bool,
    new_usage_string: String,
    scene_extent: wgpu::Extent3d,
    winit_proxy: winit::event_loop::EventLoopProxy<CustomEvent>,
    ferre_data: FerreData,
    executor: util::Executor,
}

impl FerreGui {
    pub fn new(winit_proxy: winit::event_loop::EventLoopProxy<CustomEvent>) -> Self {
        FerreGui {
            step_edit: true,
            new_usage_string: String::new(),
            selected_step: Default::default(),
            ferre_data: Default::default(),
            scene_extent: wgpu::Extent3d::default(),
            winit_proxy,
            executor: util::Executor::new(),
        }
    }

    //fn step_edit(&mut self, ui: &mut egui::Ui, step: &mut Step) {

    //}

    pub fn show_steps(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, app_state: &mut AppState, user_state: &mut UserState) {
        // reminder: we (likely) are inside a VerticalScroll Ui
        for (id, step) in self.ferre_data.steps.iter_mut() {
            // for each node id, we can show a UI to handle all the interactions:
            // - we want the animation of the variable to start when we click on the
            //   step label
            // - we do not want the animation to play backward
            if self.step_edit {
                ui.label("Step editing:");
                for pair in step.global_vars_usage.iter_mut() {
                    let usage = pair.1;
                    let mut usage_type = match usage {
                        GlobalVarUsage::Still(_) => VarUsageType::Still,
                        GlobalVarUsage::Animated(_,_) => VarUsageType::Animated,
                    };
                    let formatted = format!("variable '{}' has", &pair.0);
                    ui.horizontal(|ui| {
                        // display name of variable and usage on the same line
                        ui.label(formatted);
                        egui::ComboBox::from_label("usage")
                            .selected_text(format!("{:?}", usage_type))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut usage_type, VarUsageType::Still, "Still");
                                ui.selectable_value(&mut usage_type, VarUsageType::Animated, "Animated");
                            }
                        );
                    });
                    let selection_result = match usage {
                        GlobalVarUsage::Still(mut still_value) => {
                            // we are currently in "still" usage mode
                            match usage_type {
                                // and the user did not change his mind
                                VarUsageType::Still => {
                                    ui.add(egui::Slider::new(&mut still_value, -100.0..=100.0).text("Value"));
                                    GlobalVarUsage::Still(still_value)
                                },
                                // the user changed his mind: the old fixed value is now animated
                                // Do not display any UI, the next frame will fix this anyway
                                VarUsageType::Animated => {
                                    GlobalVarUsage::Animated(still_value, still_value)
                                }
                            }
                        }
                        GlobalVarUsage::Animated(start_value, end_value) => {
                            // we are currently in "animated" usage mode
                            match usage_type {
                                // and the user did not change his mind
                                VarUsageType::Animated => {
                                    ui.horizontal(|ui| {
                                        ui.add(egui::Slider::new(start_value, -100.0..=100.0).text("Start"));
                                        ui.add(egui::Slider::new(end_value, -100.0..=100.0).text("End"));
                                    });
                                    GlobalVarUsage::Animated(*start_value, *end_value)
                                }
                                // the user changed his mind: the old animated value is now fixed
                                // Do not display any UI, the next frame will fix this anyway
                                VarUsageType::Still => {
                                    GlobalVarUsage::Still(*start_value)
                                },
                            }
                        }
                    };
                    *usage = selection_result;
                }
                // Suggest adding a new global variable
                ui.horizontal(|ui| {
                    let _response = ui.add(egui::TextEdit::singleline(&mut self.new_usage_string));
                    if ui.button("Add new global var usage").clicked() {
                        step.global_vars_usage.insert(self.new_usage_string.clone(), GlobalVarUsage::Still(0.0));
                    }
                });
            }
            ui.horizontal(|ui| {
                let maybe_node_title = user_state.node_graph.get_node(*id);
                let title = if let Some(node) = maybe_node_title {
                    node.title.clone()
                } else {
                    "unknown node, check the frzp file".to_string()
                };
                ui.vertical(|ui| {
                    let node_label = format!("Node {}: {}", id, title);
                    ui.label(node_label);
                    ui.small(&step.comment);
                });

                // the button was clicked in this frame. This means either:
                if ui.button("show").clicked() {
                    // the toggle was on, and we need to toggle it off,
                    if step.is_on {
                        step.is_on = false;
                        app_state.update_globals(vec![
                            NameValuePair {
                                name: "a".into(),
                                value: 0.0f32,
                            }
                        ]);
                    } else {
                        // we just need to flip the boolean, render the scene, store
                        // which id is the selected one.
                        // we should also set all the other "is_on" to false, but this
                        // cannot be done inside this loop, because we are using an
                        // iterator!
                        user_to_app_state(app_state, user_state, Some(vec![*id])).expect("issue rendering");
                        self.selected_step = *id;
                        step.is_on = true;
                    }
                }

                // to store the animation, we need a egui::Id object that persists
                // between many frames. The node id is perfect to generate it.
                let animation_id = egui::Id::new(id);
                // OUTSIDE OF CLICKED EVENT: if this is on, we use the time to animate
                // this thing. We only run the animation if this is the step that the
                // user selected, and do something different based on its on/off status
                //if self.selected_step == *id {
                //    // skipping the animation alltogether (animate_bool_with_time with
                //    // 0.0 as time to accomplish the animation)
                //    if *is_on {
                //        let animated_value = ctx.animate_bool_with_time(animation_id, true, 2.5);
                //        app_state.update_globals(vec![
                //            NameValuePair {
                //                name: "a".into(),
                //                value: animated_value,
                //            }
                //        ]);
                //    } else {
                //        ctx.animate_bool_with_time(animation_id, false, 0.0);
                //    }

                //} else { // this is not the selected step: it might have changed since
                //         // last frame! We should reset this animation as well.
                //    *is_on = false;
                //    ctx.animate_bool_with_time(animation_id, false, 0.0);
                //}

            }); // horizontal

        }
    }
}

impl super::Gui for FerreGui {
    fn show(&mut self, ctx: &egui::Context, app_state: &mut AppState, user_state: &mut UserState, texture_id: TextureId) {

        egui::SidePanel::left("procedure panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Open file").clicked() {
                    file_io::async_pick_open(self.winit_proxy.clone(), &self.executor);
                }
                if ui.button("Save file").clicked() {
                    file_io::async_pick_save(self.winit_proxy.clone(), &self.executor);
                }
                if ui.button("Add test entry").clicked() {
                    self.ferre_data.steps.insert(2, Default::default());
                }
            });
            ui.separator();
            ui.checkbox(&mut self.step_edit, "Enable step editing");
            ui.separator();
            egui::ScrollArea::vertical()
                .show(ui, |ui| {
                    self.show_steps(ctx, ui, app_state, user_state);
                }); // Scrollable area.
        }); // left panel
        egui::TopBottomPanel::bottom("variables panel").show(ctx, |ui| {
            let globals = &user_state.globals;
            for variable_name in &globals.names {
                ui.label(variable_name);
            }
        }); // bottom panel
        egui::CentralPanel::default().show(ctx, |ui| {
            // compute avail size
            let avail = ui.available_size();
            // store this size so that we can report it properly to the State on next frame
            self.scene_extent = wgpu::Extent3d {
                width: (avail.x * ctx.pixels_per_point()) as u32,
                height: (avail.y * ctx.pixels_per_point()) as u32,
                depth_or_array_layers: 1,
            };
            ui.image(texture_id, avail);
        }); // central panel

//let texture_size = wgpu::Extent3d {
//    width: 320,
//    height: 320,
//    ..Default::default()
//};
//let render_request = Action::RenderScene(texture_size, &scene_view);
//state.process(render_request).expect("failed to render the scene due to an unknown error");
    }

    /// Ask the UI what size the 3D scene should be. This function gets called after show(), but
    /// before the actual rendering happens.
    fn compute_scene_size(&self) -> Option<wgpu::Extent3d> {
        Some(self.scene_extent)
    }

    /// handle loading of the ferre data
    fn load_ferre_data(&mut self, ferre_data: FerreData) {
        self.ferre_data = ferre_data;
    }

    fn export_ferre_data(&self) -> Option<FerreData> {
        Some(self.ferre_data.clone())
    }
}
