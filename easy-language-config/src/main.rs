use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use gtk::glib::{closure_local, spawn_future_local};
use gtk::prelude::*;
use gtk::{glib, Application, ApplicationWindow, Button, DropDown, StringFilterMatchMode};

const APP_ID: &str = "com.rouhim.easy-language-config";

const IP_GEOLOCATION_URL: &str =
    "https://api-bdc.net/data/ip-geolocation?key=bdc_c13e3a1984864b699e461a25f5a138ed";

const DETECTION_TIMEOUT: Duration = Duration::from_secs(5);

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_ui);

    app.run()
}

fn build_ui(app: &Application) {
    let layouts = get_layouts();
    let locales = get_locales();
    let timezones = get_timezones();

    let status_label = gtk::Label::new(None);
    status_label.set_xalign(0.0);
    status_label.set_wrap(true);
    status_label.set_margin_top(4);

    // Keyboard label //////////////////////////////////////////////////////////////////////////////
    let keyboard_label = gtk::Label::new(Some("Keyboard layout:"));
    keyboard_label.set_xalign(0.0);
    keyboard_label.set_width_chars(17);

    // Keyboard combo box
    let keyboard_drop_down = Rc::new(RefCell::new(build_dropdown(get_values(&layouts))));
    keyboard_drop_down.borrow().set_hexpand(true);
    keyboard_drop_down.borrow().set_selected(get_pos_map_or(
        &layouts,
        &get_current_keyboard_layout(),
        "us",
    ));

    // Keyboard layout
    let keyboard_layout = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    keyboard_layout.append(&keyboard_label);
    keyboard_layout.append(&*keyboard_drop_down.borrow());
    keyboard_layout.set_margin_bottom(8);
    ////////////////////////////////////////////////////////////////////////////////////////////////

    // Display label ///////////////////////////////////////////////////////////////////////////////
    let display_label = gtk::Label::new(Some("Display:"));
    display_label.set_xalign(0.0);
    display_label.set_width_chars(17);

    // Display combobox
    let display_drop_down = Rc::new(RefCell::new(build_dropdown(get_values(&locales))));
    display_drop_down.borrow().set_hexpand(true);
    display_drop_down.borrow().set_selected(get_pos_map_or(
        &locales,
        &get_current_locale(),
        "en_US",
    ));

    // Display layout
    let display_layout = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    display_layout.append(&display_label);
    display_layout.append(&*display_drop_down.borrow());
    display_layout.set_margin_bottom(8);
    ////////////////////////////////////////////////////////////////////////////////////////////////

    // Timezone label //////////////////////////////////////////////////////////////////////////////
    let timezone_label = gtk::Label::new(Some("Timezone:"));
    timezone_label.set_xalign(0.0);
    timezone_label.set_width_chars(17);

    // Timezone combobox
    let timezone_drop_down = Rc::new(RefCell::new(build_dropdown(timezones.clone())));
    timezone_drop_down.borrow().set_hexpand(true);
    timezone_drop_down.borrow().set_selected(get_pos_vec_or(
        &timezones,
        &get_current_timezone(),
        "UTC",
    ));

    // Timezone layout
    let timezone_layout = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    timezone_layout.append(&timezone_label);
    timezone_layout.append(&*timezone_drop_down.borrow());
    timezone_layout.set_margin_bottom(8);
    ////////////////////////////////////////////////////////////////////////////////////////////////

    // Create automatic detection button ///////////////////////////////////////////////////////////
    let auto_detect_button = Button::builder().label("Auto-detect by your IP").build();
    auto_detect_button.set_margin_bottom(8);

    let keyboard_for_detect = keyboard_drop_down.clone();
    let display_for_detect = display_drop_down.clone();
    let timezone_for_detect = timezone_drop_down.clone();
    let status_for_detect = status_label.clone();
    let layouts_for_detect = layouts.clone();
    let locales_for_detect = locales.clone();
    let timezones_for_detect = timezones.clone();

    auto_detect_button.connect_clicked(move |button| {
        let layouts = layouts_for_detect.clone();
        let locales = locales_for_detect.clone();
        let timezones = timezones_for_detect.clone();
        let keyboard = keyboard_for_detect.clone();
        let display = display_for_detect.clone();
        let timezone = timezone_for_detect.clone();
        let status = status_for_detect.clone();
        let button = button.clone();

        button.set_sensitive(false);
        status.set_text("Detecting language and timezone…");

        let (sender, receiver) = futures_channel::oneshot::channel();
        std::thread::spawn(move || {
            let _ = sender.send(detect());
        });

        spawn_future_local(async move {
            let message = match receiver.await {
                Ok(Ok(detection)) => {
                    let keyboard_pos = find_pos_map(&layouts, &detection.layout);
                    let display_pos = find_pos_map(&locales, &detection.locale);
                    let timezone_pos = find_pos_vec(&timezones, &detection.timezone);

                    match (keyboard_pos, display_pos, timezone_pos) {
                        (Some(keyboard_pos), Some(display_pos), Some(timezone_pos)) => {
                            keyboard.borrow().set_selected(keyboard_pos);
                            display.borrow().set_selected(display_pos);
                            timezone.borrow().set_selected(timezone_pos);
                            "Detected values from your IP. Review them and save.".to_string()
                        }
                        _ => format!(
                            "Auto-detect returned unsupported values: {} / {} / {}",
                            detection.layout, detection.locale, detection.timezone
                        ),
                    }
                }
                Ok(Err(error)) => format!("Auto-detect failed: {error}"),
                Err(_) => "Auto-detect failed: no response".to_string(),
            };

            status.set_text(&message);
            button.set_sensitive(true);
        });
    });
    ////////////////////////////////////////////////////////////////////////////////////////////////

    // Create save button //////////////////////////////////////////////////////////////////////////
    let apply_button = Button::builder().label("Save & restart").build();

    let keyboard_for_save = keyboard_drop_down.clone();
    let display_for_save = display_drop_down.clone();
    let timezone_for_save = timezone_drop_down.clone();
    let status_for_save = status_label.clone();
    let layouts_for_save = layouts;
    let locales_for_save = locales;

    apply_button.connect_clicked(move |_| {
        let Some(layout_name) = selected_string(&keyboard_for_save) else {
            status_for_save.set_text("Select a keyboard layout first.");
            return;
        };
        let Some(display_name) = selected_string(&display_for_save) else {
            status_for_save.set_text("Select a display language first.");
            return;
        };
        let Some(timezone_name) = selected_string(&timezone_for_save) else {
            status_for_save.set_text("Select a timezone first.");
            return;
        };

        let Some((layout, _)) = layouts_for_save
            .iter()
            .find(|(_, value)| value == &layout_name)
        else {
            status_for_save.set_text("The selected keyboard layout is not known.");
            return;
        };
        let Some((locale, _)) = locales_for_save
            .iter()
            .find(|(_, value)| value == &display_name)
        else {
            status_for_save.set_text("The selected display language is not known.");
            return;
        };

        match save_to_system(layout, locale, &timezone_name) {
            Ok(()) => std::process::exit(0),
            Err(error) => status_for_save.set_text(&format!("Could not save settings: {error}")),
        }
    });
    ////////////////////////////////////////////////////////////////////////////////////////////////

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
    base_layout.append(&status_label);

    // Create a window
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Easy language setup")
        .default_width(620)
        .child(&base_layout)
        .build();

    // Present window
    window.present();
}

fn selected_string(drop_down: &Rc<RefCell<DropDown>>) -> Option<String> {
    let item = drop_down.borrow().selected_item()?;
    item.downcast::<gtk::StringObject>()
        .ok()
        .map(|object| object.string().to_string())
}

/// Returns the position of the given key in the given vector
fn find_pos_map(key_value_map: &[(String, String)], to_select: &str) -> Option<u32> {
    key_value_map
        .iter()
        .position(|(key, _)| key == to_select)
        .map(|position| position as u32)
}

/// Returns the position of the given value in the given vector
fn find_pos_vec(values: &[String], to_select: &str) -> Option<u32> {
    values
        .iter()
        .position(|value| value == to_select)
        .map(|position| position as u32)
}

fn get_pos_map_or(key_value_map: &[(String, String)], to_select: &str, fallback: &str) -> u32 {
    find_pos_map(key_value_map, to_select)
        .or_else(|| find_pos_map(key_value_map, fallback))
        .unwrap_or(0)
}

fn get_pos_vec_or(values: &[String], to_select: &str, fallback: &str) -> u32 {
    find_pos_vec(values, to_select)
        .or_else(|| find_pos_vec(values, fallback))
        .unwrap_or(0)
}

fn get_values(key_value_vec: &[(String, String)]) -> Vec<String> {
    key_value_vec
        .iter()
        .map(|entry| entry.1.clone())
        .collect::<Vec<String>>()
}

fn get_current_timezone() -> String {
    let timezone = execute_command_and_return("timedatectl show --property=Timezone --value");
    let timezone = timezone.trim();
    if timezone.is_empty() {
        "UTC".to_string()
    } else {
        timezone.to_string()
    }
}

/// Returns the current locale in the format "de_DE" without encoding
fn get_current_locale() -> String {
    let output = execute_command_and_return(
        "localectl status | awk -F'LANG=' '/System Locale/{print $2; exit}'",
    );
    let locale = output.trim().split('.').next().unwrap_or("").trim();
    if locale.is_empty() {
        "en_US".to_string()
    } else {
        locale.to_string()
    }
}

/// Returns the current xkb layout in the format "de"
fn get_current_keyboard_layout() -> String {
    let output = execute_command_and_return("setxkbmap -query | awk '/^layout:/{print $2; exit}'");
    let layout = output.trim();
    if !layout.is_empty() {
        return layout
            .split(',')
            .next()
            .unwrap_or(layout)
            .trim()
            .to_string();
    }

    let output =
        execute_command_and_return("localectl status | awk -F': ' '/X11 Layout/{print $2; exit}'");
    let layout = output.trim();
    if !layout.is_empty() && layout != "n/a" {
        return layout
            .split(',')
            .next()
            .unwrap_or(layout)
            .trim()
            .to_string();
    }

    "us".to_string()
}

fn detect() -> Result<Detection, String> {
    let (country, timezone) = fetch_geolocation()?;
    let (locale, layout) = country_default(&country)
        .ok_or_else(|| format!("no default language for country {country}"))?;

    Ok(Detection {
        layout,
        locale,
        timezone,
    })
}

fn fetch_geolocation() -> Result<(String, String), String> {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(DETECTION_TIMEOUT))
        .build();
    let agent = ureq::Agent::new_with_config(config);

    let mut response = agent
        .get(IP_GEOLOCATION_URL)
        .call()
        .map_err(|error| format!("request failed: {error}"))?;
    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|error| format!("could not read response: {error}"))?;

    parse_geolocation(&body)
}

fn parse_geolocation(json: &str) -> Result<(String, String), String> {
    let response: serde_json::Value =
        serde_json::from_str(json).map_err(|error| format!("invalid response: {error}"))?;

    let country = response
        .pointer("/country/isoAlpha2")
        .and_then(|value| value.as_str())
        .ok_or("response has no country code")?;
    if country.len() != 2 {
        return Err(format!("invalid country code: {country}"));
    }

    let timezone = response
        .pointer("/location/timeZone/ianaTimeId")
        .and_then(|value| value.as_str())
        .ok_or("response has no timezone")?;

    Ok((country.to_ascii_uppercase(), timezone.to_string()))
}

fn save_to_system(layout: &str, display_locale: &str, timezone: &str) -> Result<(), String> {
    // Set the xkb layout for the running session and persist it for X11.
    run_step(&format!("setxkbmap -layout {layout}"))?;
    run_step(&format!("localectl set-x11-keymap {layout}"))?;

    // The virtual console uses a separate keymap namespace. Only touch it when
    // a matching kbd keymap is known.
    if let Some(keymap) = vconsole_keymap(layout) {
        run_step(&format!("loadkeys {keymap}"))?;
        run_step(&format!("localectl set-keymap --no-convert {keymap}"))?;
    }

    // Set the display language.
    run_step(&format!(
        "printf '%s.UTF-8 UTF-8\n' '{display_locale}' > /etc/locale.gen"
    ))?;
    run_step("locale-gen")?;
    run_step(&format!("localectl set-locale LANG={display_locale}.UTF-8"))?;

    // Set the timezone.
    run_step(&format!("timedatectl set-timezone {timezone}"))?;

    // Restart the XFCE session; getty autologin brings it back up.
    run_step("pkill -x xfce4-session")
}

fn vconsole_keymap(layout: &str) -> Option<&'static str> {
    Some(match layout {
        "us" => "us",
        "gb" => "uk",
        "de" => "de",
        "at" => "de",
        "ch" => "de_CH-latin1",
        "be" => "be-latin1",
        "fr" => "fr",
        "es" | "latam" => "es",
        "pt" => "pt-latin1",
        "br" => "br-abnt2",
        "it" => "it",
        "nl" => "nl",
        "no" => "no",
        "dk" => "dk",
        "se" => "sv-latin1",
        "fi" => "fi",
        "is" => "is-latin1",
        "pl" => "pl",
        "cz" => "cz",
        "sk" => "sk-qwertz",
        "hu" => "hu",
        "ro" => "ro",
        "ru" => "ru",
        "ua" => "ua",
        "by" => "by",
        "bg" => "bg_pho-utf8",
        "gr" => "gr",
        "tr" => "trq",
        "il" => "il",
        "ir" => "fa",
        "jp" => "jp106",
        "rs" => "sr-latin",
        "hr" => "croat",
        "si" => "slovene",
        "mk" => "mk-utf",
        "ge" => "ge",
        "kz" => "kazakh",
        "kg" => "kyrgyz",
        "ie" => "ie",
        "ca" => "ca",
        _ => return None,
    })
}

fn run_step(command: &str) -> Result<(), String> {
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .map_err(|error| format!("could not run `{command}`: {error}"))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stderr = stderr.trim();
    if stderr.is_empty() {
        Err(format!("`{command}` failed"))
    } else {
        Err(format!("`{command}` failed: {stderr}"))
    }
}

fn execute_command_and_return(command: &str) -> String {
    match std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
    {
        Ok(output) => String::from_utf8_lossy(&output.stdout).into_owned(),
        Err(_) => String::new(),
    }
}

fn get_layouts() -> Vec<(String, String)> {
    let mut entries = parse_csv(include_str!("../assets/layouts.csv"));
    entries.sort_by(|a, b| a.1.cmp(&b.1));
    entries
}

fn get_locales() -> Vec<(String, String)> {
    let mut entries = parse_csv(include_str!("../assets/locales.csv"));
    entries.sort_by(|a, b| a.1.cmp(&b.1));
    entries
}

fn get_timezones() -> Vec<String> {
    let mut entries: Vec<String> = include_str!("../assets/timezones.txt")
        .lines()
        .map(|line| line.to_string())
        .collect();
    entries.sort();
    entries
}

fn country_default(country: &str) -> Option<(String, String)> {
    let country = country.trim().to_ascii_uppercase();
    for line in include_str!("../assets/country_defaults.csv")
        .lines()
        .skip(1)
    {
        let mut parts = line.splitn(3, ',');
        let (Some(code), Some(locale), Some(layout)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        if code == country {
            return Some((locale.to_string(), layout.to_string()));
        }
    }
    None
}

fn parse_csv(csv_data: &str) -> Vec<(String, String)> {
    csv_data
        .lines()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let (key, value) = line.split_once(',')?;
            Some((key.trim().to_string(), value.trim().to_string()))
        })
        .collect()
}

fn build_dropdown(string_elements: Vec<String>) -> DropDown {
    let elements = string_elements
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<&str>>();
    let selection_model = gtk::StringList::new(elements.as_slice());
    DropDown::builder()
        .model(&selection_model)
        .enable_search(true)
        .search_match_mode(StringFilterMatchMode::Substring)
        .show_arrow(true)
        .expression(gtk::ClosureExpression::new::<Option<String>>(
            gtk::Expression::NONE,
            closure_local!(move |item: glib::Object| {
                item.downcast_ref::<gtk::StringObject>()
                    .map(|item| item.string().to_string())
            }),
        ))
        .build()
}

struct Detection {
    layout: String,
    locale: String,
    timezone: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_csv_keeps_commas_in_display_names() {
        let rows = parse_csv("id,value\nbs_Cyrl_BA,Bosnian (Cyrillic, Bosnia & Herzegovina)\n");
        assert_eq!(
            rows,
            vec![(
                "bs_Cyrl_BA".to_string(),
                "Bosnian (Cyrillic, Bosnia & Herzegovina)".to_string()
            )]
        );
    }

    #[test]
    fn locale_and_layout_names_are_unique() {
        let mut locales = get_values(&get_locales());
        let locale_count = locales.len();
        locales.sort();
        locales.dedup();
        assert_eq!(locales.len(), locale_count);

        let mut layouts = get_values(&get_layouts());
        let layout_count = layouts.len();
        layouts.sort();
        layouts.dedup();
        assert_eq!(layouts.len(), layout_count);
    }

    #[test]
    fn generated_country_defaults_reference_known_values() {
        let locales = get_locales();
        let layouts = get_layouts();

        let mut count = 0;
        for line in include_str!("../assets/country_defaults.csv")
            .lines()
            .skip(1)
        {
            let mut parts = line.splitn(3, ',');
            let (Some(country), Some(locale), Some(layout)) =
                (parts.next(), parts.next(), parts.next())
            else {
                panic!("malformed country default: {line}");
            };

            assert_eq!(country.len(), 2, "invalid country code: {country}");
            assert!(
                locales.iter().any(|(key, _)| key == locale),
                "unknown locale {locale} for {country}"
            );
            assert!(
                layouts.iter().any(|(key, _)| key == layout),
                "unknown layout {layout} for {country}"
            );
            count += 1;
        }
        assert_eq!(count, 257);
    }

    #[test]
    fn country_default_lookup_is_case_insensitive() {
        assert_eq!(
            country_default("DE"),
            Some(("de_DE".to_string(), "de".to_string()))
        );
        assert_eq!(
            country_default("ch"),
            Some(("de_CH".to_string(), "ch".to_string()))
        );
        assert_eq!(country_default("ZZ"), None);
    }

    #[test]
    fn geolocation_parsing_ignores_missing_admin_languages() {
        let json = r#"{
            "country": {"isoAlpha2": "DE", "isoAdminLanguages": []},
            "location": {"timeZone": {"ianaTimeId": "Europe/Berlin"}}
        }"#;
        assert_eq!(
            parse_geolocation(json),
            Ok(("DE".to_string(), "Europe/Berlin".to_string()))
        );
    }

    #[test]
    fn geolocation_parsing_rejects_incomplete_responses() {
        assert!(parse_geolocation("{}").is_err());
        assert!(parse_geolocation(r#"{"country": {"isoAlpha2": "DE"}, "location": {}}"#).is_err());
        assert!(parse_geolocation(r#"{"country": {"isoAlpha2": "D"}}"#).is_err());
        assert!(parse_geolocation("not json").is_err());
    }

    #[test]
    fn selection_falls_back_to_sane_defaults() {
        let locales = get_locales();
        assert_eq!(
            get_pos_map_or(&locales, "C", "en_US"),
            get_pos_map_or(&locales, "en_US", "en_US")
        );

        let timezones = get_timezones();
        assert_eq!(
            get_pos_vec_or(&timezones, "Mars/Olympus", "UTC"),
            get_pos_vec_or(&timezones, "UTC", "UTC")
        );
    }

    #[test]
    fn vconsole_keymaps_are_known() {
        assert_eq!(vconsole_keymap("ch"), Some("de_CH-latin1"));
        assert_eq!(vconsole_keymap("gb"), Some("uk"));
        assert_eq!(vconsole_keymap("cn"), None);
    }
}
