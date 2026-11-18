//! Pages shown one at a time and moved between with a slide ([`Carousel`]), and the row of dots
//! that says which one is showing ([`CarouselIndicatorDots`]).

use std::cell::{Cell, OnceCell, RefCell};

use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

mod carousel_imp {
    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::Carousel)]
    pub struct Carousel {
        /// Which page shows, counted from 0.
        #[property(get)]
        pub position: Cell<f64>,
        /// Whether a swipe moves between pages.
        #[property(get, set)]
        pub interactive: Cell<bool>,
        pub stack: OnceCell<gtk::Stack>,
        pub pages: RefCell<Vec<gtk::Widget>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Carousel {
        const NAME: &'static str = "NdlCarousel";
        type Type = super::Carousel;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<gtk::BinLayout>();
            klass.set_css_name("ndl-carousel");
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for Carousel {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            self.interactive.set(true);
            let stack = gtk::Stack::builder()
                .transition_duration(250)
                .hhomogeneous(true)
                .vhomogeneous(false)
                .build();
            stack.set_parent(&*obj);
            let swipe = gtk::GestureSwipe::new();
            let weak = obj.downgrade();
            swipe.connect_swipe(move |_, vx, vy| {
                let Some(c) = weak.upgrade() else { return };
                if !c.interactive() || vx.abs() < vy.abs() || vx.abs() < 300.0 {
                    return;
                }
                let at = c.position() as i64 + if vx < 0.0 { 1 } else { -1 };
                if let Some(page) = u32::try_from(at).ok().and_then(|i| c.nth_page(i)) {
                    c.scroll_to(&page, true);
                }
            });
            obj.add_controller(swipe);
            let _ = self.stack.set(stack);
        }

        fn dispose(&self) {
            if let Some(s) = self.stack.get() {
                s.unparent();
            }
        }
    }

    impl WidgetImpl for Carousel {}
}

glib::wrapper! {
    /// Pages side by side, one on screen at a time.
    pub struct Carousel(ObjectSubclass<carousel_imp::Carousel>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for Carousel {
    fn default() -> Self {
        Self::new()
    }
}

impl Carousel {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn append(&self, page: &impl IsA<gtk::Widget>) {
        let imp = self.imp();
        if let Some(s) = imp.stack.get() {
            s.add_child(page);
        }
        imp.pages.borrow_mut().push(page.as_ref().clone());
    }

    pub fn remove(&self, page: &impl IsA<gtk::Widget>) {
        let imp = self.imp();
        if let Some(s) = imp.stack.get() {
            s.remove(page);
        }
        imp.pages.borrow_mut().retain(|p| p != page.as_ref());
    }

    pub fn n_pages(&self) -> u32 {
        u32::try_from(self.imp().pages.borrow().len()).unwrap_or(u32::MAX)
    }

    pub fn nth_page(&self, n: u32) -> Option<gtk::Widget> {
        self.imp().pages.borrow().get(n as usize).cloned()
    }

    /// Shows `page`, sliding to it when `animate`.
    pub fn scroll_to(&self, page: &impl IsA<gtk::Widget>, animate: bool) {
        let imp = self.imp();
        let Some(at) = imp.pages.borrow().iter().position(|p| p == page.as_ref()) else {
            return;
        };
        let Some(stack) = imp.stack.get() else { return };
        let forward = at as f64 >= imp.position.get();
        stack.set_transition_type(match (animate, forward) {
            (false, _) => gtk::StackTransitionType::None,
            (true, true) => gtk::StackTransitionType::SlideLeft,
            (true, false) => gtk::StackTransitionType::SlideRight,
        });
        stack.set_visible_child(page);
        imp.position.set(at as f64);
        self.notify_position();
    }
}

mod dots_imp {
    use super::*;

    #[derive(Default)]
    pub struct CarouselIndicatorDots {
        pub carousel: RefCell<Option<(glib::WeakRef<super::Carousel>, glib::SignalHandlerId)>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CarouselIndicatorDots {
        const NAME: &'static str = "NdlCarouselIndicatorDots";
        type Type = super::CarouselIndicatorDots;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<gtk::BoxLayout>();
            klass.set_css_name("ndl-carousel-dots");
        }
    }

    impl ObjectImpl for CarouselIndicatorDots {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            if let Some(l) = obj.layout_manager().and_downcast::<gtk::BoxLayout>() {
                l.set_spacing(8);
            }
            obj.set_halign(gtk::Align::Center);
        }

        fn dispose(&self) {
            while let Some(c) = self.obj().first_child() {
                c.unparent();
            }
        }
    }

    impl WidgetImpl for CarouselIndicatorDots {}
}

glib::wrapper! {
    /// One dot per page of a [`Carousel`], the current one filled.
    pub struct CarouselIndicatorDots(ObjectSubclass<dots_imp::CarouselIndicatorDots>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for CarouselIndicatorDots {
    fn default() -> Self {
        Self::new()
    }
}

impl CarouselIndicatorDots {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_carousel(&self, carousel: Option<&Carousel>) {
        if let Some((old, id)) = self.imp().carousel.take() {
            if let Some(old) = old.upgrade() {
                old.disconnect(id);
            }
        }
        if let Some(c) = carousel {
            let weak = self.downgrade();
            let id = c.connect_position_notify(move |_| {
                if let Some(d) = weak.upgrade() {
                    d.redraw();
                }
            });
            self.imp().carousel.replace(Some((c.downgrade(), id)));
        }
        self.redraw();
    }

    fn redraw(&self) {
        while let Some(c) = self.first_child() {
            c.unparent();
        }
        let carousel = self
            .imp()
            .carousel
            .borrow()
            .as_ref()
            .and_then(|(c, _)| c.upgrade());
        let Some(carousel) = carousel else { return };
        let at = carousel.position().round() as u32;
        for i in 0..carousel.n_pages() {
            let dot = gtk::Box::builder()
                .width_request(8)
                .height_request(8)
                .valign(gtk::Align::Center)
                .css_classes(["ndl-dot"])
                .build();
            if i == at {
                dot.add_css_class("current");
            }
            dot.set_parent(self);
        }
    }
}
