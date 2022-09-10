use gtk::gdk::ContentProvider;
use std::fs;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Button, Orientation, Box, DragSource};
use clap::Parser;

#[derive(Parser)]
struct Args {
    #[clap(short, long, value_parser, default_value_t = false)]
    resizable: bool,

    #[clap(short = 'x', long, value_parser, default_value_t = false)]
    and_exit: bool,

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


fn build_ui(app: &Application) {
    let v_box = Box::builder()
    .orientation(Orientation::Vertical)
    .build();

    let args =Args::parse();
    for path in args.paths {
        assert!(path.exists(),"{0} : no such file or directory",path.display());
        let button = Button::with_label(path.display().to_string().as_str());
        let drag_source = DragSource::builder()
            .build();
        drag_source.connect_prepare(move |_,_,_| 
            Some(ContentProvider::for_bytes(
                "text/uri-list",
                &gtk::glib::Bytes::from_owned(format!("file://{0}", fs::canonicalize(&path).unwrap().display()))
            )));
        if args.and_exit {
            drag_source.connect_drag_end(|_,_,_| std::process::exit(0));
        }
        button.add_controller(&drag_source);
        v_box.append(&button);
    }

    let window = ApplicationWindow::builder()
        .title("ripdrag")
        .resizable(args.resizable)
        .application(app)
        .child(&v_box)
        .build();
    
    window.show();
}