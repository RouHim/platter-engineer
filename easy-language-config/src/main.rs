use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{glib, Application, ApplicationWindow, Button};

const APP_ID: &str = "org.gtk_rs.HelloWorld2";

fn main() -> glib::ExitCode {
    // Create a new application
    let app = Application::builder().application_id(APP_ID).build();

    // Connect to "activate" signal of `app`
    app.connect_activate(build_ui);

    // Run the application
    app.run()
}

fn build_ui(app: &Application) {
    let language_codes = get_language_codes();
    let locales = get_locales();
    let timezones = get_timezones();

    // Keyboard label
    let keyboard_label = gtk::Label::new(Some("Keyboard:"));
    keyboard_label.set_xalign(0.0);
    keyboard_label.set_width_chars(15);

    // Keyboard combo box
    let keyboard_combo_box = Rc::new(RefCell::new(gtk::ComboBoxText::new()));
    // Enable search by type
    for language_cocde in &language_codes {
        keyboard_combo_box
            .borrow()
            .append(Some(&language_cocde.0), &language_cocde.1);
    }
    keyboard_combo_box.borrow().set_hexpand(true);
    keyboard_combo_box.borrow().set_active_id(Some("en"));

    // Keyboard layout
    let keyboard_layout = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    keyboard_layout.append(&keyboard_label);
    keyboard_layout.append(&*keyboard_combo_box.borrow());
    keyboard_layout.set_margin_bottom(8);

    // Display label
    let display_label = gtk::Label::new(Some("Display:"));
    display_label.set_xalign(0.0);
    display_label.set_width_chars(15);

    // Display combobox
    let display_combo_box = Rc::new(RefCell::new(gtk::ComboBoxText::new()));
    // Enable search by type
    for locale in &locales {
        display_combo_box
            .borrow()
            .append(Some(&locale.0), &locale.1);
    }
    display_combo_box.borrow().set_hexpand(true);
    display_combo_box.borrow().set_active_id(Some("en_US"));

    // Display layout
    let display_layout = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    display_layout.append(&display_label);
    display_layout.append(&*display_combo_box.borrow());
    display_layout.set_margin_bottom(8);

    // Timezone label
    let timezone_label = gtk::Label::new(Some("Timezone:"));
    timezone_label.set_xalign(0.0);
    timezone_label.set_width_chars(15);

    // Timezone combobox
    let timezone_combo_box = Rc::new(RefCell::new(gtk::ComboBoxText::new()));
    // Enable search by type
    for timezone in &timezones {
        timezone_combo_box.borrow().append(Some(timezone), timezone);
    }
    timezone_combo_box.borrow().set_hexpand(true);
    timezone_combo_box.borrow().set_active_id(Some("UTC"));

    // Timezone layout
    let timezone_layout = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    timezone_layout.append(&timezone_label);
    timezone_layout.append(&*timezone_combo_box.borrow());
    timezone_layout.set_margin_bottom(8);

    // Create automatic detection button
    let auto_detect_button = Button::builder().label("Auto-detect by your IP").build();
    auto_detect_button.set_margin_bottom(8);
    let keyboard_combo_box_clone = keyboard_combo_box.clone();
    let display_combo_box_clone = display_combo_box.clone();
    let timezone_combo_box_clone = timezone_combo_box.clone();
    auto_detect_button.connect_clicked(move |_| {
        let (keyboard, display, timezone) = auto_detect_language();
        keyboard_combo_box_clone
            .borrow()
            .set_active_id(Some(&keyboard));
        display_combo_box_clone
            .borrow()
            .set_active_id(Some(&display));
        timezone_combo_box_clone
            .borrow()
            .set_active_id(Some(&timezone));
    });

    // Create save button
    let apply_button = Button::builder().label("Save").build();
    // When button is clicked, determine current selection of combo boxes and save them
    apply_button.connect_clicked(move |_| {
        if keyboard_combo_box.borrow().active_id().is_none()
            || display_combo_box.borrow().active_id().is_none()
        {
            return;
        }

        let keyboard_selection = keyboard_combo_box.borrow().active_id().unwrap();
        let display_selection = display_combo_box.borrow().active_id().unwrap();
        let timezone_selection = timezone_combo_box.borrow().active_id().unwrap();

        apply_to_system(&keyboard_selection, &display_selection, &timezone_selection);
    });

    // Create a vertical layout
    let base_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
    base_layout.set_margin_top(8);
    base_layout.set_margin_bottom(8);
    base_layout.set_margin_start(8);
    base_layout.set_margin_end(8);
    base_layout.append(&auto_detect_button);
    base_layout.append(&keyboard_layout);
    base_layout.append(&display_layout);
    base_layout.append(&timezone_layout);
    base_layout.append(&apply_button);

    // Create a window
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Easy language setup")
        .child(&base_layout)
        .build();

    // Present window
    window.present();
}

fn auto_detect_language() -> (String, String, String) {
    let json_response: serde_json::Value = ureq::get("https://ipapi.co/json/")
        .call()
        .unwrap()
        .into_json()
        .unwrap();

    let language_code = json_response["languages"]
        .as_str()
        .unwrap()
        .split(',')
        .next()
        .unwrap()
        .to_string();
    let display_locale = format!(
        "{}_{}",
        language_code,
        json_response["country_code"].as_str().unwrap()
    );
    let timezone = json_response["timezone"].as_str().unwrap().to_string();

    (language_code, display_locale, timezone)
}

fn apply_to_system(keyboard_language_code: &str, display_locale: &str, timezone: &str) {
    println!("Keyboard: {}", keyboard_language_code);
    println!("Display: {}", display_locale);

    // Set keyboard layout
    execute_command(&format!(
        "setxkbmap -layout {} -option caps:escape",
        keyboard_language_code
    ));

    // Set display language
    let locale_string = format!("{}.UTF-8", display_locale);
    execute_command(&format!(
        "localectl set-locale LANG={} --no-ask-password",
        locale_string
    ));

    // Set timezone
    execute_command(&format!(
        "timedatectl set-timezone {} --no-ask-password",
        timezone
    ));

    // Reload xfce gui
    execute_command("xfce4-panel -r");
}

fn execute_command(command: &str) -> bool {
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .expect("failed to execute process");

    if output.status.success() {
        return true;
    }
    false
}

fn get_language_codes() -> Vec<(String, String)> {
    parse_csv(include_str!("../assets/language-codes_csv.csv"))
}

fn get_locales() -> Vec<(String, String)> {
    parse_csv(include_str!("../assets/locales.csv"))
}

fn get_timezones() -> Vec<String> {
    include_str!("../assets/timezones.txt")
        .lines()
        .map(|s| s.to_string())
        .collect()
}

fn parse_csv(csv_data: &str) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    for line in csv_data.lines().skip(1) {
        let mut line_split = line.split(',');
        let key = line_split.next().unwrap().trim();
        let value = line_split.next().unwrap().trim();
        entries.push((key.to_string(), value.to_string()));
    }
    entries
}
