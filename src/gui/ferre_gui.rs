use std::collections::HashMap;
use egui::epaint::{Color32, text::{LayoutJob, TextFormat}, FontFamily, FontId};

use egui::TextureId;
use serde::{Serialize, Deserialize};
use winit::event::ElementState;

use crate::{CustomEvent, state::Action};
use crate::compute_graph::globals::NameValuePair;
use crate::node_graph::NodeID;
use crate::{util, file_io};
use crate::state::{UserState, AppState};

#[derive(Deserialize, Serialize)]
#[derive(Clone)]
pub enum GlobalVarUsage {
    User(f32),
    Still(f32),
    Animated(f32, f32),
}

#[derive(Clone, Debug, PartialEq)]
enum VarUsageType {
    User,
    Still,
    Animated,
}

const ANIM_TIME: f32 = 2.5;

impl Default for GlobalVarUsage {
    fn default() -> Self {
        Self::Still(0.0)
    }
}

#[derive(Deserialize, Serialize)]
#[derive(Clone, Default)]
pub struct Step {
    comment: String,
    image_name: String,
    renderables_shown: Vec<NodeID>,
    global_vars_usage: HashMap<String, (GlobalVarUsage, [f32; 2])>,
}

#[derive(Deserialize, Serialize)]
#[derive(Clone, Default)]
pub struct FerreData {
    steps: Vec<Step>,
}


#[derive(Clone, Default, PartialEq)]
pub struct AnimationStatus {
    step_idx: usize,
    running: bool,
    percent_remaining: f32,
    percent_paused: f32,
}

pub struct FerreGui {
    animating_step: Option<AnimationStatus>,
    step_edit: bool,
    new_usage_string: String,
    new_node_id: NodeID,
    loaded_images: HashMap<String, egui::TextureHandle>,
    scene_extent: wgpu::Extent3d,
    winit_proxy: winit::event_loop::EventLoopProxy<CustomEvent>,
    ferre_data: FerreData,
    open_part: String,
    executor: util::Executor,
}

impl FerreGui {
    pub fn new(winit_proxy: winit::event_loop::EventLoopProxy<CustomEvent>) -> Self {
        FerreGui {
            step_edit: true,
            new_usage_string: String::new(),
            new_node_id: 0,
            animating_step: None,
            loaded_images: HashMap::new(),
            ferre_data: Default::default(),
            scene_extent: wgpu::Extent3d::default(),
            winit_proxy,
            executor: util::Executor::new(),
            open_part: String::new(),
        }
    }

    fn show_test_formula(&self, ui: &mut egui::Ui, var: f32) {
        let mut job = LayoutJob::default();
        job.append(
            "length",
            0.0,
            TextFormat {
                font_id: FontId::new(14.0, FontFamily::Proportional),
                color: Color32::WHITE,
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
            &format!("{var}"),
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
                ui.horizontal(|ui|{
                    ui.label("Image path:");
                    let response = ui.text_edit_singleline(&mut step.image_name);
                    if response.lost_focus() && ui.input().key_pressed(egui::Key::Enter) {
                        let path = std::path::Path::new(step.image_name.as_str());
                        println!("Attempting to create a texture from {:?}", path);
                        if let Some(texture_hnd) = util::load_texture_to_egui(ctx, path) {
                            println!("Attempt succesful");
                            self.loaded_images.insert(step.image_name.clone(), texture_hnd);
                        };
                    }
                });
                ui.separator();
                for (name, (usage, range)) in step.global_vars_usage.iter_mut() {
                    let mut usage_type = match usage {
                        GlobalVarUsage::Still(_) => VarUsageType::Still,
                        GlobalVarUsage::Animated(_,_) => VarUsageType::Animated,
                        GlobalVarUsage::User(_) => VarUsageType::User,
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
                                ui.selectable_value(&mut usage_type, VarUsageType::User, "User controlled");
                            }
                        );
                    });
                    // match the OLD usage type, and so something different depending on latest selection
                    let selection_result = match usage {
                        // We previously had a "User controlled" usage
                        GlobalVarUsage::User(mut user_value) => {
                            match usage_type {
                                // and the user wants to swap to Still: reset it as such
                                VarUsageType::Still => {
                                    GlobalVarUsage::Still(user_value)
                                },
                                // the user wants to swap to Animated
                                VarUsageType::Animated => {
                                    GlobalVarUsage::Animated(user_value, user_value)
                                }
                                // the selection did not change
                                VarUsageType::User => {
                                    ui.horizontal(|ui| {
                                        ui.label("Value:");
                                        ui.add(egui::DragValue::new(&mut user_value)
                                               .speed(0.01)
                                               .min_decimals(2)
                                               .max_decimals(6));
                                        GlobalVarUsage::User(user_value)
                                    }).inner
                                },
                            }
                        }
                        GlobalVarUsage::Still(mut still_value) => {
                            // we are currently in "still" usage mode
                            match usage_type {
                                // and the user did not change his mind
                                VarUsageType::Still => {
                                    ui.horizontal(|ui| {
                                        ui.label("Value:");
                                        ui.add(egui::DragValue::new(&mut still_value)
                                               .speed(0.01)
                                               .min_decimals(2)
                                               .max_decimals(6));
                                        GlobalVarUsage::Still(still_value)
                                    }).inner
                                },
                                // the user changed his mind: the old fixed value is now animated
                                // Do not display any UI, the next frame will fix this anyway
                                VarUsageType::Animated => {
                                    GlobalVarUsage::Animated(still_value, still_value)
                                }
                                // the user selected user controlled
                                VarUsageType::User => {
                                    GlobalVarUsage::User(still_value)
                                }
                            }
                        }
                        GlobalVarUsage::Animated(mut start_value, mut end_value) => {
                            // we are currently in "animated" usage mode
                            match usage_type {
                                // the user changed his mind: the old animated value is now fixed
                                // Do not display any UI, the next frame will fix this anyway
                                VarUsageType::Still => {
                                    GlobalVarUsage::Still(start_value)
                                },
                                // the selection stays the same: show the ui
                                VarUsageType::Animated => {
                                    ui.horizontal(|ui| {
                                        ui.label("Value at start:");
                                        ui.add(egui::DragValue::new(&mut start_value)
                                               .speed(0.01)
                                               .min_decimals(2)
                                               .max_decimals(6));
                                        ui.label("end:");
                                        ui.add(egui::DragValue::new(&mut end_value)
                                               .speed(0.01)
                                               .min_decimals(2)
                                               .max_decimals(6));
                                    });
                                    GlobalVarUsage::Animated(start_value, end_value)
                                }
                                // the user selected user controlled
                                VarUsageType::User => {
                                    GlobalVarUsage::User(start_value)
                                }
                            }
                        }
                    };
                    *usage = selection_result;
                    ui.horizontal(|ui|{
                        ui.label("Range for user interaction:");
                        ui.add(egui::DragValue::new(&mut range[0]).speed(0.01).min_decimals(2));
                        ui.add(egui::DragValue::new(&mut range[1]).speed(0.01).min_decimals(2));
                    });
                }
                // Suggest adding a new global variable
                ui.horizontal(|ui| {
                    let _response = ui.add_sized(egui::vec2(36.0, 18.0), egui::TextEdit::singleline(&mut self.new_usage_string));
                    if ui.button("Add new global var usage").clicked() && !self.new_usage_string.is_empty() {
                        step.global_vars_usage.insert(self.new_usage_string.clone(), (GlobalVarUsage::Still(0.0), [0.0, 0.0]));
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
                    let _response = ui.add(egui::Slider::new(&mut self.new_node_id, 0..=50));
                    if ui.button("Add node to list of shown").clicked() {
                        step.renderables_shown.push(self.new_node_id);
                    }
                });
            } // Step edit mode ends here

            // for each node id, we can show a UI to handle all the interactions:
            // - we want the animation of the variable to start when we click on the
            //   "run" botton
            // - we do not want the animation to play backward
            let title = if let Some(node_id) = step.renderables_shown.first() {
                let maybe_node_title = user_state.node_graph.get_node(*node_id);
                if let Some(node) = maybe_node_title {
                        node.title.clone()
                    } else {
                        "unknown node, check the frzp file".to_string()
                    }
            } else {
                "No renderable in list".to_string()
            };

            ui.collapsing(title.clone(), |ui| {
                ui.small(&step.comment);
                if let Some(texture) = self.loaded_images.get(&step.image_name) {
                    ui.image(texture.id(), texture.size_vec2() * 0.5);
                }

                ui.horizontal(|ui| {
                    if ui.button("Show").clicked() {
                        // The button was clicked. No matter what, but we need to reset the
                        // animation of variables
                        ctx.clear_animations();
                        // iterate over global vars usage to reset everything.
                        // BEWARE: this will also reset the initial value for the animations!
                        let name_value_pairs: Vec<NameValuePair> = step.global_vars_usage
                            .iter()
                            // create a name_value pair only for Still and Animated globals, the User
                            // ones get filtered away because we do not want to change them
                            .map(|(name, (usage, _range))| {
                                match usage {
                                    GlobalVarUsage::User(value) | GlobalVarUsage::Still(value) => NameValuePair { name: name.clone(), value: *value },
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

                        ctx.animate_value_with_time(egui::Id::new("animation_percent"), 1.0, 0.0);
                        self.animating_step = Some(AnimationStatus{ step_idx: idx, running: false, percent_remaining: 1.0, percent_paused: 1.0 });
                    }

                    if let Some(AnimationStatus { step_idx, running, percent_remaining, percent_paused }) = &mut self.animating_step {
                        if *step_idx == idx {
                            let play_pause = if *running {
                                // animation is running!
                                if *percent_remaining > f32::EPSILON {
                                    let remaining_time = *percent_paused * ANIM_TIME;
                                    *percent_remaining = ctx.animate_value_with_time(egui::Id::new("animation_percent"), 0.0, remaining_time);
                                } else {
                                    *percent_remaining = ctx.animate_value_with_time(egui::Id::new("animation_percent"), 1.0, 0.0);
                                    *running = false;
                                }
                                "⏸ pause"
                            } else {
                                "▶ play"
                            };

                            if ui.button(play_pause).clicked() {
                                // If the button was clicked in this frame. Toggle the animation status;
                                if *running {
                                    // we need to stop the animation! Store the value
                                    *percent_paused = *percent_remaining;
                                    ctx.animate_value_with_time(egui::Id::new("animation_percent"), *percent_remaining, 0.0);
                                    *running = false;
                                } else {
                                    // we need to restore the animation!
                                    if *percent_remaining < f32::EPSILON {
                                        *percent_remaining = ctx.animate_value_with_time(egui::Id::new("animation_percent"), 1.0, 0.0);
                                    }
                                    *percent_paused = *percent_remaining;
                                    *running = true;
                                }
                            }
                        }
                    }
                })

            }); // horizontal
            ui.separator();
        }
        // OUTSIDE OF THE LOOPING OVER STEPS: if there is a request for animation, animate!
        if let Some(animation) = &self.animating_step {
            let remaining_percent = animation.percent_remaining;
            let step = &self.ferre_data.steps[animation.step_idx];
            // IF YOU WANT TO LET THE USER MODIFY THE VALUES DURING A PAUSE, ENABLE THIS IF BLOCK
            //if animation.running {
                let name_value_pairs: Vec<NameValuePair> = step.global_vars_usage
                    .iter()
                    .filter_map(|(name, (usage, _range))| {
                        match usage {
                            GlobalVarUsage::User(_) => { None }
                            GlobalVarUsage::Still(value) => {
                                Some(NameValuePair { name: name.clone(), value: *value }) // Doing nothing is
                                                                                    // probably also fine
                            }
                            GlobalVarUsage::Animated(start_val, end_val) => {
                                let animated_value = *end_val * (1.0 - remaining_percent) + *start_val * remaining_percent;
                                Some(NameValuePair { name: name.clone(), value: animated_value })
                            }
                        }
                    })
                    .collect();
                app_state.update_globals(name_value_pairs);
            //}
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
    fn show(&mut self, ctx: &egui::Context, app_state: &mut AppState, user_state: &mut UserState, texture_id: TextureId) -> Option<Action> {
        egui::SidePanel::left("procedure panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Open file").clicked() {
                    file_io::async_pick_open(self.winit_proxy.clone(), &self.executor);
                }
                if ui.button("Save file").clicked() {
                    file_io::async_pick_save(self.winit_proxy.clone(), &self.executor);
                }
            });
            ui.separator();
            if let Some(parts_list) = &app_state.parts_list {
                // this will trigger only once
                if self.open_part.is_empty() {
                    self.open_part = parts_list.first().unwrap().0.clone();
                }
                egui::ComboBox::from_id_source("Selected part")
                    .selected_text(&self.open_part)
                    .show_ui(ui, |ui| {
                        for part in parts_list {
                            if ui.selectable_value(&mut self.open_part, part.0.clone(), &part.0).clicked() {
                                self.winit_proxy.send_event(CustomEvent::OpenPart(part.1.to_path_buf()));
                            }
                        }
                    });
                ui.separator();
            }
            ui.checkbox(&mut self.step_edit, "Enable step editing");
            ui.separator();
            egui::ScrollArea::vertical()
                .show(ui, |ui| {
                    self.show_steps(ctx, ui, app_state, user_state);
                }); // Scrollable area.
        }); // left panel
        egui::TopBottomPanel::bottom("variables panel").show(ctx, |ui| {
            let mut name_value_pairs = app_state.read_globals();

            for pair in name_value_pairs.iter_mut() {
                // check if this variable exists in the usage table,
                // and see if the user is authorized to read that.
                if let Some(animation) = &self.animating_step {
                    let curr_step = &self.ferre_data.steps[animation.step_idx];
                    if let Some((_usage, range)) = curr_step.global_vars_usage.get(&pair.name) {
                        if range[0] != range[1] {
                            ui.horizontal(|ui| {
                                ui.label(&pair.name);
                                ui.add(egui::Slider::new(&mut pair.value, range[0].min(range[1]) ..= range[0].max(range[1])));
                            });
                        }
                    }
                }
            }
            app_state.update_globals(name_value_pairs);
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
            let response = ui.image(texture_id, avail).interact(egui::Sense::click_and_drag());
            if response.dragged_by(egui::PointerButton::Primary) {
                let delta = response.drag_delta();
                Some(Action::CameraMovement(delta))
            } else {
                None
            }
        }).inner // central panel inner response
    }

    /// Ask the UI what size the 3D scene should be. This function gets called after show(), but
    /// before the actual rendering happens.
    fn compute_scene_size(&self) -> Option<wgpu::Extent3d> {
        Some(self.scene_extent)
    }

    /// handle loading of the ferre data
    fn load_ferre_data(&mut self, ctx: &egui::Context, ferre_data: FerreData) {
        self.ferre_data = ferre_data;
        for step in self.ferre_data.steps.iter() {
            if !step.image_name.is_empty() {
                let path = std::path::Path::new(step.image_name.as_str());
                println!("Attempting to create a texture from {:?}", path);
                if let Some(texture_hnd) = util::load_texture_to_egui(ctx, path) {
                    println!("Attempt succesful");
                    self.loaded_images.insert(step.image_name.clone(), texture_hnd);
                };
            }
        }
    }

    fn export_ferre_data(&self) -> Option<FerreData> {
        Some(self.ferre_data.clone())
    }

    fn mark_new_part_open(&mut self, ctx: &egui::Context) {
        self.ferre_data = FerreData::default();
        ctx.memory().reset_areas();
        ctx.memory().data.clear();
    }

    fn mark_new_file_open(&mut self, ctx: &egui::Context) {
        self.ferre_data = FerreData::default();
        ctx.memory().reset_areas();
        ctx.memory().data.clear();
        self.open_part.clear();
    }
}
