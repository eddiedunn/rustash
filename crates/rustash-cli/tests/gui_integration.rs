//! Integration tests for the GUI functionality using egui_test

use anyhow::Result;
use egui_test::{test_app, TestApp, TestBackendOptions};
use rustash_cli::gui::NewSnippetData;
use serial_test::serial;
use std::sync::mpsc;

// Test that the GUI window can be created and renders the basic UI elements
#[test]
#[serial]
fn test_gui_window_renders() -> Result<()> {
    // Create a test app with our GUI
    let mut test_app = TestApp::new(TestBackendOptions::default(), |_cc| {
        let (tx, _rx) = mpsc::channel();
        Box::new(rustash_cli::gui::ChannelApp {
            app: Default::default(),
            tx,
        })
    });

    // Run the app for a few frames to let it initialize
    test_app.update();
    test_app.update();

    // Check that the window title is correct
    let window_titles: Vec<_> = test_app.windows().iter().map(|w| w.title()).collect();
    assert!(
        window_titles.iter().any(|t| t.contains("Add New Snippet")),
        "Window title not found"
    );

    // Check that the form fields are present
    let ui = test_app.windows()[0].ui();
    assert!(ui.label_contains("Title:").any(), "Title label not found");
    assert!(ui.label_contains("Tags:").any(), "Tags label not found");
    assert!(
        ui.label_contains("Content:").any(),
        "Content label not found"
    );
    assert!(ui.button_contains("Save").any(), "Save button not found");
    assert!(
        ui.button_contains("Cancel").any(),
        "Cancel button not found"
    );

    Ok(())
}

#[test]
#[serial]
fn test_gui_validation_error() -> Result<()> {
    let mut test_app = TestApp::new(TestBackendOptions::default(), |_cc| {
        let (tx, _rx) = mpsc::channel();
        Box::new(rustash_cli::gui::ChannelApp {
            app: Default::default(),
            tx,
        })
    });

    test_app.update();
    test_app.update();

    let window = &test_app.windows()[0];
    window.click_button("Save");
    test_app.update();
    assert!(window.ui().label_contains("Title cannot be empty.").any());

    Ok(())
}

// Test that the GUI can submit a new snippet
#[test]
#[serial]
fn test_gui_submit_snippet() -> Result<()> {
    // Create a channel to receive the result
    let (tx, rx) = mpsc::channel();

    // Create a test app with our GUI
    let mut test_app = TestApp::new(TestBackendOptions::default(), move |_cc| {
        let tx = tx.clone();
        Box::new(rustash_cli::gui::ChannelApp {
            app: Default::default(),
            tx,
        })
    });

    // Run the app for a few frames to let it initialize
    test_app.update();
    test_app.update();

    // Get the main window
    let window = &test_app.windows()[0];

    // Fill in the form fields
    window.type_text("Test Snippet", |ui| ui.text_edit_singleline("Title:"));

    window.type_text("test, example", |ui| ui.text_edit_singleline("Tags:"));

    window.type_text("This is a test snippet", |ui| {
        ui.text_edit_multiline("Content:")
    });

    // Click the Save button
    window.click_button("Save");

    // Run the app to process the click
    test_app.update();

    // Check that we received the expected snippet data
    if let Ok(Some(snippet_data)) = rx.try_recv() {
        assert_eq!(snippet_data.title, "Test Snippet");
        assert_eq!(snippet_data.content, "This is a test snippet");
        assert_eq!(
            snippet_data.tags,
            vec!["test".to_string(), "example".to_string()]
        );
    } else {
        panic!("Failed to receive snippet data");
    }

    Ok(())
}
