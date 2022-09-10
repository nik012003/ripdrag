use gtk::gdk::ContentProvider;
use gtk::glib;
use gtk::glib::{clone, Continue, MainContext, PRIORITY_DEFAULT};
use std::{fs, thread};
use std::io::{self, BufRead};
use std::path::PathBuf;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Button, Orientation, CenterBox, Box, DragSource};
use clap::Parser;

#[derive(Parser)]
struct Args {
    #[clap(short, long, value_parser, default_value_t = false)]
    resizable: bool,

    #[clap(short = 'x', long, value_parser, default_value_t = false)]
    and_exit: bool,

    #[clap(short, long, value_parser, default_value_t = false)]
    icons_only: bool,

    #[clap(short = 's', long, value_parser, default_value_t = 32)]
    thumb_size: i32,

    #[clap(short = 'I', long, value_parser, default_value_t = false)]
    from_stdin: bool,

    #[clap(parse(from_os_str))]
    paths: Vec<std::path::PathBuf>,
}

fn main() {
    let app = Application::builder()
        .application_id("ga.strin.ripdrag")
        .build();
    app.connect_activate(build_ui);
    app.run_with_args(&vec![""]); //we don't want gtk to parse the arguments. cleaner solutions are welcome
}

fn get_image_from_path(path: &std::path::PathBuf) -> Option<String> {
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
    return match gtk::gio::content_type_get_generic_icon_name(mime_type) {
        Some(icon_name) => Some(icon_name.to_string()),
        None => None
    };
}

fn generate_button_from_path(path: PathBuf, and_exit: bool, icons_only: bool, thumb_size: i32) -> Button{
    let button_box = CenterBox::builder()
        .orientation(Orientation::Horizontal)
        .build();
    
    match get_image_from_path(&path){
        Some(icon_name) => {
            let image = gtk::Image::builder()
                .icon_name(&icon_name)
                .pixel_size(thumb_size)
                .build();
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
    drag_source.connect_prepare(move |_,_,_| 
        Some(ContentProvider::for_bytes(
            "text/uri-list",
            &gtk::glib::Bytes::from_owned(format!("file://{0}", fs::canonicalize(&path).unwrap().display())) //TODO: find a better way to get the uri
        )));
    if and_exit {
        drag_source.connect_drag_end(|_,_,_| std::process::exit(0));
    }
    button.add_controller(&drag_source);
    return button;
}

fn build_ui(app: &Application) {
    let v_box = Box::builder()
    .orientation(Orientation::Vertical)
    .build();

    let args =Args::parse();
    for path in args.paths {
        assert!(path.exists(),"{0} : no such file or directory",path.display());
        let button = generate_button_from_path(path,args.and_exit, args.icons_only, args.thumb_size);
        v_box.append(&button);
    }

    let window = ApplicationWindow::builder()
        .title("ripdrag")
        .resizable(args.resizable)
        .application(app)
        .child(&v_box)
        .build();
    

    if args.from_stdin{
        let (sender, receiver) = MainContext::channel(PRIORITY_DEFAULT);
        thread::spawn(move || {
            let stdin = io::stdin();
            let mut lines = stdin.lock().lines();
        
            while let Some(line) = lines.next() {
                let path = PathBuf::from(line.unwrap());
                if path.exists() {
                    println!("Adding: {0}", path.display());
                    sender.send(path).expect("Error");
                }else{
                    println!("{0} : no such file or directory", path.display())
                }
            }
            }
        );
        receiver.attach(
            None,
            clone!(@weak v_box => @default-return Continue(false),
                        move |path| {
                            let button = generate_button_from_path(path,args.and_exit, args.icons_only, args.thumb_size);
                            v_box.append(&button);
                            Continue(true)
                        }
            )
        );
    }
    window.show();
}