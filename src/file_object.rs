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
                let image = gdk_pixbuf::Pixbuf::from_file_at_scale(
                    file.path().unwrap(),
                    ARGS.get().unwrap().icon_size,
                    -1,
                    true,
                )
                .ok();
                if let Some(image) = image {
                    sender
                        .send(Some((
                            image.read_pixel_bytes(),
                            image.colorspace(),
                            image.has_alpha(),
                            image.bits_per_sample(),
                            image.width(),
                            image.height(),
                            image.rowstride(),
                        )))
                        .expect("Could not create thumbnail");
                } else {
                    sender.send(None).expect("Could not send None");
                }
            } else {
                sender.send(None).expect("Could not send None");
            }
        });
        receiver.attach(
            None,
            clone!(@weak obj => @default-return
            glib::Continue(false), move |image| {
                if let Some(image) = image {
                    let obj: FileObject = obj;
                    let thumbnail = obj.thumbnail();
                    // omg
                    let image = gdk_pixbuf::Pixbuf::from_bytes(&image.0, image.1, image.2, image.3, image.4, image.5, image.6);
                    thumbnail.set_from_paintable(Some(&gdk::Texture::for_pixbuf(&image)));
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
