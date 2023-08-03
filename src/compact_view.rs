use glib::Object;
use gtk::gio::ListStore;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CssProvider, DragSource, Label, Widget};

use crate::util::{generate_file_model, setup_drag_source_all, setup_drop_target, ListWidget};

pub fn generate_compact_view() -> ListWidget {
    let file_model = generate_file_model();

    let drag_source = DragSource::new();
    setup_drag_source_all(&drag_source, &file_model);

    let obj = CompactLabel::new(file_model);
    let model = obj.model();
    let obj = obj.upcast::<Widget>();

    // styling
    let provider = CssProvider::new();
    provider.load_from_data(include_str!("style.css"));
    gtk::style_context_add_provider_for_display(
        &gtk::gdk::Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    obj.add_css_class("drag");
    obj.set_cursor_from_name(Some("grab"));

    obj.add_controller(drag_source);
    setup_drop_target(&model, &obj);

    ListWidget {
        list_model: model,
        widget: obj,
    }
}

glib::wrapper! {
    pub struct CompactLabel(ObjectSubclass<imp::CompactLabel>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Orientable, gtk::Buildable, gtk::ConstraintTarget;
}

// This is necessary to keep the model alive, otherwise it will be dropped.
impl CompactLabel {
    pub fn new(model: ListStore) -> Self {
        let create_string = |arg| format!("{} elements", arg);
        let obj: Self = Object::builder().property("model", model).build();
        let label = Label::builder()
            .label(create_string(obj.model().n_items()))
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .tooltip_text(format!("Drag {}", create_string(obj.model().n_items())))
            .vexpand(true)
            .hexpand(true)
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
    use std::cell::RefCell;

    use gtk::glib::Properties;
    use gtk::{Align, Orientation};

    use super::*;

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
            obj.set_halign(Align::Fill);
            obj.set_valign(Align::Fill);
            obj.set_hexpand(true);
            obj.set_vexpand(true);
            obj.set_orientation(Orientation::Vertical);
        }
    }

    impl WidgetImpl for CompactLabel {}

    impl BoxImpl for CompactLabel {}
}
