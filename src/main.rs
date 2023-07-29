use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::thread;

use clap::Parser;
use file_object::FileObject;
use gtk::gdk::ffi::gdk_content_provider_new_typed;
use gtk::gio::ffi::g_file_get_type;
use url::Url;

use gtk::gdk::{ContentProvider, DragAction};
use gtk::gio::{ApplicationFlags, File, ListModel, ListStore};
use gtk::glib::{
    self, clone, set_program_name, Bytes, Continue, MainContext, Priority, PRIORITY_DEFAULT,
};
use gtk::{
    prelude::*, Application, ApplicationWindow, Button, CenterBox, DragSource, DropTarget,
    EventControllerKey, Image, Label, ListBox, ListItem, ListItemFactory, ListView, MultiSelection,
    Orientation, PolicyType, ScrolledWindow, SelectionModel, SignalListItemFactory,
};

mod file_object;

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

use gtk::gio;

static CURRENT_DIRECTORY: OnceLock<gio::File> = OnceLock::new();
static ARGS: OnceLock<Cli> = OnceLock::new();

fn main() {
    CURRENT_DIRECTORY
        .set(gio::File::for_path("."))
        .expect("Could not set CURRENT_DIRECTORY");
    let args = Cli::parse();
    ARGS.set(args).expect("Could not set ARGS");
    set_program_name(Some("ripdrag"));
    let app = Application::builder()
        .application_id("ga.strin.ripdrag")
        .flags(ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_activate(move |app| build_ui(app, ARGS.get().unwrap()));
    app.run_with_args(&[""]); // we don't want gtk to parse the arguments. cleaner solutions are welcome
}

fn build_ui(app: &Application, args: &Cli) {
    // Parse arguments and check if files exist
    for path in &args.paths {
        assert!(
            path.exists(),
            "{0} : no such file or directory",
            path.display()
        );
    }
    // Create a scrollable list
    let mut list_data = build_list_data(args);
    setup_factory(&mut list_data.1, args);

    let list_view = ListView::new(Some(list_data.0), Some(list_data.1));

    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never) //  Disable horizontal scrolling
        .min_content_width(args.content_width)
        .child(&list_view)
        .build();

    // Build the main window
    let window = ApplicationWindow::builder()
        .title("ripdrag")
        .resizable(args.resizable)
        .application(app)
        .child(&scrolled_window)
        .default_height(args.content_height)
        .build();

    // Kill the app when Escape is pressed
    let event_controller = EventControllerKey::new();
    event_controller.connect_key_pressed(|_, key, _, _| {
        if key == gtk::gdk::Key::Escape {
            std::process::exit(0)
        }
        glib::signal::Inhibit(false)
    });

    window.add_controller(event_controller);
    window.set_visible(true);

    // lol
    if args.from_stdin {
        listen_to_stdin(
            &list_view
                .model()
                .unwrap()
                .downcast::<MultiSelection>()
                .unwrap()
                .model()
                .unwrap(),
        );
    }
}

fn listen_to_stdin(model: &ListModel) {
    let (sender, receiver) = MainContext::channel(Priority::default());
    thread::spawn(move || {
        let stdin = io::stdin();
        for path in stdin.lock().lines().flatten() {
            let file = gio::File::for_path(path);
            if file.query_exists(gio::Cancellable::NONE) {
                if let Err(err) = sender.send(file) {
                    println!("{}", err);
                }
            } else {
                println!("{} does not exist!", file.parse_name());
                let _ = io::stdout().flush();
            }
        }
    });
    receiver.attach(
        None,
        clone!(@weak model => @default-return Continue(false), move |file| {
            model.downcast::<ListStore>().unwrap().append(&FileObject::new(&file));
            Continue(true)
        }),
    );
}

fn build_list_data(args: &Cli) -> (MultiSelection, SignalListItemFactory) {
    let file_model = ListStore::new(FileObject::static_type());
    // setup the file_model
    if !args.paths.is_empty() && !args.all_compact {
        let files: Vec<FileObject> = args
            .paths
            .iter()
            .map(|path| FileObject::new(&gtk::gio::File::for_path(path)))
            .collect();
        file_model.extend_from_slice(&files);
    }

    // setup factory
    let factory = SignalListItemFactory::new();
    (MultiSelection::new(Some(file_model)), factory)
}

fn setup_factory(factory: &mut SignalListItemFactory, args: &Cli) {
    factory.connect_setup(|_, list_item| {
        let row = CenterBox::default();
        list_item
            .downcast_ref::<ListItem>()
            .expect("Needs to be ListItem")
            .set_child(Some(&row));
    });

    factory.connect_bind(|_, list_item| {
        let file_object = list_item
            .downcast_ref::<ListItem>()
            .expect("Needs to be ListItem")
            .item()
            .and_downcast::<FileObject>()
            .expect("The item has to be an `FileObject`.");

        let file_row = list_item
            .downcast_ref::<ListItem>()
            .expect("Needs to be ListItem")
            .child()
            .and_downcast::<CenterBox>()
            .expect("The child has to be a `Label`.");

        let str = if file_object
            .file()
            .has_parent(Some(CURRENT_DIRECTORY.get().unwrap()))
        {
            CURRENT_DIRECTORY
                .get()
                .unwrap()
                .relative_path(&file_object.file())
                .expect("Can't make a relative path")
                .to_str()
                .expect("Couldn't read file name")
                .to_string()
        } else {
            file_object.file().parse_name().to_string()
        };

        let label = Label::builder()
            .label(&str)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .tooltip_text(&str)
            .build();
        file_row.set_center_widget(Some(&label));
        file_row.set_start_widget(Some(&file_object.thumbnail()))
    });
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
