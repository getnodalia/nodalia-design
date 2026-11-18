//! relm4 container traits for the NDL widgets that hold children, so a `view!` can nest rows in a
//! group, a group in a page, or anything in a clamp, the way it does with GTK's own containers.

use relm4::{ContainerChild, RelmContainerExt, RelmRemoveExt};

use crate::{
    Bin, BreakpointBin, Carousel, Clamp, ClampScrollable, ExpanderRow, NavigationPage,
    NavigationView, PreferencesGroup, PreferencesPage, StatusPage, ToastOverlay, WrapBox,
};

/// Containers whose children may be any widget, added with `$add` and removed with `$remove`.
macro_rules! any_child {
    ($($ty:ty => $add:ident, $remove:ident;)+) => {
        $(
            impl ContainerChild for $ty {
                type Child = gtk::Widget;
            }

            impl RelmContainerExt for $ty {
                fn container_add(&self, widget: &impl AsRef<gtk::Widget>) {
                    self.$add(widget.as_ref());
                }
            }

            impl RelmRemoveExt for $ty {
                fn container_remove(&self, widget: &impl AsRef<gtk::Widget>) {
                    self.$remove(widget.as_ref());
                }
            }
        )+
    };
}

any_child! {
    PreferencesGroup => add, remove;
    ExpanderRow => add_row, remove;
    WrapBox => append, remove;
    Carousel => append, remove;
}

/// Containers that hold one child, which nesting replaces.
macro_rules! one_child {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl ContainerChild for $ty {
                type Child = gtk::Widget;
            }

            impl RelmContainerExt for $ty {
                fn container_add(&self, widget: &impl AsRef<gtk::Widget>) {
                    self.set_child(Some(widget.as_ref()));
                }
            }
        )+
    };
}

one_child!(
    Bin,
    BreakpointBin,
    Clamp,
    ClampScrollable,
    NavigationPage,
    StatusPage,
    ToastOverlay,
);

impl ContainerChild for PreferencesPage {
    type Child = PreferencesGroup;
}

impl RelmContainerExt for PreferencesPage {
    fn container_add(&self, group: &impl AsRef<PreferencesGroup>) {
        self.add(group.as_ref());
    }
}

impl RelmRemoveExt for PreferencesPage {
    fn container_remove(&self, group: &impl AsRef<PreferencesGroup>) {
        self.remove(group.as_ref());
    }
}

impl ContainerChild for NavigationView {
    type Child = NavigationPage;
}

impl RelmContainerExt for NavigationView {
    fn container_add(&self, page: &impl AsRef<NavigationPage>) {
        self.add(page.as_ref());
    }
}

impl RelmRemoveExt for NavigationView {
    fn container_remove(&self, page: &impl AsRef<NavigationPage>) {
        self.remove(page.as_ref());
    }
}
