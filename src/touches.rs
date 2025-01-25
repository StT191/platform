
use winit::{event::{Touch, TouchPhase, Force}, dpi::PhysicalPosition};
use std::{slice, iter};


pub type TouchPos = glam::Vec2;


#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TouchRegister<T: 'static = ()> {
    id: u64,
    ended: bool,
    pub ext: T,
    pub ref_location: TouchPos,
    location: TouchPos,
    pub ref_force: Option<f32>,
    force: Option<f32>,
}

impl<T> TouchRegister<T> {

    pub fn id(&self) -> u64 { self.id }
    pub fn ended(&self) -> bool { self.ended }
    pub fn location(&self) -> TouchPos { self.location }
    pub fn force(&self) -> Option<f32> { self.force }

    pub fn reset_deltas(&mut self) {
        self.ref_location = self.location;
        self.ref_force = self.force;
    }

    pub fn delta_location(&self) -> TouchPos { self.location - self.ref_location }

    pub fn diff_angle(&self, other: &Self) -> f32 { self.location.diff_angle(other.location) }
    pub fn diff_ref_angle(&self, other: &Self) -> f32 { self.ref_location.diff_angle(other.ref_location) }
    pub fn delta_diff_angle(&self, other: &Self) -> f32 { self.diff_angle(other) - self.diff_ref_angle(other) }


    pub fn mean_location<'a>(items: impl IntoIterator<Item=&'a Self>) -> TouchPos {
        TouchPos::mean(items.into_iter().map(|reg| reg.location))
    }

    pub fn mean_ref_location<'a>(items: impl IntoIterator<Item=&'a Self>) -> TouchPos {
        TouchPos::mean(items.into_iter().map(|reg| reg.ref_location))
    }

    pub fn mean_delta_location<'a>(items: impl IntoIterator<Item=&'a Self>) -> TouchPos {
        TouchPos::mean(items.into_iter().map(|reg| reg.delta_location()))
    }

    pub fn spread<'a>(items: impl IntoIterator<Item=&'a Self, IntoIter:Clone>) -> f32 {
        TouchPos::spread(items.into_iter().map(|reg| reg.location))
    }

    pub fn ref_spread<'a>(items: impl IntoIterator<Item=&'a Self, IntoIter:Clone>) -> f32 {
        TouchPos::spread(items.into_iter().map(|reg| reg.ref_location))
    }

    pub fn delta_spread<'a>(items: impl IntoIterator<Item=&'a Self, IntoIter:Clone>) -> f32 {
        let items = items.into_iter();
        Self::spread(items.clone()) - Self::ref_spread(items)
    }
}


pub trait TouchExt {
    fn new(id: u64, location: TouchPos, force: Option<f32>) -> Self;
}

impl TouchExt for () {
    fn new(_id: u64, _location: TouchPos, _force: Option<f32>) {}
}


#[derive(Debug, Clone, PartialEq)]
pub struct Touches<
    T: TouchExt + 'static = (),
    const SHRINK_TO: usize = {usize::MAX},
    const MAX: usize = {usize::MAX},
> {
    pub vec: Vec<TouchRegister<T>>,
}

impl<T: TouchExt + 'static, const SHRINK_TO:usize, const MAX:usize> Touches<T, SHRINK_TO, MAX> {

    #[allow(clippy::new_without_default)]
    pub fn new() -> Self { Self { vec: Vec::new() } }

    pub fn len(&self) -> usize { self.vec.len() }
    pub fn is_empty(&self) -> bool { self.vec.is_empty() }

    pub fn ended(&self) -> usize { self.iter().filter(|reg| reg.ended).count() }

    pub fn clear(&mut self, mut pred: impl FnMut(&TouchRegister<T>) -> bool) {
        self.vec.retain(|reg| !pred(reg));
        self.vec.shrink_to(SHRINK_TO);
    }

    pub fn reset_deltas(&mut self) { self.vec.iter_mut().for_each(|reg| reg.reset_deltas()) }

    pub fn reset(&mut self) {
        self.clear(|reg| reg.ended);
        self.reset_deltas();
    }

    pub fn iter(&self) -> slice::Iter<'_, TouchRegister<T>> { self.vec.iter() }
    pub fn iter_mut(&mut self) -> slice::IterMut<'_, TouchRegister<T>> { self.vec.iter_mut() }

    pub fn by_id<'a>(&'a mut self, id: u64)
        -> iter::Filter<slice::IterMut<'a, TouchRegister<T>>, impl FnMut(&&'a mut TouchRegister<T>) -> bool>
    {
        self.iter_mut().filter(move |reg| reg.id == id)
    }

    pub fn update(&mut self, touch: Touch) {

        match touch.phase {

            TouchPhase::Started => {

                // end previous, if id exists
                self.by_id(touch.id).for_each(|reg| reg.ended = true);

                let location = TouchPos::from_physical_position(touch.location);
                let force = touch.force.map(normalize_force);

                if self.vec.len() < MAX {
                    self.vec.push(TouchRegister {
                        id: touch.id, ended: false,
                        ext: T::new(touch.id, location, force),
                        ref_location: location, location,
                        ref_force: force, force,
                    });
                }
            },

            TouchPhase::Moved => {
                self.by_id(touch.id).for_each(|reg| {
                    if !reg.ended {
                        reg.location = TouchPos::from_physical_position(touch.location);
                        reg.force = touch.force.map(normalize_force);
                    }
                });
            },

            TouchPhase::Ended | TouchPhase::Cancelled => {
                self.by_id(touch.id).for_each(|reg| {
                    if !reg.ended {
                        reg.location = TouchPos::from_physical_position(touch.location);
                        reg.force = touch.force.map(normalize_force);
                    }
                    reg.ended = true;
                });
            },
        }
    }
}


// helper

fn normalize_force(force: Force) -> f32 {
    match force {
        Force::Normalized(f) => f as f32,
        Force::Calibrated { force, max_possible_force, .. } => {
            force as f32 / max_possible_force as f32
        },
    }
}


use std::f32::consts::PI;

pub trait AngleExt {
    fn normalize_angle(self) -> Self;
    fn norm_angle_as_delta(self) -> Self;
    fn angle_as_delta(self) -> Self;
}

impl AngleExt for f32 {
    fn normalize_angle(self) -> Self { self.rem_euclid(2.0 * PI) }
    fn norm_angle_as_delta(self) -> Self { if self > PI { self - 2.0*PI } else { self } }
    fn angle_as_delta(self) -> Self { Self::norm_angle_as_delta(Self::normalize_angle(self)) }
}


pub trait TouchPosExt: Sized {

    fn from_physical_position(pos: PhysicalPosition<f64>) -> Self;

    fn diff_angle(self, other: Self) -> f32;

    fn norm_arc_slice(self, parts: f32) -> Self;
    fn project_onto_norm_arc_slice(self, parts: f32) -> Self;

    fn filtered_norm_arc_slice(self, arc_parts: f32, filter: f32) -> Option<Self>;
    fn project_onto_filtered_norm_arc_slice(self, arc_parts: f32, filter: f32) -> Option<Self>;

    fn mean(items: impl IntoIterator<Item=Self>) -> Self;

    fn spread(items: impl IntoIterator<Item=Self, IntoIter:Clone>) -> f32;
}

impl TouchPosExt for TouchPos {

    fn from_physical_position(pos: PhysicalPosition<f64>) -> Self {
        let [x, y]: [f64; 2] = pos.into();
        Self::new(x as f32, y as f32)
    }

    fn diff_angle(self, other: Self) -> f32 { (other - self).to_angle() }

    fn norm_arc_slice(self, arc_parts: f32) -> Self {
        let w0 = PI / arc_parts;
        let ratio = (self.to_angle() / w0).round();
        Self::from_angle(ratio * w0)
    }

    fn project_onto_norm_arc_slice(self, arc_parts: f32) -> Self {
        self.project_onto(self.norm_arc_slice(arc_parts))
    }

    fn filtered_norm_arc_slice(self, arc_parts: f32, filter: f32) -> Option<Self> {
        let w0 = PI / arc_parts;
        let full_ratio = self.to_angle() / w0;
        let ratio = full_ratio.round();
        if (full_ratio - ratio).abs() <= filter/2.0 {
            Some(Self::from_angle(ratio * w0))
        }
        else { None }
    }

    fn project_onto_filtered_norm_arc_slice(self, arc_parts: f32, filter: f32) -> Option<Self> {
        self.filtered_norm_arc_slice(arc_parts, filter).map(|slice| self.project_onto(slice))
    }

    fn mean(items: impl IntoIterator<Item=Self>) -> Self {
        let (mut n, mut sum) = (0, Self::new(0.0, 0.0));
        for itm in items {
            n += 1;
            sum += itm;
        }
        if n < 2 { sum } else { sum / n as f32 }
    }

    fn spread(items: impl IntoIterator<Item=Self, IntoIter:Clone>) -> f32 {
        let mut items = items.into_iter();
        let (mut n, mut sum) = (0, 0.0);
        while let Some(itm) = items.next() {
            for other in items.clone() {
                n += 1;
                sum += itm.distance(other);
            }
        }
        if n < 2 { sum } else { sum / n as f32 }
    }
}