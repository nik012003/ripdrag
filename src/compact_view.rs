use glib::Object;
use gtk::gio::ListStore;
use gtk::glib;
use gtk::subclass::prelude::*;
use gtk::DragSource;
use gtk::Widget;
use gtk::{prelude::*, Label};

use crate::util::{drag_source_all, setup_file_model, ListWidget};

pub fn generate_compact_view() -> ListWidget {
    let file_model = setup_file_model();

    let drag_source = DragSource::new();
    drag_source_all(&drag_source, &file_model);

    let obj = CompactLabel::new(file_model);

    obj.add_controller(drag_source);

    ListWidget {
        list_model: obj.model(),
        widget: obj.upcast::<Widget>(),
    }
}

glib::wrapper! {
    pub struct CompactLabel(ObjectSubclass<imp::CompactLabel>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Orientable, gtk::Buildable, gtk::ConstraintTarget;
}

impl CompactLabel {
    pub fn new(model: ListStore) -> Self {
                    let create_string = |arg| format!("{} elements", arg);
        let obj: Self = Object::builder().property("model", model).build();
        let label = Label::builder()
                .label(create_string(obj.model().n_items()))
                .ellipsize(gtk::pango::EllipsizeMode::End)
                .halign(gtk::Align::Center)
                .tooltip_text(format!("Drag {}", create_string(obj.model().n_items())))
                .build();
            obj.set_label(label);

            obj.append(&obj.label());
            obj.model()
                .bind_property("n-items", &obj.label(), "label")
                .transform_to(|_, item_count: u32| Some(format!("{} elements", item_count)))
                .build();
        obj
    }
}

mod imp {
    use super::*;
    use gtk::{glib::Properties, Align};
    use std::cell::RefCell;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::CompactLabel)]
    pub struct CompactLabel {
        #[property(get, construct_only)]
        pub model: RefCell<ListStore>,
        #[property(get, set)]
        pub label: RefCell<Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CompactLabel {
        const NAME: &'static str = "RipDragCompactLabel";
        type Type = super::CompactLabel;
        type ParentType = gtk::Box;
    }
    
    #[glib_macros::derived_properties]
    impl ObjectImpl for CompactLabel {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.set_halign(Align::Center);
            obj.set_valign(Align::Center);
        }
    }
 
    impl WidgetImpl for CompactLabel {}

    impl BoxImpl for CompactLabel {}
}
