use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::OnceLock;

use clap::Parser;
use compact_view::generate_compact_view;
use file_object::FileObject;
use gtk::gio::{ApplicationFlags, ListStore};
use gtk::glib::{self, clone, set_program_name, Continue, MainContext, Priority};
use gtk::{gio, Application, ApplicationWindow, EventControllerKey, PolicyType, ScrolledWindow};
use gtk::prelude::*;
use list_view::{create_outer_box, generate_list_view};

mod compact_view;
mod file_object;
mod list_view;
mod util;

#[derive(Parser, Clone, Debug)]
#[command(about, version)]
struct Cli {
    /// Be verbose
    #[arg(short, long)]
    verbose: bool,

    /// Act as a target instead of source
    #[arg(short, long)]
    target: bool,

    /// With --target, keep files to drag out
    #[arg(short, long, requires = "target")]
    keep: bool,

    /// Make the window resizable
    #[arg(short, long)]
    resizable: bool,

    /// Exit after first successful drag or drop
    #[arg(short = 'x', long)]
    and_exit: bool,

    /// Only display icons, no labels
    #[arg(short, long)]
    icons_only: bool,

    /// Don't load thumbnails from images
    #[arg(short, long)]
    disable_thumbnails: bool,

    /// Size of icons and thumbnails
    #[arg(short = 's', long, value_name = "SIZE", default_value_t = 32)]
    icon_size: i32,

    /// Min width of the main window
    #[arg(short = 'W', long, value_name = "WIDTH", default_value_t = 360)]
    content_width: i32,

    /// Default height of the main window
    #[arg(short = 'H', long, value_name = "HEIGHT", default_value_t = 360)]
    content_height: i32,

    /// Accept paths from stdin
    #[arg(short = 'I', long)]
    from_stdin: bool,

    /// Show a drag all button
    #[arg(short = 'a', long)]
    all: bool,

    /// Show only the number of items and drag them together
    #[arg(short = 'A', long)]
    all_compact: bool,

    /// Don't open files on click
    #[arg(short = 'n', long)]
    no_click: bool,

    /// Paths to the files you want to drag
    #[arg(value_name = "PATH")]
    paths: Vec<PathBuf>,

    /// Always show basename of each file
    #[arg(short = 'b', long)]
    basename: bool,
}

// switch to Lazy Cell when it is stable.
/// Constant that is initialized once on start. Contains the directory where it was started from.
static CURRENT_DIRECTORY: OnceLock<gio::File> = OnceLock::new();
/// Constant that is initialized once on start. clap will parse the commandline arguments into this.
static ARGS: OnceLock<Cli> = OnceLock::new();

fn main() {
    CURRENT_DIRECTORY
        .set(gio::File::for_path("."))
        .expect("Could not set CURRENT_DIRECTORY");
    let args = Cli::parse();
    ARGS.set(args).expect("Could not set ARGS");
    set_program_name(Some("ripdrag"));
    let app = Application::builder()
        .application_id("it.catboy.ripdrag")
        .flags(ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_activate(build_ui);
    app.run_with_args(&[""]); // we don't want gtk to parse the arguments. cleaner solutions are welcome
}

fn build_ui(app: &Application) {
    // Parse arguments and check if files exist
    for path in &ARGS.get().unwrap().paths {
        assert!(
            path.exists(),
            "{0} : no such file or directory",
            path.display()
        );
    }
    // Create a scrollable list
    let (outer_box, list_data) = if ARGS.get().unwrap().all_compact {
        // Create compact list
        (None, generate_compact_view())
    } else if ARGS.get().unwrap().all {
        // Create regular list with a drag all button
        let list = generate_list_view();
        (Some(create_outer_box(&list).upcast::<gtk::Widget>()), list)
    } else {
        // Create regular list
        (None, generate_list_view())
    };

    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never) //  Disable horizontal scrolling
        .vexpand(true)
        .hexpand(true)
        .child(if let Some(outer_box) = &outer_box {
            outer_box
        } else {
            &list_data.widget
        })
        .build();

    let titlebar = gtk::HeaderBar::builder()
        .show_title_buttons(false)
        .visible(false)
        .build();

    // Build the main window
    let window = ApplicationWindow::builder()
        .title("ripdrag")
        .resizable(ARGS.get().unwrap().resizable)
        .application(app)
        .child(&scrolled_window)
        .default_height(ARGS.get().unwrap().content_height)
        .default_width(ARGS.get().unwrap().content_width)
        .titlebar(&titlebar)
        .build();

    let event_controller = EventControllerKey::new();
    event_controller.connect_key_pressed(|_, key, _, _| {
        if [gtk::gdk::Key::Escape, gtk::gdk::Key::q, gtk::gdk::Key::Q].contains(&key) {
            std::process::exit(0)
        }
        glib::signal::Inhibit(false)
    });

    window.add_controller(event_controller);
    window.present();

    if ARGS.get().unwrap().from_stdin {
        listen_to_stdin(&list_data.list_model);
    }
}

/// Listen to input from stdin.
/// Parses the input and checks if it is an existing file path.
/// Valid files will be added to the model.
fn listen_to_stdin(model: &ListStore) {
    let (sender, receiver) = MainContext::channel(Priority::default());
    gio::spawn_blocking(move || {
        let stdin = io::stdin();
        for path in stdin.lock().lines().flatten() {
            let file = gio::File::for_path(path);
            if file.query_exists(gio::Cancellable::NONE) {
                if let Err(err) = sender.send(file) {
                    println!("{}", err);
                }
            } else {
                println!("{} does not exist!", file.parse_name());
            }
            let _ = io::stdout().flush();
        }
    });
    // weak references don't work
    receiver.attach(
        None,
        clone!(@weak model => @default-return Continue(false), move |file| {
            model.append(&FileObject::new(&file));
            Continue(true)
        }),
    );
}
