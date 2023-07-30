use std::{collections::HashSet, path::PathBuf};


use glib::*;
use glib_macros::clone;
use gtk::{
    gdk,
    gio::{self, ListStore},
    prelude::*,
    CenterBox, DragSource, Label, ListItem, ListView, MultiSelection, SignalListItemFactory,
};
use url::Url;
use crate::{file_object::FileObject, ARGS, CURRENT_DIRECTORY};
use gtk::gdk::*;

pub fn generate_list_view() -> ListView {
    let mut list_data = build_list_data();
    setup_factory(&mut list_data.1, &list_data.0);
    ListView::new(Some(list_data.0), Some(list_data.1))
}


fn build_list_data() -> (MultiSelection, SignalListItemFactory) {
    let file_model = ListStore::new(FileObject::static_type());
    // setup the file_model
    if !ARGS.get().unwrap().paths.is_empty() && !ARGS.get().unwrap().all_compact {
        let files: Vec<FileObject> = ARGS.get().unwrap()
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

fn create_drag_source(row: &CenterBox, selection: &MultiSelection) -> DragSource {
    let drag_source = DragSource::new();
    if ARGS.get().unwrap().all {
        drag_source.connect_prepare(clone!(@weak selection => @default-return None, move |me, _, _| {
            me.set_state(gtk::EventSequenceState::Claimed);
            let model = selection.model().unwrap().downcast::<ListStore>().unwrap();
            let files: Vec<PathBuf> = model.into_iter().flatten().map(|file_object| {file_object.downcast::<FileObject>().unwrap().file().path().unwrap()}).collect();
            Some(generate_content_provider(&files))
        }));
    } else {
        drag_source.connect_prepare(clone!(@weak row, @weak selection, => @default-return None, move |me, _, _| {
            me.set_state(gtk::EventSequenceState::Claimed);
            let selected = selection.selection();
            let mut set : HashSet<PathBuf> = HashSet::with_capacity(selected.size() as usize);
            for index in 0..selected.size() {
                set.insert(selection.item(selected.nth(index as u32)).unwrap().downcast::<FileObject>().unwrap().file().path().unwrap());
            }
        
            let row_file = get_file(&row).path().unwrap();
            if !set.contains(&row_file)
            {
                selection.unselect_all();
                Some(generate_content_provider(&[row_file]))
            } else {
                Some(
                    generate_content_provider(&set))
            }
        }));
    }
    drag_source
}

fn generate_content_provider<'a>(paths: impl IntoIterator<Item = &'a PathBuf>) -> ContentProvider {
ContentProvider::for_bytes("text/uri-list",     &gtk::glib::Bytes::from_owned(
        paths
            .into_iter()
            .map(|path| -> String {
                Url::from_file_path(path.canonicalize().unwrap())
                    .unwrap()
                    .to_string()
            })
            .reduce(|accum, item| [accum, item].join("\n"))
            .unwrap(),
    ))
}

fn create_gesture_click(row: &CenterBox) -> gtk::GestureClick {
    let click = gtk::GestureClick::new();
    click.connect_released(clone!(@weak row => move |me, _, _, _|{
        if me.current_event_state().contains(gdk::ModifierType::CONTROL_MASK) {
            return;
        }
        let file = get_file(&row);
        if let Some(file) =  file.path() {
           let _ = opener::open(file).map_err(|err| {
                eprint!("{}", err);
                err
           });
        }
    }));

    click
}

fn get_file(row: &CenterBox) -> gio::File {
    let file_widget = if ARGS.get().unwrap().icons_only {
        row.start_widget().unwrap()
    } else {
        row.center_widget().unwrap()
    };
    gio::File::for_path(file_widget.downcast::<Label>().unwrap().text())
}

fn setup_factory(factory: &mut SignalListItemFactory, list: &MultiSelection) {
    factory.connect_setup(clone!(@weak list => move |_, list_item| {
        let row = CenterBox::default();
        let drag_source = create_drag_source(&row, &list);
        let gesture_click = create_gesture_click(&row);
        row.add_controller(gesture_click);
        row.add_controller(drag_source);
        list_item
            .downcast_ref::<ListItem>()
            .expect("Needs to be ListItem")
            .set_child(Some(&row));
    }));

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

        // show either relative or absolute path
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
            .tooltip_text(&str);

        if ARGS.get().unwrap().icons_only {
            file_row.set_start_widget(Some(&label.visible(false).build()));
            file_row.set_center_widget(Some(&file_object.thumbnail()))
        } else {
            file_row.set_center_widget(Some(&label.build()));
            file_row.set_start_widget(Some(&file_object.thumbnail()))
        }
    });
}
