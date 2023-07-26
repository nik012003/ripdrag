use std::io::{self, BufRead};
use std::path::PathBuf;
use std::thread;

use clap::Parser;
use gtk::gdk::ffi::gdk_content_provider_new_typed;
use gtk::gio::ffi::g_file_get_type;
use url::Url;

use gtk::gdk::{ContentProvider, DragAction};
use gtk::gio::{ApplicationFlags, File};
use gtk::glib::{self, clone, set_program_name, Bytes, Continue, MainContext, PRIORITY_DEFAULT};
use gtk::{
    prelude::*, Application, ApplicationWindow, Button, CenterBox, DragSource, DropTarget,
    EventControllerKey, Image, ListBox, Orientation, PolicyType, ScrolledWindow,
};

#[derive(Parser, Clone)]
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

    /// With --target, keep files to drag out
    #[arg(short, long, requires = "target")]
    print_path: bool,

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

    /// Drag all the items together
    #[arg(short = 'a', long)]
    all: bool,

    /// Show only the number of items and drag them together
    #[arg(short = 'A', long)]
    all_compact: bool,

    /// Paths to the files you want to drag
    #[arg(value_name = "PATH")]
    paths: Vec<PathBuf>,
}

fn main() {
    let args = Cli::parse();
    set_program_name(Some("ripdrag"));
    let app = Application::builder()
        .application_id("ga.strin.ripdrag")
        .flags(ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_activate(move |app| build_ui(app, args.clone()));
    app.run_with_args(&[""]); // we don't want gtk to parse the arguments. cleaner solutions are welcome
}

fn build_ui(app: &Application, args: Cli) {
    // Parse arguments and check if files exist
    for path in &args.paths {
        assert!(
            path.exists(),
            "{0} : no such file or directory",
            path.display()
        );
    }
    // Create a scrollable list
    let list_box = ListBox::new();
    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never) //  Disable horizontal scrolling
        .min_content_width(args.content_width)
        .child(&list_box)
        .build();

    // Build the main window
    let window = ApplicationWindow::builder()
        .title("ripdrag")
        .resizable(args.resizable)
        .application(app)
        .child(&scrolled_window)
        .default_height(args.content_height)
        .build();

    if args.target {
        build_target_ui(list_box, args);
    } else {
        build_source_ui(list_box, args);
    }

    // Kill the app when Escape is pressed
    let event_controller = EventControllerKey::new();
    event_controller.connect_key_pressed(|_, key, _, _| {
        if key.name().unwrap() == "Escape" {
            std::process::exit(0)
        }
        glib::signal::Inhibit(false)
    });

    window.add_controller(event_controller);
    window.show();
}

fn build_source_ui(list_box: ListBox, args: Cli) {
    // Populate the list with the buttons, if there are any
    if !args.paths.is_empty() {
        if args.all_compact {
            list_box.append(&generate_compact(args.paths.clone(), args.and_exit));
        } else {
            for button in generate_buttons_from_paths(
                args.paths.clone(),
                args.and_exit,
                args.icons_only,
                args.disable_thumbnails,
                args.icon_size,
                args.all,
            ) {
                list_box.append(&button);
            }
        }
    }

    // Read from stdin and populate the list
    if args.from_stdin {
        let mut paths: Vec<PathBuf> = args.paths.clone();
        let (sender, receiver) = MainContext::channel(PRIORITY_DEFAULT);
        thread::spawn(move || {
            let stdin = io::stdin();
            let lines = stdin.lock().lines();

            for line in lines {
                let path = PathBuf::from(line.unwrap());
                if path.exists() {
                    println!("Adding: {}", path.display());
                    sender.send(path).expect("Error");
                } else if args.verbose {
                    println!("{} : no such file or directory", path.display())
                }
            }
        });
        receiver.attach(
                None,
                clone!(@weak list_box => @default-return Continue(false),
                            move |path| {
                                if args.all_compact{
                                    paths.push(path);
                                    if let Some(child) = list_box.first_child() { list_box.remove(&child) }
                                    list_box.append(&generate_compact(paths.clone(),args.and_exit));
                                } else {
                                    let button = generate_buttons_from_paths(vec![path],args.and_exit, args.icons_only, args.disable_thumbnails, args.icon_size, args.all);
                                    list_box.append(&button[0]);
                                }
                                Continue(true)
                            }
                )
            );
    }
}

fn build_target_ui(list_box: ListBox, args: Cli) {
    // Generate the Drop Target and button
    let button = Button::builder().label("Drop your files here").build();

    let drop_target = DropTarget::new(File::static_type(), DragAction::COPY);

    let (sender, receiver) = MainContext::channel(PRIORITY_DEFAULT);

    drop_target.connect_drop(move |_, value, _, _| {
        if let Ok(file) = value.get::<File>() {
            if let Some(path) = file.path() {
                println!("{}", path.canonicalize().unwrap().to_string_lossy());
                if args.keep {
                    sender
                        .send(path)
                        .expect("Error while sending paths to the receiver");
                }
                return true;
            }
        }
        false
    });

    // get the uri_list from the drop and populate the list of files (--keep)
    let mut paths: Vec<PathBuf> = Vec::new();
    receiver.attach(
        None,
        clone!(@weak list_box => @default-return Continue(false),
                move |path| {
                    let mut new_paths :Vec<PathBuf> = Vec::new();
                    new_paths.push(path);
                    if args.all_compact{
                        // Hacky solution, check if we already created buttons
                        if let Some(child) = list_box.last_child(){
                            list_box.remove(&child);
                        }
                        paths.append(&mut new_paths);
                        list_box.append(&generate_compact(paths.clone(),args.and_exit));
                    } else {
                        // This solution is fast, but it's gonna cause problems when --all is used in combinatio with --target
                        for button in &generate_buttons_from_paths(new_paths, args.and_exit, args.icons_only, args.disable_thumbnails, args.icon_size, args.all){
                            list_box.append(button);
                        };
                    }
                    Continue(true)
                }
        )
    );
    button.add_controller(drop_target);
    list_box.append(&button);
}

fn generate_buttons_from_paths(
    paths: Vec<PathBuf>,
    and_exit: bool,
    icons_only: bool,
    disable_thumbnails: bool,
    icon_size: i32,
    all: bool,
) -> Vec<Button> {
    let mut button_vec = Vec::new();
    let uri_list = generate_uri_list(&paths);

    //TODO: make this loop multithreaded
    for path in paths.into_iter() {
        // The CenterBox(button_box) contains the image and the optional label
        // The Button contains the CenterBox and can be dragged
        let button_box = CenterBox::builder()
            .orientation(Orientation::Horizontal)
            .build();

        if let Some(image) = get_image_from_path(&path, icon_size, disable_thumbnails) {
            match icons_only {
                true => button_box.set_center_widget(Some(&image)),
                false => button_box.set_start_widget(Some(&image)),
            }
        }

        if !icons_only {
            button_box.set_center_widget(Some(
                &gtk::Label::builder()
                    .label(path.display().to_string().as_str())
                    .build(),
            ));
        }

        let button = Button::builder().child(&button_box).build();
        let drag_source = DragSource::new();

        if all {
            let list = uri_list.clone();
            drag_source.connect_prepare(move |_, _, _| {
                Some(ContentProvider::for_bytes("text/uri-list", &list))
            });
        } else {
            let p = path.clone();
            drag_source
                .connect_prepare(move |_, _, _| Some(generate_content_provider_from_path(&p)));
        }

        if and_exit {
            drag_source.connect_drag_end(|_, _, _| std::process::exit(0));
        }

        // Open the path with the default app
        button.connect_clicked(move |_| {
            opener::open(&path).unwrap();
        });

        button.add_controller(drag_source);
        button_vec.push(button);
    }
    button_vec
}

fn generate_compact(paths: Vec<PathBuf>, and_exit: bool) -> Button {
    // Here we want to generate a single draggable button, containg all the files
    let button = Button::builder()
        .label(format!("{} elements", paths.len()))
        .build();
    let drag_source = DragSource::new();

    drag_source.connect_prepare(move |_, _, _| {
        Some(ContentProvider::for_bytes(
            "text/uri-list",
            &generate_uri_list(&paths),
        ))
    });

    if and_exit {
        drag_source.connect_drag_end(|_, _, _| std::process::exit(0));
    }
    button.add_controller(drag_source);
    button
}

fn get_image_from_path(path: &PathBuf, icon_size: i32, disable_thumbnails: bool) -> Option<Image> {
    let mime_type = match path.metadata().unwrap().is_dir() {
        true => "inode/directory",
        false => match infer::get_from_path(path) {
            Ok(option) => match option {
                Some(infer_type) => infer_type.mime_type(),
                None => "text/plain",
            },
            Err(_) => "text/plain",
        },
    };
    if mime_type.contains("image") & !disable_thumbnails {
        return Some(
            Image::builder()
                .file(path.as_os_str().to_str().unwrap())
                .pixel_size(icon_size)
                .build(),
        );
    }
    gtk::gio::content_type_get_generic_icon_name(mime_type).map(|icon_name| {
        Image::builder()
            .icon_name(icon_name)
            .pixel_size(icon_size)
            .build()
    })
}

fn generate_content_provider_from_path(path: &PathBuf) -> ContentProvider {
    unsafe {
        let gfile = File::for_path(path);
        glib::translate::from_glib_full(gdk_content_provider_new_typed(g_file_get_type(), gfile))
    }
}

fn generate_uri_list(paths: &[PathBuf]) -> Bytes {
    return gtk::glib::Bytes::from_owned(
        paths
            .iter()
            .map(|path| -> String {
                Url::from_file_path(path.canonicalize().unwrap())
                    .unwrap()
                    .to_string()
            })
            .reduce(|accum, item| [accum, item].join("\n"))
            .unwrap(),
    );
}
