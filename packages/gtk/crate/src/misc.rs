//! Small widgets: status pages, spinners, avatars, width clamps and toasts.

use std::cell::{Cell, OnceCell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

// ---------------------------------------------------------------------------------------------
// StatusPage

mod status_imp {
    use super::*;

    #[derive(Default)]
    pub struct StatusPage {
        pub icon: OnceCell<gtk::Image>,
        pub title: OnceCell<gtk::Label>,
        pub description: OnceCell<gtk::Label>,
        pub child_slot: OnceCell<gtk::Box>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for StatusPage {
        const NAME: &'static str = "NdlStatusPage";
        type Type = super::StatusPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<gtk::BinLayout>();
            klass.set_css_name("ndl-status-page");
        }
    }

    impl ObjectImpl for StatusPage {
        fn constructed(&self) {
            self.parent_constructed();
            let column = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .spacing(12)
                .valign(gtk::Align::Center)
                .margin_top(36)
                .margin_bottom(36)
                .margin_start(12)
                .margin_end(12)
                .build();
            let icon = gtk::Image::builder()
                .pixel_size(128)
                .visible(false)
                .css_classes(["icon", "dim-label"])
                .build();
            let title = gtk::Label::builder()
                .wrap(true)
                .justify(gtk::Justification::Center)
                .visible(false)
                .css_classes(["title-1"])
                .build();
            let description = gtk::Label::builder()
                .wrap(true)
                .use_markup(true)
                .justify(gtk::Justification::Center)
                .visible(false)
                .css_classes(["body", "description"])
                .build();
            let slot = gtk::Box::builder()
                .halign(gtk::Align::Center)
                .visible(false)
                .build();
            column.append(&icon);
            column.append(&title);
            column.append(&description);
            column.append(&slot);
            let clamp = super::Clamp::builder()
                .maximum_size(600)
                .child(&column)
                .build();
            let scroller = gtk::ScrolledWindow::builder()
                .hscrollbar_policy(gtk::PolicyType::Never)
                .propagate_natural_height(true)
                .child(&clamp)
                .build();
            scroller.set_parent(&*self.obj());
            let _ = self.icon.set(icon);
            let _ = self.title.set(title);
            let _ = self.description.set(description);
            let _ = self.child_slot.set(slot);
        }

        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for StatusPage {}
}

glib::wrapper! {
    /// A page that says why there is nothing to show: an icon, a title, a description, and
    /// optionally something to do about it.
    pub struct StatusPage(ObjectSubclass<status_imp::StatusPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for StatusPage {
    fn default() -> Self {
        Self::new()
    }
}

impl StatusPage {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn builder() -> StatusPageBuilder {
        StatusPageBuilder::default()
    }

    pub fn set_icon_name(&self, icon: Option<&str>) {
        if let Some(i) = self.imp().icon.get() {
            i.set_icon_name(icon);
            i.set_visible(icon.is_some_and(|n| !n.is_empty()));
        }
    }

    pub fn set_paintable(&self, paintable: Option<&impl IsA<gtk::gdk::Paintable>>) {
        if let Some(i) = self.imp().icon.get() {
            i.set_paintable(paintable);
            i.set_visible(paintable.is_some());
        }
    }

    pub fn set_title(&self, title: &str) {
        if let Some(l) = self.imp().title.get() {
            l.set_label(title);
            l.set_visible(!title.is_empty());
        }
    }

    pub fn set_description(&self, description: Option<&str>) {
        if let Some(l) = self.imp().description.get() {
            let text = description.unwrap_or_default();
            l.set_label(text);
            l.set_visible(!text.is_empty());
        }
    }

    pub fn set_child(&self, child: Option<&impl IsA<gtk::Widget>>) {
        if let Some(slot) = self.imp().child_slot.get() {
            while let Some(old) = slot.first_child() {
                slot.remove(&old);
            }
            if let Some(c) = child {
                slot.append(c);
            }
            slot.set_visible(child.is_some());
        }
    }
}

#[derive(Default)]
#[must_use]
pub struct StatusPageBuilder {
    icon_name: Option<String>,
    title: Option<String>,
    description: Option<String>,
    css_classes: Vec<String>,
}

impl StatusPageBuilder {
    pub fn icon_name(mut self, icon: impl Into<String>) -> Self {
        self.icon_name = Some(icon.into());
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn css_classes(mut self, classes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.css_classes = classes.into_iter().map(Into::into).collect();
        self
    }

    pub fn build(self) -> StatusPage {
        let page = StatusPage::new();
        page.set_icon_name(self.icon_name.as_deref());
        if let Some(t) = &self.title {
            page.set_title(t);
        }
        page.set_description(self.description.as_deref());
        for c in &self.css_classes {
            page.add_css_class(c);
        }
        page
    }
}

// ---------------------------------------------------------------------------------------------
// Spinner

mod spinner_imp {
    use super::*;

    #[derive(Default)]
    pub struct Spinner {
        pub angle: Cell<f64>,
        pub tick: RefCell<Option<gtk::TickCallbackId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Spinner {
        const NAME: &'static str = "NdlSpinner";
        type Type = super::Spinner;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("ndl-spinner");
            klass.set_accessible_role(gtk::AccessibleRole::ProgressBar);
        }
    }

    impl ObjectImpl for Spinner {}

    impl WidgetImpl for Spinner {
        fn measure(&self, _orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            (16, 16, -1, -1)
        }

        fn map(&self) {
            self.parent_map();
            let id = self.obj().add_tick_callback(|widget, clock| {
                // One turn a second, read from the frame clock so a dropped frame does not slow it.
                let turns = clock.frame_time() as f64 / 1_000_000.0;
                widget.imp().angle.set(turns.fract() * 360.0);
                widget.queue_draw();
                glib::ControlFlow::Continue
            });
            self.tick.replace(Some(id));
        }

        fn unmap(&self) {
            if let Some(id) = self.tick.take() {
                id.remove();
            }
            self.parent_unmap();
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let obj = self.obj();
            let (w, h) = (obj.width() as f32, obj.height() as f32);
            let size = w.min(h);
            if size <= 0.0 {
                return;
            }
            let stroke_width = (size / 8.0).max(1.5);
            let radius = (size - stroke_width) / 2.0;
            let color = obj.color();
            let center = gtk::graphene::Point::new(w / 2.0, h / 2.0);

            let track = gtk::gsk::PathBuilder::new();
            track.add_circle(&center, radius);
            let stroke = gtk::gsk::Stroke::new(stroke_width);
            let faint = gtk::gdk::RGBA::new(
                color.red(),
                color.green(),
                color.blue(),
                color.alpha() * 0.2,
            );
            snapshot.append_stroke(&track.to_path(), &stroke, &faint);

            let start = self.angle.get().to_radians() as f32;
            let sweep = std::f32::consts::FRAC_PI_2 * 1.5;
            let arc = gtk::gsk::PathBuilder::new();
            arc.move_to(
                center.x() + radius * start.cos(),
                center.y() + radius * start.sin(),
            );
            let steps = 24;
            for i in 1..=steps {
                let a = start + sweep * i as f32 / steps as f32;
                arc.line_to(center.x() + radius * a.cos(), center.y() + radius * a.sin());
            }
            stroke.set_line_cap(gtk::gsk::LineCap::Round);
            snapshot.append_stroke(&arc.to_path(), &stroke, &color);
        }
    }
}

glib::wrapper! {
    /// A spinning arc in the current text colour. It spins whenever it is on screen.
    pub struct Spinner(ObjectSubclass<spinner_imp::Spinner>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

impl Spinner {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn builder() -> SpinnerBuilder {
        SpinnerBuilder::default()
    }
}

#[derive(Default)]
#[must_use]
pub struct SpinnerBuilder {
    width_request: Option<i32>,
    height_request: Option<i32>,
    halign: Option<gtk::Align>,
    valign: Option<gtk::Align>,
}

impl SpinnerBuilder {
    pub fn halign(mut self, align: gtk::Align) -> Self {
        self.halign = Some(align);
        self
    }

    pub fn valign(mut self, align: gtk::Align) -> Self {
        self.valign = Some(align);
        self
    }

    pub fn width_request(mut self, w: i32) -> Self {
        self.width_request = Some(w);
        self
    }

    pub fn height_request(mut self, h: i32) -> Self {
        self.height_request = Some(h);
        self
    }

    pub fn build(self) -> Spinner {
        let s = Spinner::new();
        if let Some(w) = self.width_request {
            s.set_width_request(w);
        }
        if let Some(h) = self.height_request {
            s.set_height_request(h);
        }
        if let Some(a) = self.halign {
            s.set_halign(a);
        }
        if let Some(a) = self.valign {
            s.set_valign(a);
        }
        s
    }
}

// ---------------------------------------------------------------------------------------------
// Avatar

mod avatar_imp {
    use super::*;

    #[derive(Default)]
    pub struct Avatar {
        pub size: Cell<i32>,
        pub show_initials: Cell<bool>,
        pub text: RefCell<String>,
        pub image: OnceCell<gtk::Image>,
        pub initials: OnceCell<gtk::Label>,
        pub custom: Cell<bool>,
        pub paintable: RefCell<Option<gtk::gdk::Paintable>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Avatar {
        const NAME: &'static str = "NdlAvatar";
        type Type = super::Avatar;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            // No layout manager, or GTK would skip `measure` and ignore the size.
            klass.set_css_name("ndl-avatar");
            klass.set_accessible_role(gtk::AccessibleRole::Img);
        }
    }

    impl ObjectImpl for Avatar {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.set_overflow(gtk::Overflow::Hidden);
            obj.set_valign(gtk::Align::Center);
            let image = gtk::Image::builder()
                .icon_name("avatar-default-symbolic")
                .build();
            let initials = gtk::Label::builder()
                .visible(false)
                .css_classes(["initials"])
                .build();
            image.set_parent(&*obj);
            initials.set_parent(&*obj);
            let _ = self.image.set(image);
            let _ = self.initials.set(initials);
            self.size.set(32);
        }

        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for Avatar {
        fn measure(&self, _orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            let s = self.size.get().max(0);
            (s, s, -1, -1)
        }

        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            let mut child = self.obj().first_child();
            while let Some(c) = child {
                if c.should_layout() {
                    c.allocate(width, height, baseline, None);
                }
                child = c.next_sibling();
            }
        }
    }
}

glib::wrapper! {
    /// A round picture of a person: their image if there is one, their initials if not.
    pub struct Avatar(ObjectSubclass<avatar_imp::Avatar>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for Avatar {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl Avatar {
    /// An avatar `size` px across for `text`, a name, showing its initials when `show_initials`
    /// and no image is set.
    pub fn new(size: i32, text: Option<&str>, show_initials: bool) -> Self {
        let a: Self = glib::Object::new();
        a.set_size(size);
        a.set_show_initials(show_initials);
        a.set_text(text);
        a
    }

    pub fn builder() -> AvatarBuilder {
        AvatarBuilder::default()
    }

    pub fn set_size(&self, size: i32) {
        self.imp().size.set(size);
        if let Some(i) = self.imp().image.get() {
            i.set_pixel_size(if self.imp().custom.get() {
                size
            } else {
                size / 2
            });
        }
        self.queue_resize();
    }

    pub fn size(&self) -> i32 {
        self.imp().size.get()
    }

    pub fn set_show_initials(&self, show: bool) {
        self.imp().show_initials.set(show);
        self.sync();
    }

    pub fn set_text(&self, text: Option<&str>) {
        self.imp().text.replace(text.unwrap_or_default().to_owned());
        self.sync();
    }

    pub fn text(&self) -> Option<glib::GString> {
        let text = self.imp().text.borrow();
        (!text.is_empty()).then(|| text.as_str().into())
    }

    pub fn custom_image(&self) -> Option<gtk::gdk::Paintable> {
        self.imp().paintable.borrow().clone()
    }

    /// The icon shown when there is neither an image nor initials.
    pub fn set_icon_name(&self, icon: Option<&str>) {
        if !self.imp().custom.get() {
            if let Some(i) = self.imp().image.get() {
                i.set_icon_name(Some(icon.unwrap_or("avatar-default-symbolic")));
            }
        }
    }

    pub fn set_custom_image(&self, paintable: Option<&impl IsA<gtk::gdk::Paintable>>) {
        let imp = self.imp();
        imp.custom.set(paintable.is_some());
        imp.paintable.replace(paintable.map(|p| p.as_ref().clone()));
        if let Some(i) = imp.image.get() {
            match paintable {
                Some(p) => i.set_paintable(Some(p)),
                None => i.set_icon_name(Some("avatar-default-symbolic")),
            }
        }
        self.set_size(self.size());
        self.sync();
    }

    fn sync(&self) {
        let imp = self.imp();
        let initials = initials_of(&imp.text.borrow());
        let show_initials = !imp.custom.get() && imp.show_initials.get() && !initials.is_empty();
        if let Some(l) = imp.initials.get() {
            l.set_label(&initials);
            l.set_visible(show_initials);
        }
        if let Some(i) = imp.image.get() {
            i.set_visible(!show_initials);
        }
        if imp.custom.get() {
            self.add_css_class("image");
        } else {
            self.remove_css_class("image");
        }
    }
}

/// Up to two initials: the first letters of the first and last words.
pub(crate) fn initials_of(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    let first = words.first().and_then(|w| w.chars().next());
    let last = if words.len() > 1 {
        words.last().and_then(|w| w.chars().next())
    } else {
        None
    };
    first
        .into_iter()
        .chain(last)
        .flat_map(char::to_uppercase)
        .collect()
}

#[derive(Default)]
#[must_use]
pub struct AvatarBuilder {
    size: Option<i32>,
    show_initials: Option<bool>,
    text: Option<String>,
    halign: Option<gtk::Align>,
}

impl AvatarBuilder {
    pub fn size(mut self, size: i32) -> Self {
        self.size = Some(size);
        self
    }

    pub fn show_initials(mut self, show: bool) -> Self {
        self.show_initials = Some(show);
        self
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn halign(mut self, align: gtk::Align) -> Self {
        self.halign = Some(align);
        self
    }

    pub fn build(self) -> Avatar {
        let a: Avatar = glib::Object::new();
        if let Some(s) = self.size {
            a.set_size(s);
        }
        if let Some(s) = self.show_initials {
            a.set_show_initials(s);
        }
        a.set_text(self.text.as_deref());
        if let Some(h) = self.halign {
            a.set_halign(h);
        }
        a
    }
}

// ---------------------------------------------------------------------------------------------
// Clamp and ClampScrollable

/// The width a clamped child gets out of `available`.
pub(crate) fn clamped(available: i32, maximum: i32, child_min: i32) -> i32 {
    available
        .min(maximum.max(child_min))
        .max(child_min.min(available))
        .max(0)
}

/// The most a clamp with `maximum` lets its child have out of `available`, when it starts
/// narrowing the gap at `threshold`: up to the threshold the child takes all the space, and from
/// there it eases towards the maximum instead of meeting it at once. A threshold of 0 (or one not
/// below the maximum) leaves the plain maximum.
pub(crate) fn tightened(available: i32, maximum: i32, threshold: i32) -> i32 {
    if threshold <= 0 || threshold >= maximum {
        return maximum;
    }
    let (lower, max) = (f64::from(threshold), f64::from(maximum));
    // The width at which an ease-out cubic reaches the maximum with the slope it started with.
    let upper = lower + 3.0 * (max - lower);
    let a = f64::from(available);
    if a <= lower {
        return available;
    }
    if a >= upper {
        return maximum;
    }
    let t = (a - lower) / (upper - lower);
    let eased = 1.0 - (1.0 - t).powi(3);
    (lower + (max - lower) * eased).round() as i32
}

/// Measures a clamped child along `orientation`.
fn clamp_measure(
    child: Option<&gtk::Widget>,
    max: i32,
    orientation: gtk::Orientation,
    for_size: i32,
) -> (i32, i32, i32, i32) {
    let Some(child) = child else {
        return (0, 0, -1, -1);
    };
    let (min, nat, _, _) = child.measure(orientation, for_size);
    if orientation == gtk::Orientation::Horizontal {
        (min, nat.min(max).max(min), -1, -1)
    } else {
        (min, nat, -1, -1)
    }
}

fn clamp_allocate(child: Option<&gtk::Widget>, max: i32, width: i32, height: i32, baseline: i32) {
    let Some(child) = child else { return };
    let (child_min, _, _, _) = child.measure(gtk::Orientation::Horizontal, -1);
    let w = clamped(width, max, child_min);
    let x = (width - w) / 2;
    let transform = gtk::gsk::Transform::new().translate(&gtk::graphene::Point::new(x as f32, 0.0));
    child.allocate(w, height, baseline, Some(transform));
}

mod clamp_imp {
    use super::*;

    pub struct Clamp {
        pub child: RefCell<Option<gtk::Widget>>,
        pub maximum_size: Cell<i32>,
        pub tightening_threshold: Cell<i32>,
    }

    impl Default for Clamp {
        fn default() -> Self {
            Self {
                child: RefCell::default(),
                maximum_size: Cell::new(600),
                tightening_threshold: Cell::new(0),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Clamp {
        const NAME: &'static str = "NdlClamp";
        type Type = super::Clamp;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("ndl-clamp");
        }
    }

    impl ObjectImpl for Clamp {
        fn dispose(&self) {
            if let Some(c) = self.child.take() {
                c.unparent();
            }
        }
    }

    impl WidgetImpl for Clamp {
        fn request_mode(&self) -> gtk::SizeRequestMode {
            gtk::SizeRequestMode::HeightForWidth
        }

        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            let for_size = if orientation == gtk::Orientation::Vertical && for_size >= 0 {
                for_size.min(self.maximum_size.get())
            } else {
                for_size
            };
            clamp_measure(
                self.child.borrow().as_ref(),
                self.maximum_size.get(),
                orientation,
                for_size,
            )
        }

        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            clamp_allocate(
                self.child.borrow().as_ref(),
                tightened(
                    width,
                    self.maximum_size.get(),
                    self.tightening_threshold.get(),
                ),
                width,
                height,
                baseline,
            );
        }
    }
}

glib::wrapper! {
    /// Holds its child to a maximum width, centred in whatever is wider.
    pub struct Clamp(ObjectSubclass<clamp_imp::Clamp>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for Clamp {
    fn default() -> Self {
        Self::new()
    }
}

impl Clamp {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn builder() -> ClampBuilder {
        ClampBuilder::default()
    }

    pub fn set_child(&self, child: Option<&impl IsA<gtk::Widget>>) {
        let imp = self.imp();
        if let Some(old) = imp.child.take() {
            old.unparent();
        }
        if let Some(c) = child {
            c.set_parent(self);
            imp.child.replace(Some(c.clone().upcast()));
        }
        self.queue_resize();
    }

    pub fn child(&self) -> Option<gtk::Widget> {
        self.imp().child.borrow().clone()
    }

    pub fn set_maximum_size(&self, size: i32) {
        self.imp().maximum_size.set(size);
        self.queue_resize();
    }

    pub fn maximum_size(&self) -> i32 {
        self.imp().maximum_size.get()
    }

    /// The width from which the child stops taking all the space and eases towards the maximum;
    /// 0, the default, clamps at the maximum straight away.
    pub fn set_tightening_threshold(&self, threshold: i32) {
        self.imp().tightening_threshold.set(threshold.max(0));
        self.queue_allocate();
    }
}

#[derive(Default)]
#[must_use]
pub struct ClampBuilder {
    child: Option<gtk::Widget>,
    maximum_size: Option<i32>,
}

impl ClampBuilder {
    pub fn child(mut self, child: &impl IsA<gtk::Widget>) -> Self {
        self.child = Some(child.clone().upcast());
        self
    }

    pub fn maximum_size(mut self, size: i32) -> Self {
        self.maximum_size = Some(size);
        self
    }

    pub fn build(self) -> Clamp {
        let c = Clamp::new();
        if let Some(m) = self.maximum_size {
            c.set_maximum_size(m);
        }
        c.set_child(self.child.as_ref());
        c
    }
}

mod clamp_scrollable_imp {
    use super::*;

    #[derive(glib::Properties)]
    #[properties(wrapper_type = super::ClampScrollable)]
    pub struct ClampScrollable {
        pub child: RefCell<Option<gtk::Widget>>,
        pub maximum_size: Cell<i32>,
        #[property(override_interface = gtk::Scrollable, get, set = Self::set_hadjustment, nullable)]
        pub hadjustment: RefCell<Option<gtk::Adjustment>>,
        #[property(override_interface = gtk::Scrollable, get, set = Self::set_vadjustment, nullable)]
        pub vadjustment: RefCell<Option<gtk::Adjustment>>,
        #[property(override_interface = gtk::Scrollable, get, set, builder(gtk::ScrollablePolicy::Minimum))]
        pub hscroll_policy: Cell<gtk::ScrollablePolicy>,
        #[property(override_interface = gtk::Scrollable, get, set, builder(gtk::ScrollablePolicy::Minimum))]
        pub vscroll_policy: Cell<gtk::ScrollablePolicy>,
    }

    impl Default for ClampScrollable {
        fn default() -> Self {
            Self {
                child: RefCell::default(),
                maximum_size: Cell::new(600),
                hadjustment: RefCell::default(),
                vadjustment: RefCell::default(),
                hscroll_policy: Cell::new(gtk::ScrollablePolicy::Minimum),
                vscroll_policy: Cell::new(gtk::ScrollablePolicy::Minimum),
            }
        }
    }

    impl ClampScrollable {
        fn forward(&self, name: &str, adj: Option<&gtk::Adjustment>) {
            if let Some(child) = self.child.borrow().as_ref() {
                if child.is::<gtk::Scrollable>() {
                    child.set_property(name, adj);
                }
            }
        }

        fn set_hadjustment(&self, adj: Option<gtk::Adjustment>) {
            self.forward("hadjustment", adj.as_ref());
            self.hadjustment.replace(adj);
        }

        fn set_vadjustment(&self, adj: Option<gtk::Adjustment>) {
            self.forward("vadjustment", adj.as_ref());
            self.vadjustment.replace(adj);
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ClampScrollable {
        const NAME: &'static str = "NdlClampScrollable";
        type Type = super::ClampScrollable;
        type ParentType = gtk::Widget;
        type Interfaces = (gtk::Scrollable,);

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("ndl-clamp-scrollable");
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ClampScrollable {
        fn dispose(&self) {
            if let Some(c) = self.child.take() {
                c.unparent();
            }
        }
    }

    impl WidgetImpl for ClampScrollable {
        fn request_mode(&self) -> gtk::SizeRequestMode {
            gtk::SizeRequestMode::HeightForWidth
        }

        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            clamp_measure(
                self.child.borrow().as_ref(),
                self.maximum_size.get(),
                orientation,
                for_size,
            )
        }

        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            clamp_allocate(
                self.child.borrow().as_ref(),
                self.maximum_size.get(),
                width,
                height,
                baseline,
            );
        }
    }

    impl ScrollableImpl for ClampScrollable {}
}

glib::wrapper! {
    /// A [`Clamp`] that is itself scrollable, passing its scroll adjustments to its child. Put it
    /// directly inside a `GtkScrolledWindow` around a list view: the list stays virtualised and
    /// the scrollbar stays at the window's edge.
    pub struct ClampScrollable(ObjectSubclass<clamp_scrollable_imp::ClampScrollable>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Scrollable;
}

impl Default for ClampScrollable {
    fn default() -> Self {
        Self::new()
    }
}

impl ClampScrollable {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_child(&self, child: Option<&impl IsA<gtk::Widget>>) {
        let imp = self.imp();
        if let Some(old) = imp.child.take() {
            if old.is::<gtk::Scrollable>() {
                old.set_property("hadjustment", None::<gtk::Adjustment>);
                old.set_property("vadjustment", None::<gtk::Adjustment>);
            }
            old.unparent();
        }
        if let Some(c) = child {
            let c: gtk::Widget = c.clone().upcast();
            c.set_parent(self);
            if c.is::<gtk::Scrollable>() {
                c.set_property("hadjustment", imp.hadjustment.borrow().as_ref());
                c.set_property("vadjustment", imp.vadjustment.borrow().as_ref());
            }
            imp.child.replace(Some(c));
        }
        self.queue_resize();
    }

    pub fn child(&self) -> Option<gtk::Widget> {
        self.imp().child.borrow().clone()
    }

    pub fn set_maximum_size(&self, size: i32) {
        self.imp().maximum_size.set(size);
        self.queue_resize();
    }
}

// ---------------------------------------------------------------------------------------------
// Toasts

/// Whether a toast waits its turn or goes in front of the ones waiting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToastPriority {
    #[default]
    Normal,
    /// Shown at once, before anything queued; the one showing is put back in the queue.
    High,
}

type ToastCallback = Rc<dyn Fn(&Toast)>;

#[derive(Default)]
struct ToastInner {
    title: RefCell<String>,
    custom_title: RefCell<Option<gtk::Widget>>,
    timeout: Cell<u32>,
    button_label: RefCell<Option<String>>,
    action_name: RefCell<Option<String>>,
    action_target: RefCell<Option<glib::Variant>>,
    priority: Cell<ToastPriority>,
    on_button: RefCell<Vec<ToastCallback>>,
    on_dismissed: RefCell<Vec<ToastCallback>>,
    overlay: RefCell<Option<glib::WeakRef<ToastOverlay>>>,
}

/// A short message shown over the bottom of a [`ToastOverlay`], with an optional button. A
/// handle: clones are the same toast, so it can be changed or dismissed while it shows.
#[derive(Clone)]
pub struct Toast(Rc<ToastInner>);

impl std::fmt::Debug for Toast {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Toast")
            .field("title", &self.title())
            .finish()
    }
}

impl PartialEq for Toast {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Toast {
    pub fn new(title: &str) -> Self {
        let t = Self(Rc::default());
        t.0.title.replace(title.to_owned());
        t.0.timeout.set(5);
        t
    }

    pub fn builder() -> ToastBuilder {
        ToastBuilder::default()
    }

    /// Seconds on screen; 0 keeps it until dismissed.
    pub fn set_timeout(&self, seconds: u32) {
        self.0.timeout.set(seconds);
    }

    pub fn timeout(&self) -> u32 {
        self.0.timeout.get()
    }

    pub fn title(&self) -> String {
        self.0.title.borrow().clone()
    }

    pub fn set_title(&self, title: &str) {
        self.0.title.replace(title.to_owned());
        self.refresh();
    }

    /// A widget shown in place of the title.
    pub fn set_custom_title(&self, widget: Option<&impl IsA<gtk::Widget>>) {
        self.0
            .custom_title
            .replace(widget.map(|w| w.as_ref().clone()));
        self.refresh();
    }

    pub fn set_button_label(&self, label: Option<&str>) {
        self.0.button_label.replace(label.map(str::to_owned));
        self.refresh();
    }

    /// The action the button activates, as `app.name` or `win.name`.
    pub fn set_action_name(&self, name: Option<&str>) {
        self.0.action_name.replace(name.map(str::to_owned));
    }

    pub fn set_action_target_value(&self, target: Option<&glib::Variant>) {
        self.0.action_target.replace(target.cloned());
    }

    pub fn set_priority(&self, priority: ToastPriority) {
        self.0.priority.set(priority);
    }

    pub fn priority(&self) -> ToastPriority {
        self.0.priority.get()
    }

    pub fn connect_button_clicked<F: Fn(&Self) + 'static>(&self, f: F) {
        self.0.on_button.borrow_mut().push(Rc::new(f));
    }

    pub fn connect_dismissed<F: Fn(&Self) + 'static>(&self, f: F) {
        self.0.on_dismissed.borrow_mut().push(Rc::new(f));
    }

    /// Takes the toast away, whether it is showing or still waiting.
    pub fn dismiss(&self) {
        match self.overlay() {
            Some(o) => o.dismiss_toast(self),
            None => self.emit_dismissed(),
        }
    }

    fn overlay(&self) -> Option<ToastOverlay> {
        self.0.overlay.borrow().as_ref().and_then(|o| o.upgrade())
    }

    fn refresh(&self) {
        if let Some(o) = self.overlay() {
            o.refresh(self);
        }
    }

    fn emit_button(&self) {
        let callbacks: Vec<_> = self.0.on_button.borrow().clone();
        for f in callbacks {
            f(self);
        }
    }

    fn emit_dismissed(&self) {
        self.0.overlay.replace(None);
        let callbacks: Vec<_> = self.0.on_dismissed.borrow().clone();
        for f in callbacks {
            f(self);
        }
    }
}

#[derive(Default)]
#[must_use]
pub struct ToastBuilder {
    title: Option<String>,
    timeout: Option<u32>,
    button_label: Option<String>,
    action_name: Option<String>,
    priority: Option<ToastPriority>,
}

impl ToastBuilder {
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn timeout(mut self, seconds: u32) -> Self {
        self.timeout = Some(seconds);
        self
    }

    pub fn button_label(mut self, label: impl Into<String>) -> Self {
        self.button_label = Some(label.into());
        self
    }

    pub fn action_name(mut self, name: impl Into<String>) -> Self {
        self.action_name = Some(name.into());
        self
    }

    pub fn priority(mut self, priority: ToastPriority) -> Self {
        self.priority = Some(priority);
        self
    }

    pub fn build(self) -> Toast {
        let t = Toast::new(self.title.as_deref().unwrap_or_default());
        if let Some(v) = self.timeout {
            t.set_timeout(v);
        }
        t.set_button_label(self.button_label.as_deref());
        t.set_action_name(self.action_name.as_deref());
        if let Some(p) = self.priority {
            t.set_priority(p);
        }
        t
    }
}

mod toast_imp {
    use super::*;

    #[derive(Default)]
    pub struct ToastOverlay {
        pub overlay: OnceCell<gtk::Overlay>,
        pub revealer: OnceCell<gtk::Revealer>,
        pub title_slot: OnceCell<gtk::Box>,
        pub label: OnceCell<gtk::Label>,
        pub button: OnceCell<gtk::Button>,
        pub queue: RefCell<VecDeque<Toast>>,
        pub current: RefCell<Option<Toast>>,
        pub timer: RefCell<Option<glib::SourceId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ToastOverlay {
        const NAME: &'static str = "NdlToastOverlay";
        type Type = super::ToastOverlay;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<gtk::BinLayout>();
            klass.set_css_name("ndl-toast-overlay");
        }
    }

    impl ObjectImpl for ToastOverlay {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            let overlay = gtk::Overlay::new();
            let label = gtk::Label::builder()
                .wrap(true)
                .max_width_chars(48)
                .use_markup(true)
                .build();
            let title_slot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            title_slot.append(&label);
            let button = gtk::Button::builder()
                .valign(gtk::Align::Center)
                .visible(false)
                .css_classes(["flat"])
                .build();
            let close = gtk::Button::builder()
                .icon_name("window-close-symbolic")
                .valign(gtk::Align::Center)
                .css_classes(["flat", "circular"])
                .build();
            let toast = gtk::Box::builder()
                .spacing(12)
                .css_classes(["ndl-toast"])
                .build();
            toast.append(&title_slot);
            toast.append(&button);
            toast.append(&close);
            let revealer = gtk::Revealer::builder()
                .transition_type(gtk::RevealerTransitionType::SlideUp)
                .halign(gtk::Align::Center)
                .valign(gtk::Align::End)
                .margin_bottom(12)
                .child(&toast)
                .build();
            let weak = obj.downgrade();
            close.connect_clicked(move |_| {
                if let Some(o) = weak.upgrade() {
                    o.dismiss_current();
                }
            });
            let weak = obj.downgrade();
            button.connect_clicked(move |button| {
                let Some(o) = weak.upgrade() else { return };
                let Some(t) = o.imp().current.borrow().clone() else {
                    return;
                };
                let action = t.0.action_name.borrow().clone();
                if let Some(action) = action {
                    let target = t.0.action_target.borrow().clone();
                    let _ = button.activate_action(&action, target.as_ref());
                }
                t.emit_button();
                o.dismiss_current();
            });
            let weak = obj.downgrade();
            revealer.connect_child_revealed_notify(move |r| {
                if !r.reveals_child() && !r.is_child_revealed() {
                    if let Some(o) = weak.upgrade() {
                        o.show_next();
                    }
                }
            });
            overlay.add_overlay(&revealer);
            overlay.set_parent(&*obj);
            let _ = self.overlay.set(overlay);
            let _ = self.revealer.set(revealer);
            let _ = self.title_slot.set(title_slot);
            let _ = self.label.set(label);
            let _ = self.button.set(button);
        }

        fn dispose(&self) {
            if let Some(t) = self.timer.take() {
                t.remove();
            }
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for ToastOverlay {}
}

glib::wrapper! {
    /// Shows [`Toast`]s over its child, one at a time, in the order they were added.
    pub struct ToastOverlay(ObjectSubclass<toast_imp::ToastOverlay>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for ToastOverlay {
    fn default() -> Self {
        Self::new()
    }
}

impl ToastOverlay {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_child(&self, child: Option<&impl IsA<gtk::Widget>>) {
        if let Some(o) = self.imp().overlay.get() {
            o.set_child(child);
        }
    }

    pub fn child(&self) -> Option<gtk::Widget> {
        self.imp().overlay.get().and_then(|o| o.child())
    }

    pub fn add_toast(&self, toast: Toast) {
        let imp = self.imp();
        toast.0.overlay.replace(Some(self.downgrade()));
        let showing = imp.current.borrow().clone();
        // The same words twice in a row say nothing new; keep the one showing.
        let repeat = showing.as_ref().is_some_and(|s| {
            *s == toast
                || (s.title() == toast.title()
                    && s.0.button_label.borrow().is_none()
                    && toast.0.button_label.borrow().is_none()
                    && toast.0.custom_title.borrow().is_none())
        }) && imp.queue.borrow().is_empty();
        if repeat {
            return;
        }
        if toast.priority() == ToastPriority::High {
            imp.queue.borrow_mut().push_front(toast);
            if let Some(s) = showing {
                // Put back behind the urgent one, then make room.
                imp.queue.borrow_mut().insert(1, s);
                self.hide_current();
                return;
            }
        } else {
            imp.queue.borrow_mut().push_back(toast);
        }
        if imp.current.borrow().is_none() {
            self.show_next();
        }
    }

    fn show_next(&self) {
        let imp = self.imp();
        if imp.revealer.get().is_some_and(|r| r.reveals_child()) {
            return;
        }
        imp.current.replace(None);
        let Some(toast) = imp.queue.borrow_mut().pop_front() else {
            return;
        };
        imp.current.replace(Some(toast.clone()));
        self.refresh(&toast);
        if let Some(r) = imp.revealer.get() {
            r.set_reveal_child(true);
        }
        if toast.timeout() > 0 {
            let weak = self.downgrade();
            let id = glib::timeout_add_local_once(
                Duration::from_secs(u64::from(toast.timeout())),
                move || {
                    if let Some(o) = weak.upgrade() {
                        o.imp().timer.take();
                        o.dismiss_current();
                    }
                },
            );
            if let Some(old) = imp.timer.replace(Some(id)) {
                old.remove();
            }
        }
    }

    /// Redraws `toast` if it is the one showing.
    fn refresh(&self, toast: &Toast) {
        let imp = self.imp();
        if imp.current.borrow().as_ref() != Some(toast) {
            return;
        }
        let (Some(slot), Some(label), Some(button)) =
            (imp.title_slot.get(), imp.label.get(), imp.button.get())
        else {
            return;
        };
        while let Some(c) = slot.first_child() {
            slot.remove(&c);
        }
        match toast.0.custom_title.borrow().as_ref() {
            Some(w) => {
                if let Some(old) = w.parent().and_downcast::<gtk::Box>() {
                    old.remove(w);
                }
                slot.append(w);
            }
            None => {
                label.set_label(&toast.title());
                slot.append(label);
            }
        }
        match toast.0.button_label.borrow().as_deref() {
            Some(l) if !l.is_empty() => {
                button.set_label(l);
                button.set_use_underline(true);
                button.set_visible(true);
            }
            _ => button.set_visible(false),
        }
    }

    fn hide_current(&self) {
        let imp = self.imp();
        if let Some(t) = imp.timer.take() {
            t.remove();
        }
        if let Some(r) = imp.revealer.get() {
            r.set_reveal_child(false);
            if !r.is_child_revealed() {
                self.show_next();
            }
        }
    }

    fn dismiss_current(&self) {
        let current = self.imp().current.borrow().clone();
        if let Some(t) = current {
            self.dismiss_toast(&t);
        }
    }

    fn dismiss_toast(&self, toast: &Toast) {
        let imp = self.imp();
        let showing = imp.current.borrow().as_ref() == Some(toast);
        if showing {
            self.hide_current();
        } else {
            imp.queue.borrow_mut().retain(|t| t != toast);
        }
        toast.emit_dismissed();
    }
}

#[cfg(test)]
mod tests {
    use super::{clamped, initials_of, tightened};

    #[test]
    fn initials_take_first_and_last_words() {
        assert_eq!(initials_of("Ludwig van Beethoven"), "LB");
        assert_eq!(initials_of("björk"), "B");
        assert_eq!(initials_of("   "), "");
    }

    #[test]
    fn clamp_never_exceeds_the_space() {
        assert_eq!(clamped(1000, 600, 100), 600);
        assert_eq!(clamped(400, 600, 100), 400);
        assert_eq!(clamped(400, 300, 350), 350);
        assert_eq!(clamped(200, 300, 350), 200);
    }

    #[test]
    fn tightening_eases_between_threshold_and_maximum() {
        assert_eq!(tightened(1000, 600, 0), 600);
        assert_eq!(tightened(300, 600, 400), 300);
        assert_eq!(tightened(400, 600, 400), 400);
        let mid = tightened(600, 600, 400);
        assert!(mid > 400 && mid < 600, "{mid}");
        assert_eq!(tightened(1000, 600, 400), 600);
    }
}
