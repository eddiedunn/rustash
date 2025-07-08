//! GUI functionality for adding snippets using egui.

use anyhow::Result;
use eframe::egui;
use std::sync::mpsc;

/// Data structure to hold the state of the form, to be sent back to the main thread.
#[derive(Debug, Clone, Default)]
pub struct NewSnippetData {
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
}

/// The state for our egui application.
#[derive(Default)]
struct AddSnippetApp {
    title: String,
    content: String,
    tags_str: String,
    error_message: Option<String>,
    // This will hold the snippet data if the user clicks "Save".
    result: Option<NewSnippetData>,
}

/// The application that will be run by eframe. It holds the app state and the sender part of a channel.
struct ChannelApp {
    app: AddSnippetApp,
    tx: mpsc::Sender<Option<NewSnippetData>>,
}

impl eframe::App for ChannelApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Add New Snippet");
            ui.add_space(10.0);

            // Form fields
            ui.horizontal(|ui| {
                ui.label("Title:");
                ui.add(egui::TextEdit::singleline(&mut self.app.title).hint_text("Enter snippet title"));
            });
            ui.horizontal(|ui| {
                ui.label("Tags:");
                ui.add(egui::TextEdit::singleline(&mut self.app.tags_str).hint_text("e.g., rust, cli, example"));
            });

            ui.add_space(5.0);
            ui.label("Content:");
            ui.add(
                egui::TextEdit::multiline(&mut self.app.content)
                    .font(egui::TextStyle::Monospace)
                    .desired_rows(10)
                    .desired_width(f32::INFINITY),
            );

            ui.add_space(10.0);

            // Action buttons
            ui.horizontal(|ui| {
                if ui.button("Save Snippet").clicked() {
                    if self.app.title.trim().is_empty() {
                        self.app.error_message = Some("Title cannot be empty.".to_string());
                    } else if self.app.content.trim().is_empty() {
                        self.app.error_message = Some("Content cannot be empty.".to_string());
                    } else {
                        // Success, prepare the result and close the window
                        self.app.result = Some(NewSnippetData {
                            title: self.app.title.clone(),
                            content: self.app.content.clone(),
                            tags: self.app.tags_str
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect(),
                        });
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }

                if ui.button("Cancel").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            // Display error message if any
            if let Some(err) = &self.app.error_message {
                ui.add_space(5.0);
                ui.colored_label(egui::Color32::RED, err);
            }
        });
    }
    
    // This method is called when the window is about to close.
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Send the result back to the main thread, whether it's Some or None.
        self.tx.send(self.app.result.clone()).ok();
    }
}

/// Public function to launch the GUI window and wait for the result.
pub fn show_add_window() -> Result<Option<NewSnippetData>> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([500.0, 350.0])
            .with_resizable(true),
        ..Default::default()
    };
    
    let (tx, rx) = mpsc::channel();

    eframe::run_native(
        "Add New Rustash Snippet",
        options,
        Box::new(move |_cc| {
            // This closure is called once to create the app.
            Box::new(ChannelApp {
                app: AddSnippetApp::default(),
                tx,
            })
        }),
    ).map_err(|e| anyhow::anyhow!("Failed to run GUI: {}", e))?;

    // Block and wait for the GUI thread to send the result.
    Ok(rx.recv()?)
}
