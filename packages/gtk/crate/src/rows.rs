//! List rows: a title with an optional subtitle, things before it and things after it.
//!
//! Every row is its own `GtkListBoxRow` subclass sharing one layout ([`Parts`]) rather than a
//! subclass of [`ActionRow`]: subclassing a Rust-defined GObject type needs an `unsafe impl`, and
//! the rows have too little behaviour in common to be worth it.

use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;
use std::sync::OnceLock;

use gtk::glib;
use gtk::glib::subclass::Signal;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

/// The layout every row shares: `[prefixes] title/subtitle [suffixes]`.
#[derive(Debug)]
pub(crate) struct Parts {
    #[allow(dead_code)]
    pub header: gtk::Box,
    pub prefixes: gtk::Box,
    pub titles: gtk::Box,
    pub title: gtk::Label,
    pub subtitle: gtk::Label,
    pub suffixes: gtk::Box,
}

impl Parts {
    pub(crate) fn new(row: &gtk::ListBoxRow) -> Self {
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        header.add_css_class("header");
        let prefixes = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        prefixes.add_css_class("prefixes");
        prefixes.set_visible(false);
        let titles = gtk::Box::new(gtk::Orientation::Vertical, 2);
        titles.add_css_class("title-box");
        titles.set_hexpand(true);
        titles.set_valign(gtk::Align::Center);
        let title = gtk::Label::builder()
            .xalign(0.0)
            .wrap(true)
            .wrap_mode(gtk::pango::WrapMode::WordChar)
            .use_markup(true)
            .css_classes(["title"])
            .build();
        let subtitle = gtk::Label::builder()
            .xalign(0.0)
            .wrap(true)
            .wrap_mode(gtk::pango::WrapMode::WordChar)
            .use_markup(true)
            .visible(false)
            .css_classes(["subtitle"])
            .build();
        titles.append(&title);
        titles.append(&subtitle);
        let suffixes = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        suffixes.add_css_class("suffixes");
        suffixes.set_visible(false);
        header.append(&prefixes);
        header.append(&titles);
        header.append(&suffixes);
        row.set_child(Some(&header));
        row.add_css_class("ndl-row");
        Self {
            header,
            prefixes,
            titles,
            title,
            subtitle,
            suffixes,
        }
    }

    pub(crate) fn set_title(&self, title: &str) {
        self.title.set_label(title);
        self.title.set_visible(!title.is_empty());
    }

    pub(crate) fn set_subtitle(&self, subtitle: &str) {
        self.subtitle.set_label(subtitle);
        self.subtitle.set_visible(!subtitle.is_empty());
    }

    pub(crate) fn add_prefix(&self, widget: &impl IsA<gtk::Widget>) {
        self.prefixes.append(widget);
        self.prefixes.set_visible(true);
    }

    pub(crate) fn add_suffix(&self, widget: &impl IsA<gtk::Widget>) {
        self.suffixes.append(widget);
        self.suffixes.set_visible(true);
    }

    pub(crate) fn remove(&self, widget: &impl IsA<gtk::Widget>) {
        for holder in [&self.prefixes, &self.suffixes] {
            if widget.as_ref().parent().as_ref() == Some(holder.upcast_ref()) {
                holder.remove(widget);
                holder.set_visible(holder.first_child().is_some());
            }
        }
    }
}

/// Calls `f` whenever the list the row sits in activates it. `GtkListBox` reports activation on
/// the list, not the row, so the row listens to whichever list it is in now.
pub(crate) fn on_row_activated(row: &gtk::ListBoxRow, f: impl Fn() + 'static) {
    let f = Rc::new(f);
    let current: RefCell<Option<(glib::WeakRef<gtk::ListBox>, glib::SignalHandlerId)>> =
        RefCell::default();
    row.connect_parent_notify(move |row| {
        if let Some((list, id)) = current.take() {
            if let Some(list) = list.upgrade() {
                list.disconnect(id);
            }
        }
        if let Some(list) = row.parent().and_downcast::<gtk::ListBox>() {
            let me = row.downgrade();
            let f = f.clone();
            let id = list.connect_row_activated(move |_, activated| {
                if me.upgrade().as_ref() == Some(activated) {
                    f();
                }
            });
            current.replace(Some((list.downgrade(), id)));
        }
    });
}

/// Wraps `label` up to `lines` lines, then ellipsizes; 0 wraps without limit.
fn set_lines(label: &gtk::Label, lines: i32) {
    label.set_wrap(true);
    label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    if lines > 0 {
        label.set_lines(lines);
        label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    } else {
        label.set_lines(-1);
        label.set_ellipsize(gtk::pango::EllipsizeMode::None);
    }
}

/// The `activated` signal, as a static of the calling class. A `Signal` registers once, so a
/// single static shared by every row type made the second type to load panic.
macro_rules! activated_signal {
    () => {{
        static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
        SIGNALS.get_or_init(|| vec![Signal::builder("activated").build()])
    }};
}

/// The methods every row has, written once.
macro_rules! row_common {
    ($ty:ty) => {
        impl $ty {
            fn parts(&self) -> &Parts {
                self.imp()
                    .parts
                    .get_or_init(|| Parts::new(self.upcast_ref()))
            }

            pub fn set_title(&self, title: &str) {
                self.parts().set_title(title);
            }

            pub fn title(&self) -> glib::GString {
                self.parts().title.label()
            }

            pub fn set_subtitle(&self, subtitle: &str) {
                self.parts().set_subtitle(subtitle);
            }

            pub fn subtitle(&self) -> glib::GString {
                self.parts().subtitle.label()
            }

            /// The most lines the title wraps to before it ends in "…"; 0 for no limit.
            pub fn set_title_lines(&self, lines: i32) {
                set_lines(&self.parts().title, lines);
            }

            /// The most lines the subtitle wraps to before it ends in "…"; 0 for no limit.
            pub fn set_subtitle_lines(&self, lines: i32) {
                set_lines(&self.parts().subtitle, lines);
            }

            pub fn set_subtitle_selectable(&self, selectable: bool) {
                self.parts().subtitle.set_selectable(selectable);
            }

            /// Whether the title and subtitle are Pango markup.
            pub fn set_use_markup(&self, markup: bool) {
                self.parts().title.set_use_markup(markup);
                self.parts().subtitle.set_use_markup(markup);
            }

            pub fn add_prefix(&self, widget: &impl IsA<gtk::Widget>) {
                self.parts().add_prefix(widget);
            }

            pub fn add_suffix(&self, widget: &impl IsA<gtk::Widget>) {
                self.parts().add_suffix(widget);
            }

            pub fn remove(&self, widget: &impl IsA<gtk::Widget>) {
                self.parts().remove(widget);
            }

            pub fn connect_activated<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
                self.connect_local("activated", false, move |args| {
                    if let Some(row) = args.first().and_then(|v| v.get::<Self>().ok()) {
                        f(&row);
                    }
                    None
                })
            }
        }
    };
}

/// Builder fields every row takes.
macro_rules! row_builder {
    ($builder:ident, $ty:ty, { $($field:ident : $fty:ty),* $(,)? }, |$row:ident, $b:ident| $extra:block) => {
        #[derive(Default)]
        #[must_use]
        pub struct $builder {
            title: Option<String>,
            subtitle: Option<String>,
            sensitive: Option<bool>,
            css_classes: Vec<String>,
            $($field: Option<$fty>,)*
        }

        impl $builder {
            pub fn title(mut self, title: impl Into<String>) -> Self {
                self.title = Some(title.into());
                self
            }

            pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
                self.subtitle = Some(subtitle.into());
                self
            }

            pub fn sensitive(mut self, sensitive: bool) -> Self {
                self.sensitive = Some(sensitive);
                self
            }

            pub fn css_classes(mut self, classes: impl IntoIterator<Item = impl Into<String>>) -> Self {
                self.css_classes = classes.into_iter().map(Into::into).collect();
                self
            }

            $(
                pub fn $field(mut self, value: impl Into<$fty>) -> Self {
                    self.$field = Some(value.into());
                    self
                }
            )*

            pub fn build(self) -> $ty {
                let $row = <$ty>::new();
                if let Some(t) = &self.title {
                    $row.set_title(t);
                }
                if let Some(s) = &self.subtitle {
                    $row.set_subtitle(s);
                }
                if let Some(s) = self.sensitive {
                    $row.set_sensitive(s);
                }
                for class in &self.css_classes {
                    $row.add_css_class(class);
                }
                let $b = self;
                $extra
                $row
            }
        }
    };
}

// ---------------------------------------------------------------------------------------------
// ActionRow

mod action_imp {
    use super::*;

    #[derive(Default)]
    pub struct ActionRow {
        pub(crate) parts: OnceCell<Parts>,
        pub activatable_widget: RefCell<Option<glib::WeakRef<gtk::Widget>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ActionRow {
        const NAME: &'static str = "NdlActionRow";
        type Type = super::ActionRow;
        type ParentType = gtk::ListBoxRow;
    }

    impl ObjectImpl for ActionRow {
        fn signals() -> &'static [Signal] {
            activated_signal!()
        }

        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.set_activatable(false);
            obj.add_css_class("ndl-action-row");
            let weak = obj.downgrade();
            on_row_activated(obj.upcast_ref(), move || {
                if let Some(row) = weak.upgrade() {
                    row.activate_row();
                }
            });
        }
    }

    impl WidgetImpl for ActionRow {}
    impl ListBoxRowImpl for ActionRow {}
}

glib::wrapper! {
    /// A row with a title, a subtitle, and widgets before and after them.
    pub struct ActionRow(ObjectSubclass<action_imp::ActionRow>)
        @extends gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

row_common!(ActionRow);
row_builder!(ActionRowBuilder, ActionRow, { activatable: bool }, |row, b| {
    if let Some(a) = b.activatable {
        row.set_activatable(a);
    }
});

impl Default for ActionRow {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionRow {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn builder() -> ActionRowBuilder {
        ActionRowBuilder::default()
    }

    /// The widget activating the row activates, the way a label activates its mnemonic target.
    pub fn set_activatable_widget(&self, widget: Option<&impl IsA<gtk::Widget>>) {
        self.imp()
            .activatable_widget
            .replace(widget.map(|w| w.as_ref().downgrade()));
        if widget.is_some() {
            self.set_activatable(true);
        }
    }

    fn activate_row(&self) {
        let target = self
            .imp()
            .activatable_widget
            .borrow()
            .as_ref()
            .and_then(|w| w.upgrade());
        if let Some(target) = target {
            target.mnemonic_activate(false);
        }
        self.emit_by_name::<()>("activated", &[]);
    }
}

// ---------------------------------------------------------------------------------------------
// SwitchRow

mod switch_imp {
    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::SwitchRow)]
    pub struct SwitchRow {
        pub(crate) parts: OnceCell<Parts>,
        #[property(get, set)]
        pub active: Cell<bool>,
        pub switch: OnceCell<gtk::Switch>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SwitchRow {
        const NAME: &'static str = "NdlSwitchRow";
        type Type = super::SwitchRow;
        type ParentType = gtk::ListBoxRow;
    }

    #[glib::derived_properties]
    impl ObjectImpl for SwitchRow {
        fn signals() -> &'static [Signal] {
            activated_signal!()
        }

        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.add_css_class("ndl-switch-row");
            let switch = gtk::Switch::builder()
                .valign(gtk::Align::Center)
                .can_focus(false)
                .build();
            obj.bind_property("active", &switch, "active")
                .bidirectional()
                .sync_create()
                .build();
            obj.add_suffix(&switch);
            let _ = self.switch.set(switch);
            let weak = obj.downgrade();
            on_row_activated(obj.upcast_ref(), move || {
                if let Some(row) = weak.upgrade() {
                    row.set_active(!row.active());
                    row.emit_by_name::<()>("activated", &[]);
                }
            });
        }
    }

    impl WidgetImpl for SwitchRow {}
    impl ListBoxRowImpl for SwitchRow {}
}

glib::wrapper! {
    /// A row holding an on/off switch. Activating the row flips it.
    pub struct SwitchRow(ObjectSubclass<switch_imp::SwitchRow>)
        @extends gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

row_common!(SwitchRow);
row_builder!(SwitchRowBuilder, SwitchRow, { active: bool }, |row, b| {
    if let Some(a) = b.active {
        row.set_active(a);
    }
});

impl Default for SwitchRow {
    fn default() -> Self {
        Self::new()
    }
}

impl SwitchRow {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn builder() -> SwitchRowBuilder {
        SwitchRowBuilder::default()
    }

    pub fn is_active(&self) -> bool {
        self.active()
    }
}

// ---------------------------------------------------------------------------------------------
// ComboRow

mod combo_imp {
    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::ComboRow)]
    pub struct ComboRow {
        pub(crate) parts: OnceCell<Parts>,
        #[property(get, set)]
        pub selected: Cell<u32>,
        pub dropdown: OnceCell<gtk::DropDown>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ComboRow {
        const NAME: &'static str = "NdlComboRow";
        type Type = super::ComboRow;
        type ParentType = gtk::ListBoxRow;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ComboRow {
        fn signals() -> &'static [Signal] {
            activated_signal!()
        }

        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.add_css_class("ndl-combo-row");
            let dropdown = gtk::DropDown::builder().valign(gtk::Align::Center).build();
            dropdown.add_css_class("flat");
            obj.bind_property("selected", &dropdown, "selected")
                .bidirectional()
                .sync_create()
                .build();
            obj.add_suffix(&dropdown);
            let _ = self.dropdown.set(dropdown);
            let weak = obj.downgrade();
            on_row_activated(obj.upcast_ref(), move || {
                if let Some(row) = weak.upgrade() {
                    if let Some(d) = row.imp().dropdown.get() {
                        d.activate();
                    }
                    row.emit_by_name::<()>("activated", &[]);
                }
            });
        }
    }

    impl WidgetImpl for ComboRow {}
    impl ListBoxRowImpl for ComboRow {}
}

glib::wrapper! {
    /// A row that picks one item out of a list.
    pub struct ComboRow(ObjectSubclass<combo_imp::ComboRow>)
        @extends gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

row_common!(ComboRow);
/// Anything that is a list model, for [`ComboRowBuilder::model`].
pub struct ModelArg(gtk::gio::ListModel);

impl<T: IsA<gtk::gio::ListModel>> From<&T> for ModelArg {
    fn from(model: &T) -> Self {
        Self(model.as_ref().clone())
    }
}

row_builder!(ComboRowBuilder, ComboRow, { selected: u32, model: ModelArg }, |row, b| {
    if let Some(m) = &b.model {
        row.set_model(Some(&m.0));
    }
    if let Some(s) = b.selected {
        row.set_selected(s);
    }
});

impl Default for ComboRow {
    fn default() -> Self {
        Self::new()
    }
}

impl ComboRow {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn builder() -> ComboRowBuilder {
        ComboRowBuilder::default()
    }

    pub fn set_model(&self, model: Option<&impl IsA<gtk::gio::ListModel>>) {
        if let Some(d) = self.imp().dropdown.get() {
            let selected = self.selected();
            d.set_model(model);
            d.set_selected(selected);
        }
    }

    pub fn model(&self) -> Option<gtk::gio::ListModel> {
        self.imp().dropdown.get().and_then(|d| d.model())
    }

    /// How the chosen item shows in the row (and in the list, unless it has its own).
    pub fn set_factory(&self, factory: Option<&impl IsA<gtk::ListItemFactory>>) {
        if let Some(d) = self.imp().dropdown.get() {
            d.set_factory(factory);
        }
    }

    /// How each item shows in the list that opens.
    pub fn set_list_factory(&self, factory: Option<&impl IsA<gtk::ListItemFactory>>) {
        if let Some(d) = self.imp().dropdown.get() {
            d.set_list_factory(factory);
        }
    }

    pub fn selected_item(&self) -> Option<glib::Object> {
        self.imp().dropdown.get().and_then(|d| d.selected_item())
    }

    /// Lets the list be searched by the string `expression` gives for each item.
    pub fn set_expression(&self, expression: Option<impl AsRef<gtk::Expression>>) {
        if let Some(d) = self.imp().dropdown.get() {
            d.set_expression(expression);
        }
    }

    pub fn set_enable_search(&self, enable: bool) {
        if let Some(d) = self.imp().dropdown.get() {
            d.set_enable_search(enable);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// SpinRow

mod spin_imp {
    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::SpinRow)]
    pub struct SpinRow {
        pub(crate) parts: OnceCell<Parts>,
        #[property(get, set)]
        pub value: Cell<f64>,
        pub spin: OnceCell<gtk::SpinButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SpinRow {
        const NAME: &'static str = "NdlSpinRow";
        type Type = super::SpinRow;
        type ParentType = gtk::ListBoxRow;
    }

    #[glib::derived_properties]
    impl ObjectImpl for SpinRow {
        fn signals() -> &'static [Signal] {
            activated_signal!()
        }

        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.set_activatable(false);
            obj.add_css_class("ndl-spin-row");
            let spin = gtk::SpinButton::builder()
                .valign(gtk::Align::Center)
                .build();
            obj.add_suffix(&spin);
            let _ = self.spin.set(spin);
        }
    }

    impl WidgetImpl for SpinRow {}
    impl ListBoxRowImpl for SpinRow {}
}

glib::wrapper! {
    /// A row holding a number with buttons to step it.
    pub struct SpinRow(ObjectSubclass<spin_imp::SpinRow>)
        @extends gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

row_common!(SpinRow);

impl SpinRow {
    /// A row stepping through `min..=max` by `step`, showing as many decimals as `step` needs.
    pub fn with_range(min: f64, max: f64, step: f64) -> Self {
        let row: Self = glib::Object::new();
        if let Some(spin) = row.imp().spin.get() {
            let adjustment = gtk::Adjustment::new(min, min, max, step, step * 10.0, 0.0);
            spin.set_adjustment(&adjustment);
            spin.set_digits(digits_for(step));
            row.set_value(min);
            row.bind_property("value", &adjustment, "value")
                .bidirectional()
                .sync_create()
                .build();
        }
        row
    }

    pub fn spin_button(&self) -> Option<gtk::SpinButton> {
        self.imp().spin.get().cloned()
    }

    pub fn new() -> Self {
        glib::Object::new()
    }

    /// Steps through `adjustment` instead, which the row's value then follows.
    pub fn set_adjustment(&self, adjustment: Option<&gtk::Adjustment>) {
        let Some(spin) = self.imp().spin.get() else {
            return;
        };
        let Some(adjustment) = adjustment else { return };
        spin.set_adjustment(adjustment);
        self.bind_property("value", adjustment, "value")
            .bidirectional()
            .sync_create()
            .build();
        adjustment.set_value(self.value().clamp(adjustment.lower(), adjustment.upper()));
        self.set_value(adjustment.value());
    }

    pub fn adjustment(&self) -> Option<gtk::Adjustment> {
        self.imp().spin.get().map(|s| s.adjustment())
    }

    pub fn set_digits(&self, digits: u32) {
        if let Some(spin) = self.imp().spin.get() {
            spin.set_digits(digits);
        }
    }

    pub fn set_range(&self, min: f64, max: f64) {
        if let Some(spin) = self.imp().spin.get() {
            spin.set_range(min, max);
        }
    }

    /// Whether only digits (and a sign and decimal point) can be typed.
    pub fn set_numeric(&self, numeric: bool) {
        if let Some(spin) = self.imp().spin.get() {
            spin.set_numeric(numeric);
        }
    }

    pub fn set_increments(&self, step: f64, page: f64) {
        if let Some(spin) = self.imp().spin.get() {
            spin.set_increments(step, page);
        }
    }
}

impl Default for SpinRow {
    fn default() -> Self {
        Self::new()
    }
}

/// Decimal places needed to show every multiple of `step`.
fn digits_for(step: f64) -> u32 {
    let mut digits = 0;
    let mut s = step.abs();
    while digits < 6 && s.is_finite() && (s - s.round()).abs() > 1e-9 {
        s *= 10.0;
        digits += 1;
    }
    digits
}

// ---------------------------------------------------------------------------------------------
// ButtonRow

mod button_imp {
    use super::*;

    #[derive(Default)]
    pub struct ButtonRow {
        pub(crate) parts: OnceCell<Parts>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ButtonRow {
        const NAME: &'static str = "NdlButtonRow";
        type Type = super::ButtonRow;
        type ParentType = gtk::ListBoxRow;
    }

    impl ObjectImpl for ButtonRow {
        fn signals() -> &'static [Signal] {
            activated_signal!()
        }

        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.add_css_class("ndl-button-row");
            let parts = obj.parts();
            parts.title.set_xalign(0.5);
            parts.titles.set_halign(gtk::Align::Center);
            let weak = obj.downgrade();
            on_row_activated(obj.upcast_ref(), move || {
                if let Some(row) = weak.upgrade() {
                    row.emit_by_name::<()>("activated", &[]);
                }
            });
        }
    }

    impl WidgetImpl for ButtonRow {}
    impl ListBoxRowImpl for ButtonRow {}
}

glib::wrapper! {
    /// A row that is a button: its title centred, the whole row the target.
    pub struct ButtonRow(ObjectSubclass<button_imp::ButtonRow>)
        @extends gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

row_common!(ButtonRow);
row_builder!(ButtonRowBuilder, ButtonRow, {}, |_row, _b| {});

impl Default for ButtonRow {
    fn default() -> Self {
        Self::new()
    }
}

impl ButtonRow {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn builder() -> ButtonRowBuilder {
        ButtonRowBuilder::default()
    }
}

// ---------------------------------------------------------------------------------------------
// EntryRow and PasswordEntryRow

mod entry_imp {
    use super::*;

    /// What an entry row and a password row share: the editable they hand everything to, and
    /// the apply button.
    macro_rules! entry_imp {
        ($ty:ident, $name:literal, $make:expr) => {
            #[derive(Default)]
            pub struct $ty {
                pub(crate) parts: OnceCell<Parts>,
                pub editable: OnceCell<gtk::Editable>,
                pub apply: OnceCell<gtk::Button>,
                /// Whether the apply button shows once the text changes.
                pub show_apply: Cell<bool>,
            }

            #[glib::object_subclass]
            impl ObjectSubclass for $ty {
                const NAME: &'static str = $name;
                type Type = super::$ty;
                type ParentType = gtk::ListBoxRow;
                type Interfaces = (gtk::Editable,);
            }

            impl ObjectImpl for $ty {
                fn signals() -> &'static [Signal] {
                    static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
                    SIGNALS.get_or_init(|| {
                        // `changed` is the Editable interface's, which the delegate forwards.
                        vec![
                            Signal::builder("activated").build(),
                            Signal::builder("apply").build(),
                            Signal::builder("entry-activated").build(),
                        ]
                    })
                }

                fn constructed(&self) {
                    self.parent_constructed();
                    let obj = self.obj();
                    let make: fn() -> gtk::Editable = $make;
                    let editable = make();
                    let parts = self.parts.get_or_init(|| Parts::new(obj.upcast_ref()));
                    build_entry_row(obj.upcast_ref(), parts, &editable);
                    let apply = apply_button(obj.upcast_ref(), &editable);
                    parts.add_suffix(&apply);
                    let _ = self.apply.set(apply);
                    let _ = self.editable.set(editable);
                    obj.init_delegate();
                }

                fn dispose(&self) {
                    self.obj().finish_delegate();
                }

                fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
                    self.delegate_set_property(id, value, pspec);
                }

                fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
                    self.delegate_get_property(id, pspec)
                        .unwrap_or_else(|| pspec.default_value().clone())
                }
            }

            impl WidgetImpl for $ty {
                fn grab_focus(&self) -> bool {
                    self.editable.get().is_some_and(|e| e.grab_focus())
                }
            }

            impl ListBoxRowImpl for $ty {}

            impl EditableImpl for $ty {
                fn delegate(&self) -> Option<gtk::Editable> {
                    self.editable.get().cloned()
                }
            }
        };
    }

    entry_imp!(EntryRow, "NdlEntryRow", || gtk::Text::new().upcast());
    entry_imp!(PasswordEntryRow, "NdlPasswordEntryRow", || {
        let entry = gtk::PasswordEntry::builder().show_peek_icon(true).build();
        entry.add_css_class("flat");
        entry.upcast()
    });
}

/// Whether `row`, an entry or password row, asked for an apply button.
fn shows_apply(row: &gtk::ListBoxRow) -> bool {
    if let Some(r) = row.downcast_ref::<EntryRow>() {
        return r.imp().show_apply.get();
    }
    row.downcast_ref::<PasswordEntryRow>()
        .is_some_and(|r| r.imp().show_apply.get())
}

/// The button that confirms an edit, shown once the text changes when the row asks for it.
/// Enter in the field does what it does.
fn apply_button(row: &gtk::ListBoxRow, editable: &gtk::Editable) -> gtk::Button {
    let apply = gtk::Button::from_icon_name("object-select-symbolic");
    apply.add_css_class("suggested-action");
    apply.add_css_class("circular");
    apply.set_valign(gtk::Align::Center);
    apply.set_tooltip_text(Some("Apply"));
    apply.set_visible(false);
    let fire = {
        let row = row.downgrade();
        let apply = apply.downgrade();
        move || {
            let (Some(row), Some(apply)) = (row.upgrade(), apply.upgrade()) else {
                return;
            };
            if apply.is_visible() {
                apply.set_visible(false);
                row.emit_by_name::<()>("apply", &[]);
            }
        }
    };
    let on_click = fire.clone();
    apply.connect_clicked(move |_| on_click());
    {
        let row = row.downgrade();
        let apply = apply.downgrade();
        editable.connect_changed(move |_| {
            let (Some(row), Some(apply)) = (row.upgrade(), apply.upgrade()) else {
                return;
            };
            apply.set_visible(shows_apply(&row));
        });
    }
    {
        let row = row.downgrade();
        editable.connect_local("activate", false, move |_| {
            if let Some(row) = row.upgrade() {
                row.emit_by_name::<()>("entry-activated", &[]);
            }
            fire();
            None
        });
    }
    apply
}

/// Puts the title above an editable that spans the row.
fn build_entry_row(row: &gtk::ListBoxRow, parts: &Parts, editable: &gtk::Editable) {
    row.set_activatable(false);
    row.add_css_class("ndl-entry-row");
    parts.title.add_css_class("caption");
    parts.titles.append(editable);
    editable.set_hexpand(true);
    let weak = row.downgrade();
    let focus = gtk::GestureClick::new();
    let target = editable.downgrade();
    focus.connect_released(move |_, _, _, _| {
        if let (Some(_), Some(e)) = (weak.upgrade(), target.upgrade()) {
            e.grab_focus();
        }
    });
    row.add_controller(focus);
    keep_title_in_step(parts, editable);
}

/// An empty field that is not being edited shows its title as the placeholder, centred in the
/// row, instead of a small title above nothing. The title moves up once there is something to
/// label: when the field has text or focus.
fn keep_title_in_step(parts: &Parts, editable: &gtk::Editable) {
    use std::rc::Rc;
    let has_placeholder = editable.find_property("placeholder-text").is_some();
    let sync: Rc<dyn Fn()> = {
        let title = parts.title.downgrade();
        let editable = editable.downgrade();
        Rc::new(move || {
            let (Some(title), Some(editable)) = (title.upgrade(), editable.upgrade()) else {
                return;
            };
            let focused = editable
                .upcast_ref::<gtk::Widget>()
                .state_flags()
                .contains(gtk::StateFlags::FOCUS_WITHIN);
            let resting = editable.text().is_empty() && !focused;
            let shown = !resting || !has_placeholder;
            if title.is_visible() != shown {
                title.set_visible(shown);
            }
            if has_placeholder {
                let text = title.text();
                editable.set_property("placeholder-text", text.as_str());
            }
        })
    };
    let again = sync.clone();
    editable.connect_changed(move |_| again());
    let again = sync.clone();
    editable.connect_state_flags_changed(move |_, _| again());
    let again = sync.clone();
    parts.title.connect_label_notify(move |_| again());
    // Setting a title makes the label visible; the row decides again.
    let again = sync.clone();
    parts.title.connect_visible_notify(move |_| again());
    sync();
}

macro_rules! entry_common {
    ($ty:ty, $builder:ident) => {
        impl $ty {
            pub fn connect_changed<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
                self.connect_local("changed", false, move |args| {
                    if let Some(row) = args.first().and_then(|v| v.get::<Self>().ok()) {
                        f(&row);
                    }
                    None
                })
            }

            pub fn builder() -> $builder {
                $builder::default()
            }

            pub fn new() -> Self {
                glib::Object::new()
            }

            /// Shows a button to confirm the edit once the text changes; it and Enter emit
            /// `apply`.
            pub fn set_show_apply_button(&self, show: bool) {
                self.imp().show_apply.set(show);
                if !show {
                    if let Some(b) = self.imp().apply.get() {
                        b.set_visible(false);
                    }
                }
            }

            pub fn shows_apply_button(&self) -> bool {
                self.imp().show_apply.get()
            }

            pub fn connect_apply<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
                self.connect_local("apply", false, move |args| {
                    if let Some(row) = args.first().and_then(|v| v.get::<Self>().ok()) {
                        f(&row);
                    }
                    None
                })
            }

            /// What is typed, for the on-screen keyboard and input methods.
            pub fn set_input_purpose(&self, purpose: gtk::InputPurpose) {
                if let Some(e) = self.imp().editable.get() {
                    if e.find_property("input-purpose").is_some() {
                        e.set_property("input-purpose", purpose);
                    }
                }
            }

            pub fn set_input_hints(&self, hints: gtk::InputHints) {
                if let Some(e) = self.imp().editable.get() {
                    if e.find_property("input-hints").is_some() {
                        e.set_property("input-hints", hints);
                    }
                }
            }

            /// Whether Enter in the field also activates the window's default widget.
            pub fn set_activates_default(&self, activates: bool) {
                if let Some(e) = self.imp().editable.get() {
                    if e.find_property("activates-default").is_some() {
                        e.set_property("activates-default", activates);
                    }
                }
            }

            /// Enter in the field.
            pub fn connect_entry_activated<F: Fn(&Self) + 'static>(
                &self,
                f: F,
            ) -> glib::SignalHandlerId {
                self.connect_local("entry-activated", false, move |args| {
                    if let Some(row) = args.first().and_then(|v| v.get::<Self>().ok()) {
                        f(&row);
                    }
                    None
                })
            }
        }

        impl Default for $ty {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

glib::wrapper! {
    /// A row whose value is a line of text, with its title above it.
    pub struct EntryRow(ObjectSubclass<entry_imp::EntryRow>)
        @extends gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget, gtk::Editable;
}

row_common!(EntryRow);
entry_common!(EntryRow, EntryRowBuilder);
row_builder!(EntryRowBuilder, EntryRow, { text: String, editable: bool }, |row, b| {
    if let Some(t) = &b.text {
        row.set_text(t);
    }
    if let Some(e) = b.editable {
        row.set_editable(e);
    }
});

glib::wrapper! {
    /// An [`EntryRow`] that hides what is typed, with a button to show it.
    pub struct PasswordEntryRow(ObjectSubclass<entry_imp::PasswordEntryRow>)
        @extends gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget, gtk::Editable;
}

row_common!(PasswordEntryRow);
entry_common!(PasswordEntryRow, PasswordEntryRowBuilder);
row_builder!(PasswordEntryRowBuilder, PasswordEntryRow, { text: String }, |row, b| {
    if let Some(t) = &b.text {
        row.set_text(t);
    }
});

#[cfg(test)]
mod tests {
    use super::digits_for;

    #[test]
    fn digits_follow_the_step() {
        assert_eq!(digits_for(1.0), 0);
        assert_eq!(digits_for(0.5), 1);
        assert_eq!(digits_for(0.25), 2);
        assert_eq!(digits_for(f64::NAN), 0);
    }
}

// ---------------------------------------------------------------------------------------------
// ExpanderRow

mod expander_imp {
    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::ExpanderRow)]
    pub struct ExpanderRow {
        pub(crate) parts: OnceCell<Parts>,
        #[property(get, set = Self::set_expanded)]
        pub expanded: Cell<bool>,
        #[property(get, set = Self::set_enable_expansion)]
        pub enable_expansion: Cell<bool>,
        #[property(get, set = Self::set_show_enable_switch)]
        pub show_enable_switch: Cell<bool>,
        pub revealer: OnceCell<gtk::Revealer>,
        pub list: OnceCell<gtk::ListBox>,
        pub arrow: OnceCell<gtk::Image>,
        pub switch: OnceCell<gtk::Switch>,
    }

    impl ExpanderRow {
        fn set_expanded(&self, expanded: bool) {
            let expanded = expanded && self.enable_expansion.get();
            if self.expanded.replace(expanded) == expanded {
                return;
            }
            self.sync();
            self.obj().notify_expanded();
        }

        fn set_enable_expansion(&self, enable: bool) {
            if self.enable_expansion.replace(enable) == enable {
                return;
            }
            if !enable && self.expanded.replace(false) {
                self.obj().notify_expanded();
            }
            self.sync();
            self.obj().notify_enable_expansion();
        }

        fn set_show_enable_switch(&self, show: bool) {
            self.show_enable_switch.set(show);
            self.sync();
        }

        fn sync(&self) {
            let obj = self.obj();
            let expanded = self.expanded.get();
            if let Some(r) = self.revealer.get() {
                r.set_reveal_child(expanded);
            }
            if let Some(a) = self.arrow.get() {
                a.set_visible(!self.show_enable_switch.get());
                a.set_sensitive(self.enable_expansion.get());
            }
            if let Some(s) = self.switch.get() {
                s.set_visible(self.show_enable_switch.get());
            }
            if expanded {
                obj.add_css_class("expanded");
            } else {
                obj.remove_css_class("expanded");
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ExpanderRow {
        const NAME: &'static str = "NdlExpanderRow";
        type Type = super::ExpanderRow;
        type ParentType = gtk::ListBoxRow;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ExpanderRow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            self.enable_expansion.set(true);
            obj.add_css_class("ndl-expander-row");
            obj.set_activatable(false);
            let parts = Parts::new(obj.upcast_ref());
            // The header is the row's first line; the nested rows reveal below it.
            obj.set_child(gtk::Widget::NONE);
            let header_row = gtk::ListBoxRow::builder().child(&parts.header).build();
            header_row.add_css_class("ndl-expander-header");
            let header_list = gtk::ListBox::builder()
                .selection_mode(gtk::SelectionMode::None)
                .css_classes(["ndl-expander-title"])
                .build();
            header_list.append(&header_row);
            let weak = obj.downgrade();
            header_list.connect_row_activated(move |_, _| {
                if let Some(row) = weak.upgrade() {
                    row.set_expanded(!row.expanded());
                }
            });
            let switch = gtk::Switch::builder()
                .valign(gtk::Align::Center)
                .visible(false)
                .build();
            obj.bind_property("enable-expansion", &switch, "active")
                .bidirectional()
                .sync_create()
                .build();
            let arrow = gtk::Image::from_icon_name("pan-end-symbolic");
            arrow.add_css_class("ndl-expander-arrow");
            parts.add_suffix(&switch);
            parts.add_suffix(&arrow);
            let list = gtk::ListBox::builder()
                .selection_mode(gtk::SelectionMode::None)
                .css_classes(["ndl-expander-list"])
                .build();
            let revealer = gtk::Revealer::builder()
                .transition_type(gtk::RevealerTransitionType::SlideDown)
                .child(&list)
                .build();
            let body = gtk::Box::new(gtk::Orientation::Vertical, 0);
            body.append(&header_list);
            body.append(&revealer);
            obj.set_child(Some(&body));
            let _ = self.parts.set(parts);
            let _ = self.revealer.set(revealer);
            let _ = self.list.set(list);
            let _ = self.arrow.set(arrow);
            let _ = self.switch.set(switch);
            self.sync();
        }
    }

    impl WidgetImpl for ExpanderRow {}
    impl ListBoxRowImpl for ExpanderRow {}
}

glib::wrapper! {
    /// A row that opens to show more rows beneath it.
    pub struct ExpanderRow(ObjectSubclass<expander_imp::ExpanderRow>)
        @extends gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

row_builder!(ExpanderRowBuilder, ExpanderRow, { expanded: bool, show_enable_switch: bool, enable_expansion: bool }, |row, b| {
    if let Some(v) = b.show_enable_switch {
        row.set_show_enable_switch(v);
    }
    if let Some(v) = b.enable_expansion {
        row.set_enable_expansion(v);
    }
    if let Some(v) = b.expanded {
        row.set_expanded(v);
    }
});

impl Default for ExpanderRow {
    fn default() -> Self {
        Self::new()
    }
}

impl ExpanderRow {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn is_expanded(&self) -> bool {
        self.expanded()
    }

    pub fn builder() -> ExpanderRowBuilder {
        ExpanderRowBuilder::default()
    }

    fn parts(&self) -> &Parts {
        self.imp()
            .parts
            .get_or_init(|| Parts::new(self.upcast_ref()))
    }

    pub fn set_title(&self, title: &str) {
        self.parts().set_title(title);
    }

    pub fn title(&self) -> glib::GString {
        self.parts().title.label()
    }

    pub fn set_subtitle(&self, subtitle: &str) {
        self.parts().set_subtitle(subtitle);
    }

    pub fn subtitle(&self) -> glib::GString {
        self.parts().subtitle.label()
    }

    pub fn add_prefix(&self, widget: &impl IsA<gtk::Widget>) {
        self.parts().add_prefix(widget);
    }

    /// Before the arrow.
    pub fn add_suffix(&self, widget: &impl IsA<gtk::Widget>) {
        self.parts().suffixes.prepend(widget);
    }

    /// A nested row, shown while expanded.
    pub fn add_row(&self, row: &impl IsA<gtk::Widget>) {
        if let Some(l) = self.imp().list.get() {
            l.append(row);
        }
    }

    /// Takes away a nested row, a prefix or a suffix.
    pub fn remove(&self, widget: &impl IsA<gtk::Widget>) {
        let w = widget.as_ref();
        if let Some(l) = self.imp().list.get() {
            let row = if w.is::<gtk::ListBoxRow>() {
                Some(w.clone())
            } else {
                w.parent()
            };
            if let Some(row) = row.filter(|r| r.parent().as_ref() == Some(l.upcast_ref())) {
                l.remove(&row);
                return;
            }
        }
        self.parts().remove(widget);
    }

    pub fn enables_expansion(&self) -> bool {
        self.enable_expansion()
    }
}

// ---------------------------------------------------------------------------------------------
// PreferencesRow

mod preferences_imp {
    use super::*;

    /// What the row is called. It is not drawn: the row's child is all it shows, and the
    /// title is what a search over the rows and a screen reader find.
    #[derive(Default)]
    pub struct PreferencesRow {
        pub title: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PreferencesRow {
        const NAME: &'static str = "NdlPreferencesRow";
        type Type = super::PreferencesRow;
        type ParentType = gtk::ListBoxRow;
    }

    impl ObjectImpl for PreferencesRow {}
    impl WidgetImpl for PreferencesRow {}
    impl ListBoxRowImpl for PreferencesRow {}
}

glib::wrapper! {
    /// A row that shows only the child it is given, under a title kept for search and
    /// accessibility.
    pub struct PreferencesRow(ObjectSubclass<preferences_imp::PreferencesRow>)
        @extends gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for PreferencesRow {
    fn default() -> Self {
        Self::new()
    }
}

impl PreferencesRow {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_title(&self, title: &str) {
        self.update_property(&[gtk::accessible::Property::Label(title)]);
        self.imp().title.replace(title.to_owned());
    }

    pub fn title(&self) -> glib::GString {
        self.imp().title.borrow().as_str().into()
    }
}

/// The title of any NDL row, or `None` when `widget` is not one. For searching a list of rows
/// without knowing which kinds it holds.
pub fn row_title(widget: &gtk::Widget) -> Option<glib::GString> {
    macro_rules! try_rows {
        ($($ty:ty),*) => {
            $(if let Some(r) = widget.downcast_ref::<$ty>() {
                return Some(r.title());
            })*
        };
    }
    try_rows!(
        ActionRow,
        SwitchRow,
        ComboRow,
        SpinRow,
        ButtonRow,
        EntryRow,
        PasswordEntryRow,
        ExpanderRow
    );
    widget.downcast_ref::<PreferencesRow>().map(|r| r.title())
}
