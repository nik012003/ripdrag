use glib::Object;
use glib::Properties;
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

glib::wrapper! {
    pub struct FileObject(ObjectSubclass<imp::FileObject>);
}

impl FileObject {
    pub fn new(file: gio::File) -> Self {
        Object::builder().property("file", file).build()
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
        #[property(get, set)]
        thumbnail: RefCell<Option<gtk::Image>>,
    }

    impl Default for FileObject {
        fn default() -> Self {
            Self {
                file: RefCell::new(gio::File::for_path("/does-not-exist")),
                thumbnail: RefCell::new(None),
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
