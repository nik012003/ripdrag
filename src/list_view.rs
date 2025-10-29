use glib::clone;
use gtk::gdk::*;
use gtk::gio::{self, ListStore};
use gtk::prelude::*;
use gtk::{
    gdk, CenterBox, DragSource, Label, ListItem, ListView, MultiSelection, SignalListItemFactory,
    Widget,
};

use crate::file_object::FileObject;
use crate::util::{
    drag_source_and_exit, generate_content_provider, generate_file_model, setup_drag_source_all,
    ListWidget,
};
use crate::{ARGS, CURRENT_DIRECTORY};

pub fn generate_list_view() -> ListWidget {
    let list_data = build_list_data();
    setup_factory(&list_data.1, &list_data.0);
    let list_view = ListView::new(Some(list_data.0), Some(list_data.1));

    // lol
    let model = list_view
        .model()
        .unwrap()
        .downcast::<MultiSelection>()
        .unwrap()
        .model()
        .unwrap()
        .downcast::<ListStore>()
        .unwrap();
    let widget = list_view.upcast::<Widget>();

    ListWidget {
        list_model: model,
        widget,
    }
}

fn build_list_data() -> (MultiSelection, SignalListItemFactory) {
    let factory = SignalListItemFactory::new();
    (MultiSelection::new(Some(generate_file_model())), factory)
}

fn create_drag_source(row: &CenterBox, selection: &MultiSelection) -> DragSource {
    let drag_source = DragSource::new();
    drag_source.connect_prepare(clone!(
        #[weak]
        row,
        #[weak]
        selection,
        #[upgrade_or]
        None,
        move |me, _, _| {
            // This will prevent the click to trigger, a drag should happen!
            me.set_state(gtk::EventSequenceState::Claimed);
            let selected = selection.selection();
            let mut files: Vec<String> = Vec::with_capacity(selected.size() as usize);
            for index in 0..selected.size() {
                files.push(
                    selection
                        .item(selected.nth(index as u32))
                        .unwrap()
                        .downcast::<FileObject>()
                        .unwrap()
                        .file()
                        .uri()
                        .to_string(),
                );
            }

            // Is the activated row also selected?
            let row_file = get_file(&row).uri().to_string();
            if !files.contains(&row_file) {
                selection.unselect_all();
                generate_content_provider(&[row_file])
            } else {
                generate_content_provider(&files)
            }
        }
    ));

    if ARGS.get().unwrap().and_exit {
        drag_source_and_exit(&drag_source);
    }
    drag_source
}

fn create_gesture_click(row: &CenterBox) -> gtk::GestureClick {
    let click = gtk::GestureClick::new();
    click.connect_released(clone!(
        #[weak]
        row,
        move |me, _, _, _| {
            // Ignore the click when CTRL is being hold
            if me
                .current_event_state()
                .contains(gdk::ModifierType::CONTROL_MASK)
                || me
                    .current_event_state()
                    .contains(gdk::ModifierType::SHIFT_MASK)
            {
                return;
            }
            let file = get_file(&row);
            if let Some(file) = file.path() {
                let _ = opener::open(file).map_err(|err| {
                    eprint!("{}", err);
                    err
                });
            }
        }
    ));

    click
}

/// This is a helper function that makes a file from the CenterBox widget.
fn get_file(row: &CenterBox) -> gio::File {
    let file_widget = if ARGS.get().unwrap().icons_only {
        row.start_widget().unwrap()
    } else {
        row.center_widget().unwrap()
    };
    // This is safe because the tooltip is always set to the full path
    gio::File::for_path(file_widget.tooltip_text().unwrap())
}

// Setup the widgets in the ListView
fn setup_factory(factory: &SignalListItemFactory, list: &MultiSelection) {
    factory.connect_setup(clone!(
        #[weak]
        list,
        move |_, list_item| {
            let row = CenterBox::default();

            let drag_source = create_drag_source(&row, &list);
            if !ARGS.get().unwrap().no_click {
                let gesture_click = create_gesture_click(&row);
                row.add_controller(gesture_click);
            }
            row.add_controller(drag_source);

            list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .set_child(Some(&row));
        }
    ));

    factory.connect_bind(|_, list_item| {
        let file_object = list_item
            .downcast_ref::<ListItem>()
            .expect("Needs to be ListItem")
            .item()
            .and_downcast::<FileObject>()
            .expect("The item has to be an `FileObject`.");

        let file_row = list_item
            .downcast_ref::<ListItem>()
            .expect("Needs to be ListItem")
            .child()
            .and_downcast::<CenterBox>()
            .expect("The child has to be a `Label`.");

        // This is safe because the file needs to exist
        let path = file_object.file().parse_name().to_string();

        // show either relative or absolute path
        // only used for the display label
        let str = if ARGS.get().unwrap().basename
            || file_object
                .file()
                .has_parent(Some(CURRENT_DIRECTORY.get().unwrap()))
        {
            file_object
                .file()
                .basename()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string()
        } else {
            path.to_owned()
        };

        // Always set the tooltip to the full path
        // The label will change depending on the basename flag
        let label = Label::builder()
            .label(&str)
            .hexpand(true)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .tooltip_text(&path);

        if ARGS.get().unwrap().icons_only {
            file_row.set_start_widget(Some(&label.visible(false).build()));
            file_row.set_center_widget(Some(&file_object.thumbnail()))
        } else {
            file_row.set_center_widget(Some(&label.build()));
            file_row.set_start_widget(Some(&file_object.thumbnail()))
        }
    });
}

/// Creates an outer box that adds a drag all button to the top
pub fn create_outer_box(list: &ListWidget) -> Widget {
    let outer_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let row = gtk::CenterBox::builder()
        .height_request(ARGS.get().unwrap().icon_size)
        .focusable(true)
        .build();
    let label = Label::builder()
        .label("Drag All Items")
        .css_classes(["drag"])
        .hexpand(true)
        .tooltip_text("Drag All Items")
        .ellipsize(gtk::pango::EllipsizeMode::End);
    row.set_center_widget(Some(&label.build()));

    let drag_source = DragSource::new();
    setup_drag_source_all(&drag_source, &list.list_model);
    if !ARGS.get().unwrap().no_click {
        let gesture_click = create_gesture_click(&row);
        row.add_controller(gesture_click);
    }
    if ARGS.get().unwrap().and_exit {
        drag_source_and_exit(&drag_source);
    }

    // Add styling
    let provider = gtk::CssProvider::new();
    provider.load_from_data(include_str!("style.css"));
    gtk::style_context_add_provider_for_display(
        &gtk::gdk::Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    row.add_controller(drag_source);
    outer_box.append(&row);
    outer_box.append(&list.widget);
    let outer_box = outer_box.upcast::<Widget>();
    outer_box
}
