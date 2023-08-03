use std::collections::HashSet;
use std::path::PathBuf;

use glib_macros::clone;
use gtk::gdk::*;
use gtk::gio::{self, ListStore};
use gtk::prelude::*;
use gtk::{
    gdk, CenterBox, DragSource, Label, ListItem, ListView, MultiSelection, SignalListItemFactory,
    Widget,
};

use crate::file_object::FileObject;
use crate::util::{
    generate_content_provider, generate_file_model, setup_drag_source_all, setup_drop_target,
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

    setup_drop_target(&model, &widget);

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
    if ARGS.get().unwrap().all {
        setup_drag_source_all(
            &drag_source,
            &selection.model().unwrap().downcast::<ListStore>().unwrap(),
        );
    } else {
        drag_source.connect_prepare(clone!(@weak row, @weak selection, => @default-return None, move |me, _, _| {
            me.set_state(gtk::EventSequenceState::Claimed);
            let selected = selection.selection();
            let mut set : HashSet<PathBuf> = HashSet::with_capacity(selected.size() as usize);
            for index in 0..selected.size() {
                set.insert(selection.item(selected.nth(index as u32)).unwrap().downcast::<FileObject>().unwrap().file().path().unwrap());
            }
            let row_file = get_file(&row).path().unwrap();
            if !set.contains(&row_file)
            {
                selection.unselect_all();
                generate_content_provider(&[row_file])
            } else {
                generate_content_provider(&set)
            }
        }));
    }
    drag_source
}

fn create_gesture_click(row: &CenterBox) -> gtk::GestureClick {
    let click = gtk::GestureClick::new();
    click.connect_released(clone!(@weak row => move |me, _, _, _|{
        if me.current_event_state().contains(gdk::ModifierType::CONTROL_MASK) {
            return;
        }
        let file = get_file(&row);
        if let Some(file) =  file.path() {
           let _ = opener::open(file).map_err(|err| {
                eprint!("{}", err);
                err
           });
        }
    }));

    click
}

fn get_file(row: &CenterBox) -> gio::File {
    let file_widget = if ARGS.get().unwrap().icons_only {
        row.start_widget().unwrap()
    } else {
        row.center_widget().unwrap()
    };
    gio::File::for_path(file_widget.downcast::<Label>().unwrap().text())
}

fn setup_factory(factory: &SignalListItemFactory, list: &MultiSelection) {
    factory.connect_setup(clone!(@weak list => move |_, list_item| {
        let row = CenterBox::default();
        let drag_source = create_drag_source(&row, &list);
        let gesture_click = create_gesture_click(&row);
        row.add_controller(gesture_click);
        row.add_controller(drag_source);

        list_item
            .downcast_ref::<ListItem>()
            .expect("Needs to be ListItem")
            .set_child(Some(&row));
    }));

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

        // show either relative or absolute path
        let str = if file_object
            .file()
            .has_parent(Some(CURRENT_DIRECTORY.get().unwrap()))
        {
            CURRENT_DIRECTORY
                .get()
                .unwrap()
                .relative_path(&file_object.file())
                .expect("Can't make a relative path")
                .to_str()
                .expect("Couldn't read file name")
                .to_string()
        } else {
            file_object.file().parse_name().to_string()
        };

        let label = Label::builder()
            .label(&str)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .tooltip_text(&str);

        if ARGS.get().unwrap().icons_only {
            file_row.set_start_widget(Some(&label.visible(false).build()));
            file_row.set_center_widget(Some(&file_object.thumbnail()))
        } else {
            file_row.set_center_widget(Some(&label.build()));
            file_row.set_start_widget(Some(&file_object.thumbnail()))
        }
    });
}
