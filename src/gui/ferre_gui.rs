use std::collections::{BTreeMap, HashMap};
use egui::epaint::{Color32, text::{LayoutJob, TextFormat}, FontFamily, FontId};

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
    renderables_shown: Vec<NodeID>,
    global_vars_usage: HashMap<String, GlobalVarUsage>,
}

#[derive(Deserialize, Serialize)]
#[derive(Clone, Default)]
pub struct FerreData {
    steps: Vec<Step>,
}

pub struct FerreGui {
    animating_step: Option<usize>,
    step_edit: bool,
    new_usage_string: String,
    new_node_id: NodeID,
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
            new_node_id: 0,
            animating_step: None,
            ferre_data: Default::default(),
            scene_extent: wgpu::Extent3d::default(),
            winit_proxy,
            executor: util::Executor::new(),
        }
    }

    fn show_test_formula(&self, ui: &mut egui::Ui, height: f32) {
        let mut job = LayoutJob::default();
        job.append(
            "f",
            0.0,
            TextFormat {
                font_id: FontId::new(14.0, FontFamily::Proportional),
                color: Color32::WHITE,
                ..Default::default()
            },
        );
        job.append(
            "z",
            0.0,
            TextFormat {
                font_id: FontId::new(10.0, FontFamily::Proportional),
                color: Color32::WHITE,
                valign: egui::Align::BOTTOM,
                ..Default::default()
            },
        );
        job.append(
            " = ",
            0.0,
            TextFormat {
                font_id: FontId::new(14.0, FontFamily::Proportional),
                color: Color32::WHITE,
                ..Default::default()
            },
        );
        job.append(
            &format!("{height}"),
            0.0,
            TextFormat {
                font_id: FontId::new(14.0, FontFamily::Proportional),
                color: Color32::RED,
                ..Default::default()
            },
        );
        let galley = ui.fonts().layout_job(job);
        ui.label(galley);
    }

    pub fn show_steps(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, app_state: &mut AppState, user_state: &mut UserState) {
        // reminder: we (likely) are inside a VerticalScroll Ui
        for (idx, step) in self.ferre_data.steps.iter_mut().enumerate() {
            if self.step_edit {
                ui.separator();
                ui.horizontal(|ui|{
                    ui.label(format!("Step {idx} editing:"));
                    if ui.button("clear global usage").clicked() {
                        step.global_vars_usage.clear();
                    }
                });
                for (name, usage) in step.global_vars_usage.iter_mut() {
                    let mut usage_type = match usage {
                        GlobalVarUsage::Still(_) => VarUsageType::Still,
                        GlobalVarUsage::Animated(_,_) => VarUsageType::Animated,
                    };
                    let formatted = format!("variable '{}' has", name);
                    ui.horizontal(|ui| {
                        // display name of variable and usage on the same line
                        ui.label(formatted);
                        egui::ComboBox::new(format!("{idx}+{name}"), "usage")
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
                                    ui.add(egui::Slider::new(&mut still_value, -5.0..=5.0).text("Value"));
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
                                        ui.add(egui::Slider::new(start_value, -5.0..=5.0).text("Start"));
                                        ui.add(egui::Slider::new(end_value, -5.0..=5.0).text("End"));
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
                    let _response = ui.add_sized(egui::vec2(36.0, 18.0), egui::TextEdit::singleline(&mut self.new_usage_string));
                    if ui.button("Add new global var usage").clicked() {
                        step.global_vars_usage.insert(self.new_usage_string.clone(), GlobalVarUsage::Still(0.0));
                        self.new_usage_string.clear();
                    }
                });
                ui.horizontal(|ui| {
                    // iterate through all node_ids: do not retain the ones the user clicks
                    step.renderables_shown.retain(|node_id| {
                        !ui.button(node_id.to_string()).clicked()
                    })
                });
                // Suggest adding a new NodeID to show
                ui.horizontal(|ui| {
                    let _response = ui.add(egui::Slider::new(&mut self.new_node_id, 0..=20));
                    if ui.button("Add node to list of shown").clicked() {
                        step.renderables_shown.push(self.new_node_id);
                    }
                });
            } // Step edit mode ends here

            // for each node id, we can show a UI to handle all the interactions:
            // - we want the animation of the variable to start when we click on the
            //   "run" botton
            // - we do not want the animation to play backward
            ui.horizontal(|ui| {
                if let Some(node_id) = step.renderables_shown.first() {
                    let maybe_node_title = user_state.node_graph.get_node(*node_id);
                    let title = if let Some(node) = maybe_node_title {
                        node.title.clone()
                    } else {
                        "unknown node, check the frzp file".to_string()
                    };
                    ui.vertical(|ui| {
                        let node_label = format!("Node {}: {}", *node_id, title);
                        ui.label(node_label);
                        ui.small(&step.comment);
                    });

                } else {
                    ui.label("Nothing to show for this step yet");

                }
                // Unfortunately we are conflating two different concepts in here: showing objects
                // and animating them. We need to change both the UI and the code
                // to separate efficiently the two concepts.
                if ui.button("show").clicked() {
                    // The button was clicked. No matter what, but we need to reset the
                    // animation of variables
                    ctx.clear_animations();
                    // iterate over global vars usage to reset everything.
                    // BEWARE: this will also reset the initial value for the animations!
                    let name_value_pairs: Vec<NameValuePair> = step.global_vars_usage
                        .iter()
                        .map(|(name, usage)| {
                            match usage {
                                GlobalVarUsage::Still(value) => NameValuePair { name: name.clone(), value: *value },
                                GlobalVarUsage::Animated(start_val, _) => {
                                    let animation_id = egui::Id::new(name);
                                    // first call after clear animations will set the start val
                                    ctx.animate_value_with_time(animation_id, *start_val, 0.0);
                                    NameValuePair { name: name.clone(), value: *start_val }
                                }
                            }
                        })
                        .collect();
                    app_state.update_globals(name_value_pairs);

                    // If the button was clicked in this frame. This means either:
                    // - the toggle was on, and we need to toggle it off,
                    // - the toggle was off OR unselected, and we need to toggle it on,
                    self.animating_step = match self.animating_step {
                        // we clicked the same thing that was already being animated. Set animation
                        // to None, so that animation stops
                        Some(prev_idx) if prev_idx == idx => {
                            None
                        }
                        // we clicked a NEW step (either because nothing was being animated, or
                        // because we selected one that is different from the previous
                        Some(_) | None => {
                            app_state.renderer.set_renderable_filter(Some(step.renderables_shown.clone()));
                            Some(idx)
                        }
                    };
                }

            }); // horizontal
            ui.separator();
        }
        // OUTSIDE OF THE LOOPING OVER STEPS: if there is a request for animation, animate!
        if let Some(idx) = self.animating_step {
            let step = &self.ferre_data.steps[idx];
            let name_value_pairs: Vec<NameValuePair> = step.global_vars_usage
                .iter()
                .map(|(name, usage)| {
                    match usage {
                        GlobalVarUsage::Still(value) => {
                            NameValuePair { name: name.clone(), value: *value } // Doing nothing is
                                                                                // probably also fine
                        }
                        GlobalVarUsage::Animated(_start_val, end_val) => {
                            let animation_id = egui::Id::new(name);
                            let animated_value = ctx.animate_value_with_time(animation_id, *end_val, 2.5);
                            NameValuePair { name: name.clone(), value: animated_value }
                        }
                    }
                })
                .collect();
            app_state.update_globals(name_value_pairs);
        }

        // At the end of the steps, add the button to add new steps
        if self.step_edit {
            ui.horizontal(|ui| {
                if ui.button("New step").clicked() {
                    self.ferre_data.steps.push(Step::default());
                }
                if ui.button("Clone last step").clicked() {
                    self.ferre_data.steps.push(self.ferre_data.steps.last().cloned().unwrap_or_default());
                }
            });
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
            let mut name_value_pairs = app_state.read_globals();

            let mut formula_height: f32 = 0.0;
            for pair in name_value_pairs.iter_mut() {
                ui.horizontal(|ui| {
                    ui.label(&pair.name);
                    ui.add(egui::Slider::new(&mut pair.value, -5.0..=5.0));
                });
                if pair.name.contains('b') {
                    formula_height = pair.value;
                }
            }
            app_state.update_globals(name_value_pairs);
            self.show_test_formula(ui, formula_height);
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
