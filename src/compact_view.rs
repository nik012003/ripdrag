use gtk::DragSource;
use gtk::Widget;
use gtk::{prelude::*, Label};

use crate::util::{drag_source_all, setup_file_model, ListWidget};

pub fn generate_compact_view() -> ListWidget {
    let file_model = setup_file_model();
    let create_string = |arg| format!("{} elements", arg);

    let obj = Label::builder()
        .label(create_string(file_model.n_items()))
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .halign(gtk::Align::Center)
        .tooltip_text(format!("Drag {}", create_string(file_model.n_items())))
        .build();
    file_model
        .bind_property("n-items", &obj, "label")
        .transform_to(|_, item_count: u32| Some(format!("{} elements", item_count)))
        .build();

    let drag_source = DragSource::new();
    drag_source_all(&drag_source, &file_model);

    obj.add_controller(drag_source);

    ListWidget {
        list_model: file_model,
        widget: obj.upcast::<Widget>(),
    }
}
