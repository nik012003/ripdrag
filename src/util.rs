use gtk::gdk::{ContentProvider, DragAction, FileList};
use gtk::gio::{self, File, ListStore};
use gtk::glib::{clone, Bytes};
use gtk::prelude::*;
use gtk::{gdk, glib, DragSource, DropTarget, EventSequenceState, Widget};

use crate::file_object::FileObject;
use crate::ARGS;

/// Helper record type.
pub struct ListWidget {
    pub list_model: ListStore,
    pub widget: Widget,
}

pub fn generate_file_model() -> ListStore {
    let file_model = ListStore::builder()
        .item_type(FileObject::static_type())
        .build();

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
    paths: impl IntoIterator<Item = &'a String>,
) -> Option<ContentProvider> {
    let mut uri_list = paths
        .into_iter()
        .cloned()
        .collect::<Vec<String>>()
        .join("\r\n");

    if uri_list.is_empty() {
        return None;
    } else {
        uri_list += "\r\n";
        let bytes = Bytes::from(uri_list.as_bytes());
        Some(ContentProvider::for_bytes("text/uri-list", &bytes))
    }
}
/// For the -a or -A flag.
pub fn setup_drag_source_all(drag_source: &DragSource, list_model: &ListStore) {
    drag_source.connect_prepare(clone!(
        #[weak]
        list_model,
        #[upgrade_or_default]
        move |me, _, _| {
            me.set_state(EventSequenceState::Claimed);
            let files: Vec<String> = list_model
                .into_iter()
                .flatten()
                .map(|file_object| {
                    file_object
                        .downcast::<FileObject>()
                        .unwrap()
                        .file()
                        .uri()
                        .to_string()
                })
                .collect();
            generate_content_provider(&files)
        }
    ));
}

fn create_tmp_file(file: &File) -> Option<FileObject> {
    let print_err = |err| eprintln!("{}", err);
    if file.path().is_some() {
        Some(FileObject::new(file))
    } else {
        let info = file.query_info(
            gio::FILE_ATTRIBUTE_STANDARD_DISPLAY_NAME,
            gio::FileQueryInfoFlags::NONE,
            gio::Cancellable::NONE,
        );
        if let Err(err) = info {
            print_err(err);
            return None;
        }
        let tmp_file = gio::File::new_tmp(None::<String>);
        match tmp_file {
            Ok(val) => {
                let (tmp_file, stream) = val;
                // download the file
                let bytes = file.load_bytes(gio::Cancellable::NONE);
                if bytes.is_err() {
                    return None;
                }
                // write it
                let _ = stream
                    .output_stream()
                    .write_bytes(&bytes.unwrap().0, gio::Cancellable::NONE)
                    .map_err(|err| println!("{}", err));

                // rename it
                // unwrapping basename is safe because the file exists
                let rename_result = tmp_file.set_display_name(
                    &format!(
                        "{}{}",
                        tmp_file.basename().unwrap().display(),
                        info.unwrap().display_name()
                    ),
                    gio::Cancellable::NONE,
                );

                if let Err(err) = rename_result {
                    print_err(err);
                    Some(FileObject::new(&tmp_file))
                } else {
                    Some(FileObject::new(&rename_result.unwrap()))
                }
            }
            Err(err) => {
                println!("{}", err);
                None
            }
        }
    }
}

/// TODO: This will not work for directories <https://gitlab.gnome.org/GNOME/gtk/-/issues/5348>.
/// Will add dropped files to the model if keep is set.
pub fn setup_drop_target(model: &ListStore, widget: &Widget) {
    let drop_target = DropTarget::builder()
        .name("file-drop-target")
        .actions(DragAction::COPY)
        .build();
    drop_target.set_types(&[FileList::static_type(), glib::types::Type::STRING]);

    drop_target.connect_drop(clone!(
        #[weak]
        model,
        #[upgrade_or]
        false,
        move |_, value, _, _| {
            let mut files_vec: Vec<File> = vec![];

            if let Ok(file_uris) = value.get::<&str>() {
                files_vec = file_uris
                    .split('\n')
                    .collect::<Vec<&str>>()
                    .iter()
                    .filter_map(|uri| glib::Uri::parse(uri, glib::UriFlags::PARSE_RELAXED).ok())
                    .map(|uri| File::for_uri(uri.to_str().as_str()))
                    .collect();
            } else if let Ok(files) = value.get::<gdk::FileList>() {
                files_vec = files.files();
            }

            if files_vec.is_empty() {
                return false;
            }

            for item in &files_vec {
                println!("{}", item.parse_name());
            }

            if ARGS.get().unwrap().keep {
                let file_objs: Vec<FileObject> = files_vec
                    .iter()
                    .filter_map(|item| create_tmp_file(item))
                    .collect();
                model.extend_from_slice(&file_objs);
            } else if ARGS.get().unwrap().and_exit {
                std::process::exit(0);
            }

            true
        }
    ));

    widget.add_controller(drop_target);
}

pub fn drag_source_and_exit(drag_source: &DragSource) {
    drag_source.connect_drag_end(|_, _, _| {
        std::process::exit(0);
    });
}
