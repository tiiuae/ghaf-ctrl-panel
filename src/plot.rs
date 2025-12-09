use crate::serie::Serie;
use gtk::{glib, subclass::prelude::*};

pub type Formatter = Box<dyn Fn(f32) -> String>;

mod imp {
    use crate::serie::Serie;
    use glib::{Object, Properties};
    use gtk::{Builder, cairo, glib, prelude::*, subclass::prelude::*};
    use std::cell::{Cell, RefCell};

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::Plot)]
    #[allow(clippy::similar_names)]
    pub struct Plot {
        #[property(get)]
        minx: Cell<f32>,
        #[property(get)]
        maxx: Cell<f32>,
        #[property(get)]
        miny: Cell<f32>,
        #[property(get)]
        maxy: Cell<f32>,

        fixed_miny: Cell<Option<f32>>,
        fixed_maxy: Cell<Option<f32>>,
        fixed_minx: Cell<Option<f32>>,
        fixed_maxx: Cell<Option<f32>>,
        label_format: RefCell<Option<super::Formatter>>,
        series: RefCell<Vec<Serie>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Plot {
        const NAME: &'static str = "Plot";
        type Type = super::Plot;
        type ParentType = gtk::DrawingArea;
        type Interfaces = (gtk::Buildable,);

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("plot");
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for Plot {
        fn constructed(&self) {
            self.parent_constructed();

            self.obj().set_draw_func(glib::clone!(
                #[strong(rename_to = plot)]
                self.obj(),
                move |_, context, width, height| plot.imp().draw(context, width, height)
            ));
        }
    }

    impl WidgetImpl for Plot {}
    impl DrawingAreaImpl for Plot {}

    impl BuildableImpl for Plot {
        fn add_child(&self, builder: &Builder, child: &Object, type_: Option<&str>) {
            println!("add_child()");
            if let Some(serie) = child.downcast_ref() {
                self.add_serie(serie);
                println!("add_child({})", serie.color());
            } else {
                self.parent_add_child(builder, child, type_);
            }
        }
    }

    impl Plot {
        #[allow(clippy::similar_names)]
        #[allow(clippy::cast_lossless)]
        #[allow(clippy::cast_possible_truncation)]
        fn draw(&self, context: &cairo::Context, width: i32, height: i32) {
            let c = self.obj().color();
            let w = width as f64;
            let h = height as f64;

            let Some((minx, maxx, miny, maxy)) =
                self.series.borrow().iter().fold(None, |acc, cur| {
                    cur.values().fold(acc, |acc, (x, y)| {
                        Some(acc.map_or((x, x, y, y), |(xn, xx, yn, yx)| {
                            (x.min(xn), x.max(xx), y.min(yn), y.max(yx))
                        }))
                    })
                })
            else {
                return;
            };

            let minx = self.fixed_minx.get().unwrap_or(minx) as f64;
            let maxx = self.fixed_maxx.get().unwrap_or(maxx) as f64;
            let miny = self.fixed_miny.get().unwrap_or(miny) as f64;
            let maxy = self.fixed_maxy.get().unwrap_or(maxy) as f64;

            let xscale = if maxx > minx { w / (maxx - minx) } else { 1.0 };
            let yscale = if maxy > miny { h / (maxy - miny) } else { 1.0 };
            let matrix =
                cairo::Matrix::new(xscale, 0., 0., -yscale, -minx * xscale, h + miny * yscale);

            for serie in self.series.borrow().iter() {
                let mut iter = serie.values().map(|(x, y)| (x as f64, y as f64));
                let c = serie.actual_color().unwrap_or(c);

                let Some((x0, y0)) = iter.next() else {
                    continue;
                };

                context.set_source_color(&c);
                context.set_matrix(matrix);
                context.move_to(x0, y0);

                for (x, y) in iter {
                    context.line_to(x, y);
                }

                context.identity_matrix();
                context.stroke_preserve().ok();

                context.line_to(context.current_point().unwrap().0, h);
                context.line_to(0., h);

                context.set_source_rgba(c.red().into(), c.green().into(), c.blue().into(), 0.5);
                context.fill().ok();

                context.set_operator(cairo::Operator::Source);
            }

            context.set_operator(cairo::Operator::Over);
            let label = self
                .label_format
                .borrow()
                .as_ref()
                .map_or_else(|| format!("{miny}"), |f| f(miny as f32));
            let layout = self.obj().create_pango_layout(Some(&label));
            context.set_source_color(&c);
            context.move_to(0., h - layout.pixel_size().1 as f64);
            pangocairo::functions::show_layout(context, &layout);

            let label = self
                .label_format
                .borrow()
                .as_ref()
                .map_or_else(|| format!("{maxy}"), |f| f(maxy as f32));
            let layout = self.obj().create_pango_layout(Some(&label));
            context.move_to(0., 0.);
            pangocairo::functions::show_layout(context, &layout);
        }

        pub fn add_serie(&self, serie: &Serie) {
            self.series.borrow_mut().push(serie.clone());
            serie.connect_local(
                "changed",
                true,
                glib::clone!(
                    #[strong(rename_to = plot)]
                    self.obj(),
                    move |_| {
                        plot.queue_draw();
                        None
                    }
                ),
            );
        }

        #[allow(clippy::similar_names)]
        pub fn set_view(
            &self,
            minx: Option<f32>,
            maxx: Option<f32>,
            miny: Option<f32>,
            maxy: Option<f32>,
        ) {
            self.fixed_minx.replace(minx);
            self.fixed_maxx.replace(maxx);
            self.fixed_miny.replace(miny);
            self.fixed_maxy.replace(maxy);
        }

        pub fn set_label_format(&self, f: super::Formatter) {
            *self.label_format.borrow_mut() = Some(f);
        }
    }
}

glib::wrapper! {
    pub struct Plot(ObjectSubclass<imp::Plot>)
        @extends gtk::Widget, gtk::DrawingArea,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for Plot {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl Plot {
    pub fn add_serie(&self, serie: &Serie) {
        println!("Added serie with color {}", serie.color());
        self.imp().add_serie(serie);
    }

    #[allow(clippy::similar_names)]
    pub fn set_view(
        &self,
        minx: Option<f32>,
        maxx: Option<f32>,
        miny: Option<f32>,
        maxy: Option<f32>,
    ) {
        self.imp().set_view(minx, maxx, miny, maxy);
    }

    pub fn set_label_format<F: Fn(f32) -> String + 'static>(&self, f: F) {
        self.imp().set_label_format(Box::new(f));
    }
}
