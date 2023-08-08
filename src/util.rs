use std::path::PathBuf;

use gtk::gdk::{ContentFormats, ContentProvider, DragAction, FileList};
use gtk::gio::{File, ListStore};
use gtk::glib::{clone, Bytes};
use gtk::prelude::*;
use gtk::{gdk, glib, DragSource, DropTarget, EventSequenceState, Widget};
use url::Url;

use crate::file_object::FileObject;
use crate::ARGS;

/// Helper record type.
pub struct ListWidget {
    pub list_model: ListStore,
    pub widget: Widget,
}

pub fn generate_file_model() -> ListStore {
    let file_model = ListStore::new(FileObject::static_type());
    let files: Vec<FileObject> = ARGS
        .get()
        .unwrap()
        .paths
        .iter()
        .map(|path| FileObject::new(&File::for_path(path)))
        .collect();
    file_model.extend_from_slice(&files);
    file_model
}

/// Returns data for dragging files.
pub fn generate_content_provider<'a>(
    paths: impl IntoIterator<Item = &'a PathBuf>,
) -> Option<ContentProvider> {
    let bytes = &Bytes::from_owned(
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
        Some(ContentProvider::for_bytes("text/uri-list", bytes))
    }
}

/// For the -a or -A flag.
pub fn setup_drag_source_all(drag_source: &DragSource, list_model: &ListStore) {
    drag_source.connect_prepare(
        clone!(@weak list_model => @default-return None, move |me, _, _| {
            me.set_state(EventSequenceState::Claimed);
            let files: Vec<PathBuf> = list_model.into_iter().flatten().map(|file_object| {
                file_object.downcast::<FileObject>().unwrap().file().path().unwrap()}).collect();
            generate_content_provider(&files)
        }),
    );
}

/// TODO: This will not work for directories <https://gitlab.gnome.org/GNOME/gtk/-/issues/5348>.
/// Will add dropped files to the model if keep is set.
pub fn setup_drop_target(model: &ListStore, widget: &Widget) {
    let drop_target = DropTarget::builder()
        .name("file-drop-target")
        .actions(DragAction::COPY)
        .formats(&ContentFormats::for_type(FileList::static_type()))
        .build();

    drop_target.connect_drop(
        clone!(@weak model => @default-return false, move |_, value, _, _|
            {
                if let Ok(files) = value.get::<gdk::FileList>() {
                    let files = files.files();
                    if files.is_empty() {
                        return false;
                    }
                    let vec: Vec<FileObject> = files.iter().map(|item| {println!("{}", item.parse_name()); FileObject::new(item)}).collect();
                    if ARGS.get().unwrap().keep {
                        model.extend_from_slice(&vec);
                    }
                    return true
                } 
                false
        }),
    );

    widget.add_controller(drop_target);
}

pub fn drag_source_and_exit(drag_source: &DragSource) {
    drag_source.connect_drag_end(|_, _, _|{
        std::process::exit(0);
    });
}