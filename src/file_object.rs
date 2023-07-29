use glib::Object;
use glib::Properties;
use gtk::gio;
use gtk::gio::FileInfo;
use gtk::gio::FileQueryInfoFlags;
use gtk::glib;
use gtk::glib::GString;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use crate::ARGS;

glib::wrapper! {
    pub struct FileObject(ObjectSubclass<imp::FileObject>);
}

impl FileObject {
    pub fn new(file: &gio::File) -> Self {
        let obj = Object::builder().property("file", file);
        let file_type = file.query_info(
            gio::FILE_ATTRIBUTE_STANDARD_CONTENT_TYPE,
            FileQueryInfoFlags::NONE,
            gio::Cancellable::NONE,
        );
        let icon_name = gio::content_type_get_generic_icon_name(
            &file_type
                .unwrap_or(FileInfo::default())
                .content_type()
                .unwrap_or(GString::format(format_args!("text/plain"))),
        );

        let image = gtk::Image::builder()
            .icon_name(icon_name.unwrap_or(glib::GString::format(format_args!("text/default"))))
            .pixel_size(ARGS.get().unwrap().icon_size)
            .build();
        obj.property("thumbnail", image).build()
    }
}

mod imp {
    use super::*;
    use std::cell::RefCell;

    #[derive(Properties)]
    #[properties(wrapper_type = super::FileObject)]
    pub struct FileObject {
        #[property(get, construct_only)]
        file: RefCell<gio::File>,
        #[property(get, construct_only)]
        thumbnail: RefCell<gtk::Image>,
    }

    impl Default for FileObject {
        fn default() -> Self {
            Self {
                file: RefCell::new(gio::File::for_path("/does-not-exist")),
                thumbnail: RefCell::new(gtk::Image::default()),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FileObject {
        const NAME: &'static str = "RipDragFileObject";
        type Type = super::FileObject;
        type ParentType = glib::Object;
    }

    // Trait shared by all GObjects
    #[glib_macros::derived_properties]
    impl ObjectImpl for FileObject {}
}
