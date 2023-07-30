use glib::Object;
use glib::Properties;
use glib_macros::clone;
use gtk::gdk;
use gtk::gdk_pixbuf;
use gtk::gio;
use gtk::gio::FileInfo;
use gtk::gio::FileQueryInfoFlags;
use gtk::glib;
use gtk::glib::GString;
use gtk::glib::MainContext;
use gtk::glib::Priority;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use crate::ARGS;

glib::wrapper! {
    pub struct FileObject(ObjectSubclass<imp::FileObject>);
}

trait MimeType {
    fn mime_type(&self) -> GString;
}

impl MimeType for gio::File {
    fn mime_type(&self) -> GString {
        let file_type = self.query_info(
            gio::FILE_ATTRIBUTE_STANDARD_CONTENT_TYPE,
            FileQueryInfoFlags::NONE,
            gio::Cancellable::NONE,
        );

        file_type
            .unwrap_or(FileInfo::default())
            .content_type()
            .unwrap_or(GString::format(format_args!("text/plain")))
    }
}

impl FileObject {
    pub fn new(file: &gio::File) -> Self {
        let obj = Object::builder().property("file", file);
        let icon_name = gio::content_type_get_generic_icon_name(&file.mime_type());

        let image = gtk::Image::builder()
            .icon_name(icon_name.unwrap_or(glib::GString::format(format_args!("text/default"))))
            .pixel_size(ARGS.get().unwrap().icon_size)
            .build();
        let obj = obj.property("thumbnail", image).build();
        let file = file.clone();
        let (sender, receiver) = MainContext::channel(Priority::default());
        gio::spawn_blocking(move || {
            let mime_type = file.mime_type();
            if !ARGS.get().unwrap().disable_thumbnails
                && gio::content_type_is_mime_type(&mime_type, "image/*")
            {
                let image = gdk::Texture::from_file(&file).ok();
                sender.send(image).expect("Could not create thumbnail");
            }
        });
        receiver.attach(
            None,
            clone!(@weak obj => @default-return
            glib::Continue(false), move |image| {
                if let Some(image) = image {
                    let obj: FileObject = obj;
                    let thumbnail = obj.thumbnail();

                    thumbnail.set_from_paintable(Some(&image));
                }

                glib::Continue(false)
            }),
        );

        obj
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

    impl FileObject {
        fn get_thumbnail(&self) -> gtk::Image {
            let mime_type = self.file.borrow().mime_type();

            if !ARGS.get().unwrap().disable_thumbnails
                && gio::content_type_is_mime_type(&mime_type, "image/*")
            {
                gtk::Image::builder()
                    .file(self.file.borrow().parse_name())
                    .pixel_size(ARGS.get().unwrap().icon_size)
                    .build()
            } else {
                self.thumbnail.borrow().clone()
            }
        }
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
