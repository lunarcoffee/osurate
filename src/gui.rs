#![cfg(feature = "gui")]

use std::fs::File;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::Arc;

use druid::{
    AppDelegate, AppLauncher, Color, Command, commands, Data, DelegateCtx, Env, FileDialogOptions, FileInfo, FileSpec,
    Handled, Lens, LocalizedString, PlatformError, Selector, Target, Widget, WidgetExt, WindowDesc,
};
use druid::text::{BasicTextInput, Editor};
use druid::widget::{Button, Flex, Label, LineBreaking, List, TextBox};

use crate::{audio, util};
use crate::audio::AudioStretchError;
use crate::beatmap::{Beatmap, ParseError};

pub fn run_gui() -> ! {
    let main_window = WindowDesc::new(make_ui).title("osurate | osu! Rate Generator").resizable(false);
    let data = AppData { rates_str: Arc::new(String::new()), files: vec![], log: "Info: started\n".to_string() };

    AppLauncher::with_window(main_window)
        .delegate(Delegate {})
        .use_simple_logger()
        .launch(data)
        .unwrap_or_else(|_| util::log_fatal("failed to start gui"));
    process::exit(0)
}

#[derive(Clone, Lens)]
struct AppData {
    rates_str: Arc<String>,
    files: Vec<PathBuf>,
    log: String,
}

impl Data for AppData {
    fn same(&self, other: &Self) -> bool {
        self.rates_str == other.rates_str && self.files == other.files && self.log == other.log
    }
}

struct Delegate;

impl AppDelegate<AppData> for Delegate {
    // When the user selects a file, store it.
    fn command(&mut self, _: &mut DelegateCtx, _: Target, cmd: &Command, data: &mut AppData, _: &Env) -> Handled {
        if let Some(file_info) = cmd.get(commands::OPEN_FILE) {
            let path = file_info.path().to_path_buf();
            data.files.push(path);
            Handled::Yes
        } else {
            Handled::No
        }
    }
}

fn make_ui() -> impl Widget<AppData> {
    let rates_input = TextBox::new()
        .with_placeholder("Rates (i.e. 1.1,1.15,1.2)")
        .lens(AppData::rates_str)
        .expand_width()
        .padding((8., 8., 8., 4.));

    let select_files_button = Button::new("Select Beatmap")
        .on_click(|ctx, data, _| {
            // Opening multiple files is currently unsupported in Druid (#1067).
            let options = FileDialogOptions::new()
                .title("Select a beatmap to generate rates for")
                .button_text("Select")
                .allowed_types(vec![FileSpec::new("osu! beatmaps", &["osu"])]);
            ctx.submit_command(Command::new(commands::SHOW_OPEN_PANEL, options, Target::Auto));
        })
        .padding(4.);

    let undo_button = Button::new("Remove Last")
        .on_click(|_, data: &mut AppData, _| { let _ = data.files.pop(); })
        .padding(4.);
    let clear_button = Button::new("Clear").on_click(|_, data: &mut AppData, _| data.files.clear()).padding(4.);

    let generate_button = Button::new("Generate")
        .on_click(|ctx, data: &mut AppData, _| {
            let rates_str = data.rates_str.to_string();
            let rates_iter = rates_str.split(",").map(|r| r.parse::<f64>());
            let rates = match rates_iter.collect::<Result<Vec<_>, _>>() {
                Ok(r) if r.iter().all(|&r| r >= 0.01) => r,
                _ => {
                    data.log += "Error: rates below 0.01 are not supported";
                    return;
                }
            };

            // Unlike the CLI version, press on after encountering errors.
            for file in &data.files {
                data.log += &match crate::generate_rates(file, &rates) {
                    Err(e) => format!("Error: {}\n", e),
                    Ok(map_name) => format!("Info: generated rate(s) for {}\n", map_name),
                };
            }
        })
        .padding(8.);

    let selected_maps_label = Label::dynamic(
        |data: &AppData, _| {
            let name = data.files.iter()
                .map(|f| f.file_name().unwrap().to_string_lossy().trim_end_matches(".osu").to_string())
                .collect::<Vec<_>>()
                .join("\n");
            format!("Selected map(s):\n{}", if name.is_empty() { "(none)".to_string() } else { name })
        })
        .with_line_break_mode(LineBreaking::WordWrap)
        .with_text_size(12.)
        .background(Color::grey(0.12))
        .rounded(4.)
        .expand_width()
        .height(153.)
        .padding((8., 3., 8., 6.));

    let log_label = Label::dynamic(|data: &AppData, _| data.log.to_string())
        .with_line_break_mode(LineBreaking::WordWrap)
        .with_text_size(12.)
        .background(Color::grey(0.12))
        .rounded(4.)
        .expand_width()
        .height(153.)
        .padding((8., 4.));

    Flex::column()
        .with_child(rates_input)
        .with_child(Flex::row()
            .with_child(select_files_button)
            .with_child(undo_button)
            .with_child(clear_button)
            .with_child(generate_button))
        .with_child(selected_maps_label)
        .with_child(log_label)
        .background(Color::grey(0.05))
}
