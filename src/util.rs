use std::collections::HashSet;
use std::path::PathBuf;

use gtk::gio::ListModel;
use gtk::{gdk, gio, glib, DropTarget};
use gtk::{
    gdk::ContentProvider, gio::ListStore, glib::clone, prelude::*, CenterBox, DragSource, Label,
    MultiSelection, Widget,
};
use url::Url;

use crate::{file_object::FileObject, ARGS};

pub struct ListWidget {
    pub list_model: ListStore,
    pub widget: Widget,
}

pub fn setup_file_model() -> ListStore {
    let file_model = ListStore::new(FileObject::static_type());
    let files: Vec<FileObject> = ARGS
        .get()
        .unwrap()
        .paths
        .iter()
        .map(|path| FileObject::new(&gtk::gio::File::for_path(path)))
        .collect();
    file_model.extend_from_slice(&files);
    file_model
}

pub fn generate_content_provider<'a>(
    paths: impl IntoIterator<Item = &'a PathBuf>,
) -> Option<ContentProvider> {
    let bytes = &gtk::glib::Bytes::from_owned(
        paths
            .into_iter()
            .map(|path| -> String {
                Url::from_file_path(path.canonicalize().unwrap())
                    .unwrap()
                    .to_string()
            })
            .fold("".to_string(), |accum, item| [accum, item].join("\n")),
    );
    if bytes.is_empty() {
        None
    } else {
        Some(ContentProvider::for_bytes("text/uri-list", &bytes))
    }
}

pub fn drag_source_all(drag_source: &DragSource, list_model: &ListStore) {
    drag_source.connect_prepare(clone!(@weak list_model => @default-return None, move |me, _, _| {
            me.set_state(gtk::EventSequenceState::Claimed);
            let files: Vec<PathBuf> = list_model.into_iter().flatten().map(|file_object| {file_object.downcast::<FileObject>().unwrap().file().path().unwrap()}).collect();
            generate_content_provider(&files)
        }));
}

pub fn setup_drop_target(model: &ListStore, widget: &Widget) {
    let drop_target = DropTarget::new(gio::File::static_type(), gdk::DragAction::COPY);
    drop_target.connect_drop(
        clone!(@weak model => @default-return false, move |_, value, _, _|
            {
                if let Ok(file) = value.get::<gio::File>() {
                    let file = FileObject::new(&file);
                    if ARGS.get().unwrap().keep {
                        model.append(&file);
                    }
                    println!("{}", file.file().parse_name());
                    true
                } else {
                    false
                }

        }),
    );

    widget.add_controller(drop_target);
}
