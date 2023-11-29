use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{glib, Application, ApplicationWindow, Button};

const APP_ID: &str = "com.rouhim.easy-language-config";

const IP_GEOLOCATION_URL: &str =
    "https://api-bdc.net/data/ip-geolocation?key=bdc_c13e3a1984864b699e461a25f5a138ed";

fn main() -> glib::ExitCode {
    // Ensure that in .zshrc is no 'startxfce4 &' command
    execute_command("sed -i '/startxfce4/d' ~/.zshrc");

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
    for language_code in &language_codes {
        keyboard_combo_box
            .borrow()
            .append(Some(&language_code.0), &language_code.1);
    }
    keyboard_combo_box.borrow().set_hexpand(true);
    keyboard_combo_box
        .borrow()
        .set_active_id(Some(&get_current_keyboard_language()));

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
    display_combo_box
        .borrow()
        .set_active_id(Some(&get_current_locale()));

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
    timezone_combo_box
        .borrow()
        .set_active_id(Some(&get_current_timezone()));

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
        let (keyboard, display, timezone) = match auto_detect_language() {
            Some((keyboard, display, timezone)) => (keyboard, display, timezone),
            None => return,
        };
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
    let apply_button = Button::builder().label("Save & restart").build();
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

fn get_current_timezone() -> String {
    execute_command_and_return("timedatectl | grep 'Time zone' | awk '{print $3}'")
        .trim()
        .to_string()
}

/// Returns the current locale in the format "de_DE" without encoding
fn get_current_locale() -> String {
    let mut response =
        execute_command_and_return("localectl | grep 'System Locale' | awk '{print $3}'")
            .trim()
            .to_string();

    // If the response contains = split it and return the second part
    if response.contains('=') {
        response = response.split('=').collect::<Vec<&str>>()[1].to_string();
    }

    // Remove encoding
    response = response.split('.').collect::<Vec<&str>>()[0].to_string();

    response
}

// Returns the current keyboard language code in the format "de"
fn get_current_keyboard_language() -> String {
    let response = execute_command_and_return("localectl | grep 'Keymap' | awk '{print $3}'")
        .trim()
        .to_string();
    response
}

fn auto_detect_language() -> Option<(String, String, String)> {
    let http_response = ureq::get(IP_GEOLOCATION_URL).call();

    let json_response: serde_json::Value = match http_response.is_ok() {
        false => return None,
        true => http_response.unwrap().into_json().unwrap(),
    };

    // country ->isoAlpha2 "DE"
    let country_code = json_response["country"].as_object().unwrap()["isoAlpha2"]
        .as_str()
        .unwrap()
        .to_string();

    // country -> isoAdminLanguages[0] -> isoAlpha2 "de"
    let country_admin_code = json_response["country"].as_object().unwrap()["isoAdminLanguages"]
        .as_array()
        .unwrap()[0]
        .as_object()
        .unwrap()["isoAlpha2"]
        .as_str()
        .unwrap()
        .to_string();

    let display_locale = format!("{}_{}", country_admin_code, country_code);

    // location -> timeZone -> ianaTimeId
    let timezone = json_response["location"].as_object().unwrap()["timeZone"]
        .as_object()
        .unwrap()["ianaTimeId"]
        .as_str()
        .unwrap()
        .to_string();

    Some((country_admin_code, display_locale, timezone))
}

fn apply_to_system(keyboard_language_code: &str, display_locale: &str, timezone: &str) {
    // Set keyboard layout for current session, and persistent
    execute_command(&format!("loadkeys {}", keyboard_language_code));
    execute_command(&format!("localectl set-keymap {}", keyboard_language_code));
    execute_command(&format!(
        "setxkbmap -layout {} -option caps:escape",
        keyboard_language_code
    ));

    // Set display language
    // Add to /etc/locale.gen
    execute_command(&format!(
        "echo '{}.UTF-8 UTF-8' > /etc/locale.gen",
        display_locale
    ));
    // and run locale-gen
    execute_command("locale-gen");

    // Set to /etc/locale.conf
    execute_command(&format!(
        "localectl set-locale LANG={}.UTF-8 --no-ask-password",
        display_locale
    ));

    // Set timezone
    execute_command(&format!(
        "timedatectl set-timezone {} --no-ask-password",
        timezone
    ));

    // Write also to .zshrc
    execute_command(&format!(
        "echo 'export LANG={}.UTF-8' > ~/.zshrc",
        display_locale
    ));

    // startxfce4 automatically
    execute_command("echo 'startxfce4 &' >> ~/.zshrc");

    // Shutdown xfce4
    execute_command("pkill xfce4");

    std::process::exit(0);
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

fn execute_command_and_return(command: &str) -> String {
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .expect("failed to execute process");

    String::from_utf8(output.stdout).unwrap()
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
