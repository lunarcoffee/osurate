#![cfg(feature = "gui")]

use std::path::PathBuf;
use std::process;
use std::sync::Arc;

use druid::{
    AppDelegate, AppLauncher, Color, Command, commands, Data, DelegateCtx, Env, FileDialogOptions, FileSpec, Handled,
    Lens, Target, Widget, WidgetExt, WindowDesc,
};
use druid::widget::{Button, Flex, Label, LineBreaking, TextBox};

use crate::util;

pub fn run_gui() -> ! {
    let main_window = WindowDesc::new(make_ui)
        .title("osurate | osu! Rate Generator")
        .window_size((460., 380.))
        .resizable(false);

    let data = AppData { rates_str: Arc::new(String::new()), files: vec![], status: "[Info] started".to_string() };
    AppLauncher::with_window(main_window).delegate(Delegate {}).launch(data)
        .unwrap_or_else(|_| util::log_fatal("failed to start gui"));
    process::exit(0)
}

#[derive(Clone, Lens)]
struct AppData {
    rates_str: Arc<String>,
    files: Vec<PathBuf>,
    status: String,
}

impl Data for AppData {
    fn same(&self, other: &Self) -> bool {
        self.rates_str == other.rates_str && self.files == other.files && self.status == other.status
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
        .padding((6., 7., 6., 2.));

    let select_files_button = Button::new("Select Beatmap")
        .on_click(|ctx, _, _| {
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

    let clear_button = Button::new("Clear")
        .on_click(|_, data: &mut AppData, _| data.files.clear())
        .padding(4.);

    // This blocks the UI thread when pressed, not a huge deal though.
    let generate_button = Button::new("Generate")
        .on_click(|_, data: &mut AppData, _| {
            let rates_str = data.rates_str.to_string();
            let rates_iter = rates_str.split(",").map(|r| r.parse::<f64>());
            let rates = match rates_iter.collect::<Result<Vec<_>, _>>() {
                Ok(r) if r.iter().all(|&r| r >= 0.01) => r,
                _ => {
                    data.status = "[Error] invalid rate(s) specified".to_string();
                    return;
                }
            };

            // Unlike the CLI version, press on after encountering errors.
            for file in &data.files {
                data.status = match crate::generate_rates(file, &rates) {
                    Err(e) => format!("[Error] {}", e),
                    Ok(map_name) => format!("[Info] generated rate(s) for {}", map_name),
                };
            }
        })
        .padding(6.);

    let configure_label = |l: Label<AppData>| l
        .with_line_break_mode(LineBreaking::WordWrap)
        .with_text_size(12.)
        .background(Color::grey(0.12))
        .border(Color::grey(0.12), 3.)
        .rounded(4.)
        .expand_width();

    let selected_maps_label = configure_label(Label::dynamic(
        |data: &AppData, _| {
            let name = data.files.iter()
                .map(|f| f.file_name().unwrap().to_string_lossy().trim_end_matches(".osu").to_string())
                .collect::<Vec<_>>()
                .join("\n");
            format!("Selected map(s):\n{}", if name.is_empty() { "(none)".to_string() } else { name })
        }))
        .expand_height()
        .padding((6., 1., 6., 6.));

    let status_label = configure_label(Label::dynamic(|data: &AppData, _| data.status.to_string()))
        .padding((6., 2., 6., 6.));

    Flex::column()
        .with_child(rates_input)
        .with_child(Flex::row()
            .with_child(select_files_button)
            .with_child(undo_button)
            .with_child(clear_button)
            .with_child(generate_button))
        .with_flex_child(selected_maps_label, 1.)
        .with_child(status_label)
        .background(Color::grey(0.05))
}
