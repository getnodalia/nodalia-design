//! Small containers: a single-child [`Bin`], a [`WrapBox`] that flows its children onto more
//! lines, and the icon-and-label [`ButtonContent`] a button can hold.

use std::cell::{Cell, OnceCell, RefCell};

use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

// ---------------------------------------------------------------------------------------------
// Bin

mod bin_imp {
    use super::*;

    #[derive(Default)]
    pub struct Bin {
        pub child: RefCell<Option<gtk::Widget>>,
    }

    impl Bin {
        pub(super) fn set_child_value(&self, child: Option<gtk::Widget>) {
            if self.child.borrow().as_ref() == child.as_ref() {
                return;
            }
            if let Some(old) = self.child.take() {
                old.unparent();
            }
            if let Some(c) = &child {
                c.set_parent(&*self.obj());
            }
            self.child.replace(child);
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Bin {
        const NAME: &'static str = "NdlBin";
        type Type = super::Bin;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<gtk::BinLayout>();
        }
    }

    impl ObjectImpl for Bin {
        fn dispose(&self) {
            if let Some(c) = self.child.take() {
                c.unparent();
            }
        }
    }

    impl WidgetImpl for Bin {}
}

glib::wrapper! {
    /// A widget that holds one child and adds nothing of its own.
    pub struct Bin(ObjectSubclass<bin_imp::Bin>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for Bin {
    fn default() -> Self {
        Self::new()
    }
}

impl Bin {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_child(&self, child: Option<&impl IsA<gtk::Widget>>) {
        self.imp()
            .set_child_value(child.map(|c| c.as_ref().clone()));
    }

    pub fn child(&self) -> Option<gtk::Widget> {
        self.imp().child.borrow().clone()
    }
}

// ---------------------------------------------------------------------------------------------
// WrapBox

mod wrap_imp {
    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::WrapBox)]
    pub struct WrapBox {
        #[property(get, set = Self::set_child_spacing)]
        pub child_spacing: Cell<i32>,
        #[property(get, set = Self::set_line_spacing)]
        pub line_spacing: Cell<i32>,
    }

    impl WrapBox {
        fn set_child_spacing(&self, v: i32) {
            self.child_spacing.set(v.max(0));
            self.obj().queue_resize();
        }

        fn set_line_spacing(&self, v: i32) {
            self.line_spacing.set(v.max(0));
            self.obj().queue_resize();
        }

        fn children(&self) -> Vec<gtk::Widget> {
            let mut out = Vec::new();
            let mut c = self.obj().first_child();
            while let Some(w) = c {
                c = w.next_sibling();
                if w.should_layout() {
                    out.push(w);
                }
            }
            out
        }

        /// Children placed into lines for `width`: each line is (children with their widths,
        /// line height).
        fn lines(&self, width: i32) -> Vec<(Vec<(gtk::Widget, i32)>, i32)> {
            let gap = self.child_spacing.get();
            let mut lines: Vec<(Vec<(gtk::Widget, i32)>, i32)> = Vec::new();
            let mut used = 0;
            for w in self.children() {
                let (_, nat, _, _) = w.measure(gtk::Orientation::Horizontal, -1);
                let (min_w, _, _, _) = w.measure(gtk::Orientation::Horizontal, -1);
                let cw = nat.min(width.max(min_w));
                let start_new = match lines.last() {
                    None => true,
                    Some((items, _)) => !items.is_empty() && used + gap + cw > width,
                };
                if start_new {
                    lines.push((Vec::new(), 0));
                    used = 0;
                } else {
                    used += gap;
                }
                let (_, h, _, _) = w.measure(gtk::Orientation::Vertical, cw);
                if let Some((items, lh)) = lines.last_mut() {
                    items.push((w, cw));
                    *lh = (*lh).max(h);
                }
                used += cw;
            }
            lines
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WrapBox {
        const NAME: &'static str = "NdlWrapBox";
        type Type = super::WrapBox;
        type ParentType = gtk::Widget;
    }

    #[glib::derived_properties]
    impl ObjectImpl for WrapBox {
        fn dispose(&self) {
            while let Some(c) = self.obj().first_child() {
                c.unparent();
            }
        }
    }

    impl WidgetImpl for WrapBox {
        fn request_mode(&self) -> gtk::SizeRequestMode {
            gtk::SizeRequestMode::HeightForWidth
        }

        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            let children = self.children();
            if orientation == gtk::Orientation::Horizontal {
                let gap = self.child_spacing.get();
                let mut min = 0;
                let mut nat = 0;
                for (i, w) in children.iter().enumerate() {
                    let (m, n, _, _) = w.measure(gtk::Orientation::Horizontal, -1);
                    min = min.max(m);
                    nat += n + if i > 0 { gap } else { 0 };
                }
                return (min, nat.max(min), -1, -1);
            }
            let width = if for_size < 0 {
                self.measure(gtk::Orientation::Horizontal, -1).1
            } else {
                for_size
            };
            let lines = self.lines(width);
            let gaps = self.line_spacing.get() * (lines.len().saturating_sub(1) as i32);
            let h = lines.iter().map(|(_, h)| h).sum::<i32>() + gaps;
            (h, h, -1, -1)
        }

        fn size_allocate(&self, width: i32, _height: i32, _baseline: i32) {
            let gap = self.child_spacing.get();
            let rtl = self.obj().direction() == gtk::TextDirection::Rtl;
            let mut y = 0;
            for (items, lh) in self.lines(width) {
                let mut x = 0;
                for (w, cw) in items {
                    let left = if rtl { width - x - cw } else { x };
                    w.size_allocate(&gtk::Allocation::new(left, y, cw, lh), -1);
                    x += cw + gap;
                }
                y += lh + self.line_spacing.get();
            }
        }
    }
}

glib::wrapper! {
    /// Lays its children out in a row and moves the ones that don't fit onto the next line.
    pub struct WrapBox(ObjectSubclass<wrap_imp::WrapBox>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for WrapBox {
    fn default() -> Self {
        Self::new()
    }
}

impl WrapBox {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn append(&self, child: &impl IsA<gtk::Widget>) {
        child.set_parent(self);
    }

    pub fn prepend(&self, child: &impl IsA<gtk::Widget>) {
        child.insert_after(self, gtk::Widget::NONE);
    }

    pub fn remove(&self, child: &impl IsA<gtk::Widget>) {
        if child.parent().as_ref() == Some(self.upcast_ref()) {
            child.unparent();
        }
    }
}

// ---------------------------------------------------------------------------------------------
// ButtonContent

mod content_imp {
    use super::*;

    #[derive(Default)]
    pub struct ButtonContent {
        pub icon: OnceCell<gtk::Image>,
        pub label: OnceCell<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ButtonContent {
        const NAME: &'static str = "NdlButtonContent";
        type Type = super::ButtonContent;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<gtk::BoxLayout>();
            klass.set_css_name("ndl-button-content");
        }
    }

    impl ObjectImpl for ButtonContent {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            if let Some(l) = obj.layout_manager().and_downcast::<gtk::BoxLayout>() {
                l.set_spacing(6);
            }
            obj.set_halign(gtk::Align::Center);
            let icon = gtk::Image::new();
            icon.set_visible(false);
            let label = gtk::Label::builder().use_underline(true).build();
            icon.set_parent(&*obj);
            label.set_parent(&*obj);
            let _ = self.icon.set(icon);
            let _ = self.label.set(label);
        }

        fn dispose(&self) {
            while let Some(c) = self.obj().first_child() {
                c.unparent();
            }
        }
    }

    impl WidgetImpl for ButtonContent {}
}

glib::wrapper! {
    /// An icon beside a label, for a button that needs both.
    pub struct ButtonContent(ObjectSubclass<content_imp::ButtonContent>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for ButtonContent {
    fn default() -> Self {
        Self::new()
    }
}

impl ButtonContent {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn builder() -> ButtonContentBuilder {
        ButtonContentBuilder::default()
    }

    pub fn set_icon_name(&self, icon: &str) {
        if let Some(i) = self.imp().icon.get() {
            i.set_icon_name(Some(icon));
            i.set_visible(!icon.is_empty());
        }
    }

    pub fn set_label(&self, label: &str) {
        if let Some(l) = self.imp().label.get() {
            l.set_label(label);
        }
    }

    pub fn label(&self) -> glib::GString {
        self.imp()
            .label
            .get()
            .map(|l| l.label())
            .unwrap_or_default()
    }
}

#[derive(Default)]
pub struct ButtonContentBuilder {
    icon_name: Option<String>,
    label: Option<String>,
}

impl ButtonContentBuilder {
    pub fn icon_name(mut self, icon: impl Into<String>) -> Self {
        self.icon_name = Some(icon.into());
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn build(self) -> ButtonContent {
        let c = ButtonContent::new();
        if let Some(i) = &self.icon_name {
            c.set_icon_name(i);
        }
        if let Some(l) = &self.label {
            c.set_label(l);
        }
        c
    }
}
