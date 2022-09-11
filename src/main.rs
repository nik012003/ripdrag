use std::thread;
use std::io::{self, BufRead};
use std::path::PathBuf;
use clap::Parser;

use gtk::{prelude::*, Application, ApplicationWindow, Button, Orientation, CenterBox, ListBox, DragSource, EventControllerKey, Image, ScrolledWindow, PolicyType};
use gtk::glib::{self,clone, Continue, MainContext, PRIORITY_DEFAULT, Bytes};
use gtk::gdk::ContentProvider;

/// Drag and Drop files to and from the terminal
#[derive(Parser)]
#[clap(about)]
struct Cli {
    /// Make the window resizable
    #[clap(short, long, value_parser, default_value_t = false)]
    resizable: bool,
    
    /// Exit after first successful drag or drop 
    #[clap(short = 'x', long, value_parser, default_value_t = false)]
    and_exit: bool,

    /// Only display icons, no labels
    #[clap(short, long, value_parser, default_value_t = false)]
    icons_only: bool,

    /// Size of icons and thumbnails
    #[clap(short = 's', long, value_parser, default_value_t = 32)]
    thumb_size: i32,

    /// Width of the main window
    #[clap(short = 'w', long, value_parser, default_value_t = 360)]
    content_width: i32,

    /// Height of the main window
    #[clap(short = 'h', long, value_parser, default_value_t = 360)]
    content_height: i32,

    /// Accept paths from stdin
    #[clap(short = 'I', long, value_parser, default_value_t = false)]
    from_stdin: bool,

    /// Drag all the items together
    #[clap(short = 'a', long, value_parser, default_value_t = false)]
    all: bool,

    /// Show only the number of items and drag them together
    #[clap(short = 'A', long, value_parser, default_value_t = false)]
    all_compact: bool,

    /// Paths to the files you want to drag
    #[clap(parse(from_os_str))]
    paths: Vec<std::path::PathBuf>,
}

fn main() {
    let app = Application::builder()
        .application_id("ga.strin.ripdrag")
        .build();
    app.connect_activate(build_ui);
    app.run_with_args(&vec![""]); // we don't want gtk to parse the arguments. cleaner solutions are welcome
}

fn build_ui(app: &Application) {
    // Parse arguments and check if files exist
    let args =Cli::parse();
    for path in &args.paths {
        assert!(path.exists(),"{0} : no such file or directory",path.display());
    }

    // Create a scrollable list 
    let list_box = ListBox::new();
    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never) //  Disable horizontal scrolling
        .min_content_width(args.content_width)
        .min_content_height(args.content_height)
        .child(&list_box)
        .build();
    
    // Populate the list with the buttons, if there are any
    if args.paths.len() > 0{
        if args.all_compact{
            list_box.append(&generate_compact(args.paths.clone(), args.and_exit));
        }else {
            for button in generate_buttons_from_paths(args.paths.clone(), args.and_exit, args.icons_only, args.thumb_size, args.all){
                list_box.append(&button);
            }
        }
    }

    // Build the main window
    let window = ApplicationWindow::builder()
        .title("ripdrag")
        .resizable(args.resizable)
        .application(app)
        .child(&scrolled_window)
        .build();
    
    // Kill the app when Escape is pressed
    let event_controller = EventControllerKey::new();
    event_controller.connect_key_pressed(|_,key,_,_| {
        if key.name().unwrap() == "Escape"{
            std::process::exit(0)
        }
        return glib::signal::Inhibit(false);
    });
    window.add_controller(&event_controller);

    // Read from stdin and populate the list
    if args.from_stdin{
        let mut paths: Vec<PathBuf> = args.paths.clone();
        let (sender, receiver) = MainContext::channel(PRIORITY_DEFAULT);
        thread::spawn(move || {
            let stdin = io::stdin();
            let mut lines = stdin.lock().lines();
        
            while let Some(line) = lines.next() {
                let path = PathBuf::from(line.unwrap());
                if path.exists() {
                    println!("Adding: {}", path.display());
                    sender.send(path).expect("Error");
                }else{
                    println!("{} : no such file or directory", path.display())
                }
            }
            }
        );
        receiver.attach(
            None,
            clone!(@weak list_box => @default-return Continue(false),
                        move |path| {
                            if args.all_compact{
                                paths.push(path);
                                match list_box.first_child(){
                                    Some(child) => list_box.remove(&child),
                                    None => {}
                                };
                                list_box.append(&generate_compact(paths.clone(),args.and_exit));
                            } else {
                                let button = generate_buttons_from_paths(vec![path],args.and_exit, args.icons_only, args.thumb_size, args.all);
                                list_box.append(&button[0]);
                            }
                            Continue(true)
                        }
            )
        );
    }
    window.show();
}

fn generate_buttons_from_paths(paths: Vec<PathBuf>, and_exit: bool, icons_only: bool, thumb_size: i32, all: bool) -> Vec<Button>{
    let mut button_vec = Vec::new();
    let uri_list = generate_uri_list(&paths);

    for path in paths.into_iter(){
        // The CenterBox(button_box) contains the image and the optional label
        // The Button contains the CenterBox and can be dragged
        let button_box = CenterBox::builder()
            .orientation(Orientation::Horizontal)
            .build();
        
        match get_image_from_path(&path,thumb_size){
            Some(image) => {
                if icons_only{
                    button_box.set_center_widget(Some(&image));
                } else{
                    button_box.set_start_widget(Some(&image));
                }
            },
            None => {}
        };

        if !icons_only{
            button_box.set_center_widget(Some(&gtk::Label::builder()
                .label(path.display().to_string().as_str())
                .build()));
        }

        let button = Button::builder().child(&button_box).build();
        let drag_source = DragSource::builder().build();
        
        if all{
            let list = uri_list.clone();
            drag_source.connect_prepare(move |_,_,_| Some(ContentProvider::for_bytes("text/uri-list", &list)));
        } else {
            // TODO: find a better way to the uri
            let uri = gtk::glib::Bytes::from_owned(format!("file:// {0}", path.canonicalize().unwrap().display()));
            drag_source.connect_prepare(move |_,_,_| Some(ContentProvider::for_bytes("text/uri-list", &uri)));
        }

        if and_exit {
            drag_source.connect_drag_end(|_,_,_| std::process::exit(0));
        }
        
        // Open the path with the default app
        button.connect_clicked(move |_| {
            opener::open(&path).unwrap();{}
        });
        
        button.add_controller(&drag_source);
        button_vec.push(button);
    }
    return button_vec;
}

fn generate_compact(paths: Vec<PathBuf>, and_exit: bool) -> Button{
    // Here we want to generate a single draggable button, containg all the files
    let button = Button::builder()
        .label(&format!("{} elements", paths.len()))
        .build();
    let drag_source = DragSource::builder().build();
    
    drag_source.connect_prepare(move |_,_,_| 
        Some(ContentProvider::for_bytes("text/uri-list", &generate_uri_list(&paths)))
    );

    if and_exit {
        drag_source.connect_drag_end(|_,_,_| std::process::exit(0));
    }
    button.add_controller(&drag_source);
    return  button;
}

fn get_image_from_path(path: &std::path::PathBuf, thumb_size: i32) -> Option<Image> {
    let mime_type;
    if path.metadata().unwrap().is_dir(){
        mime_type = "inode/directory";
    } else {
        mime_type = match infer::get_from_path(&path) {
            Ok(option) => match option {
                Some(infer_type) => infer_type.mime_type(),
                None => "text/plain"
            },
            Err(_) => "text/plain"
        };
    }
    if mime_type.contains("image"){
        return Some(Image::builder()
            .file(&path.as_os_str().to_str().unwrap())
            .pixel_size(thumb_size)
            .build());
    }
    return match gtk::gio::content_type_get_generic_icon_name(mime_type) {
        Some(icon_name) => Some(Image::builder()
            .icon_name(&icon_name)
            .pixel_size(thumb_size)
            .build()),
        None => None
    };
}

fn generate_uri_list(paths: &Vec<PathBuf>) -> Bytes {
    return gtk::glib::Bytes::from_owned(
        paths.iter()
        .map(|path| -> String {format!("file:// {0}", path.canonicalize().unwrap().display())})
        .reduce(|accum, item| [accum,item].join("\n")).unwrap()
    );
}