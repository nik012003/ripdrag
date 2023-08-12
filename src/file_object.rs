use gio::FileQueryInfoFlags;
use glib::{GString, MainContext, Object, Priority, Properties};
use glib_macros::clone;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gdk, gdk_pixbuf, gio, glib};

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
            .unwrap_or_default()
            .content_type()
            .unwrap_or(GString::format(format_args!("text/plain")))
    }
}

impl FileObject {
    pub fn new(file: &gio::File) -> Self {
        let obj = Object::builder().property("file", file);
        let icon_name = gio::content_type_get_generic_icon_name(&file.mime_type());
        // use the default thumbnail
        let icon = gtk::Image::builder()
            .icon_name(icon_name.unwrap_or(glib::GString::format(format_args!("text/default"))))
            .pixel_size(ARGS.get().unwrap().icon_size)
            .build();
        let obj = obj.property("thumbnail", icon).build();
        let file = file.clone();

        // For every image a thumbnail of the image is sent. When it is not an image a None is sent.
        // There is no thumbnail for GIFs.
        let (sender, receiver) = MainContext::channel(Priority::default());
        gio::spawn_blocking(move || {
            let print_err = |err| eprintln!("{}", err);
            if !file.query_exists(gio::Cancellable::NONE) {
                let _ = sender.send(None).map_err(print_err);
            }

            let mime_type = file.mime_type();
            // this only works for images
            if !ARGS.get().unwrap().disable_thumbnails
                && gio::content_type_is_mime_type(&mime_type, "image/*")
            {
                let image = gdk_pixbuf::Pixbuf::from_file_at_scale(
                    file.path().unwrap(),
                    ARGS.get().unwrap().icon_size,
                    -1,
                    true,
                );
                if let Ok(image) = image {
                    let _ = sender
                        .send(Some((
                            image.read_pixel_bytes(),
                            image.colorspace(),
                            image.has_alpha(),
                            image.bits_per_sample(),
                            image.width(),
                            image.height(),
                            image.rowstride(),
                        )))
                        .map_err(print_err);
                } else {
                    eprintln!("{}", image.unwrap_err());
                    let _ = sender.send(None).map_err(print_err);
                }
            } else {
                let _ = sender.send(None).map_err(print_err);
            }
        });
        // Sets the thumbnail and closes the receiver channel regardless of what was sent.
        receiver.attach(
            None,
            clone!(@weak obj => @default-return
            glib::Continue(false), move |image| {
                if let Some(image) = image {
                    let obj: FileObject = obj;
                    let thumbnail = obj.thumbnail();
                    // (apply gdk_pixbuf::Pixbuf::from_bytes image)
                    let image = Pixbuf::from_bytes(&image.0, image.1, image.2, image.3, image.4, image.5, image.6);
                    thumbnail.set_from_paintable(Some(&gdk::Texture::for_pixbuf(&image)));
                }
                glib::Continue(false)
            }),
        );

        obj
    }
}

mod imp {
    use std::cell::RefCell;

    use super::*;

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
