//! Wrapper for pixman region operations.

use std::mem;

use libc::{c_float, c_int, c_uint};
use wlroots_sys::{wl_output_transform, wlr_region_expand, wlr_region_rotated_bounds,
                  wlr_region_scale, wlr_region_transform, pixman_box32_t, pixman_region32_clear,
                  pixman_region32_copy, pixman_region32_equal, pixman_region32_fini,
                  pixman_region32_init, pixman_region32_intersect, pixman_region32_not_empty,
                  pixman_region32_reset, pixman_region32_subtract, pixman_region32_t,
                  pixman_region32_translate, pixman_region32_union, pixman_region32_union_rect};

use {Area, Origin, Size};

/// A pixman region, used for damage tracking.
#[derive(Debug)]
pub struct PixmanRegion {
    pub region: pixman_region32_t
}

impl PixmanRegion {
    /// Make a new pixman region.
    pub fn new() -> Self {
        unsafe {
            // NOTE Rational for zeroed memory:
            // We are automatically filling it in with pixman_region32_init.
            let mut region = mem::zeroed();
            pixman_region32_init(&mut region);
            PixmanRegion { region }
        }
    }

    /// Scales a region, ie. multiplies all its coordinates by `scale`.
    ///
    /// The resulting coordinates are rounded up or down so that the new region is
    /// at least as big as the original one.
    pub fn scale(&mut self, scale: c_float) -> PixmanRegion {
        unsafe {
            // NOTE Rationale for zeroed:
            // This snippet is panic safe and will always be initlized by the union
            // function.
            let mut region: pixman_region32_t = mem::zeroed();
            wlr_region_scale(&mut region, &mut self.region, scale);
            PixmanRegion { region }
        }
    }

    /// Clear the region.
    pub fn clear(&mut self) {
        unsafe {
            pixman_region32_clear(&mut self.region);
        }
    }

    /// Reset the given area in the region.
    pub fn reset(&mut self, area: Area) {
        unsafe {
            let Area { origin: Origin { x, y },
                       size: Size { width, height } } = area;
            let mut pixman_box = pixman_box32_t { x1: x,
                                                  y1: y,
                                                  x2: x + width,
                                                  y2: y + height };
            pixman_region32_reset(&mut self.region, &mut pixman_box)
        }
    }

    /// Add a rectangle with the given dimensions to the area.
    pub fn rectangle(&mut self, x: c_int, y: c_int, width: c_uint, height: c_uint) {
        unsafe {
            let region_ptr = &mut self.region as *mut _;
            pixman_region32_union_rect(region_ptr, region_ptr, x, y, width, height);
        }
    }

    /// Applies a transform to a region inside a box of size `width` x `height`.
    pub fn transform(&mut self,
                     transform: wl_output_transform,
                     width: c_int,
                     height: c_int)
                     -> PixmanRegion {
        unsafe {
            // NOTE Rationale for zeroed:
            // This snippet is panic safe and will always be initlized by the union
            // function.
            let mut region: pixman_region32_t = mem::zeroed();
            wlr_region_transform(&mut region, &mut self.region, transform, width, height);
            PixmanRegion { region }
        }
    }

    /// Expands the region by `distance`. If `distance` is negative it shrinks
    /// the region.
    pub fn expand(&mut self, distance: c_int) -> PixmanRegion {
        unsafe {
            // NOTE Rationale for zeroed:
            // This snippet is panic safe and will always be initlized by the union
            // function.
            let mut region: pixman_region32_t = mem::zeroed();
            wlr_region_expand(&mut region, &mut self.region, distance);
            PixmanRegion { region }
        }
    }

    /// Builds the smallest possible region that contains the region rotated
    /// about the point (ox, oy).
    pub fn rotated_bounds(&mut self, rotation: c_float, ox: c_int, oy: c_int) -> PixmanRegion {
        unsafe {
            // NOTE Rationale for zeroed:
            // This snippet is panic safe and will always be initlized by the union
            // function.
            let mut region: pixman_region32_t = mem::zeroed();
            wlr_region_rotated_bounds(&mut region, &mut self.region, rotation, ox, oy);
            PixmanRegion { region }
        }
    }

    /// Translate the region using the given coordinates.
    pub fn translate(&mut self, x: c_int, y: c_int) {
        unsafe {
            pixman_region32_translate(&mut self.region, x, y);
        }
    }

    /// Subtract two pixman regions.
    pub fn subtract(&mut self, other: &mut PixmanRegion) -> PixmanRegion {
        unsafe {
            // NOTE Rationale for zeroed:
            // This snippet is panic safe and will always be initlized by the union
            // function.
            let mut region: pixman_region32_t = mem::zeroed();
            // TODO This returns a bool. Can this fail?
            pixman_region32_subtract(&mut region, &mut self.region, &mut other.region);
            PixmanRegion { region }
        }
    }

    /// Take the union of two pixman regions.
    pub fn union(&mut self, other: &mut PixmanRegion) -> PixmanRegion {
        unsafe {
            // NOTE Rationale for zeroed:
            // This snippet is panic safe and will always be initlized by the union
            // function.
            let mut region: pixman_region32_t = mem::zeroed();
            // TODO This returns a bool. Can this fail?
            pixman_region32_union(&mut region, &mut self.region, &mut other.region);
            PixmanRegion { region }
        }
    }

    /// Take the intersection of two pixman regions.
    pub fn intersect(&mut self, other: &mut PixmanRegion) -> PixmanRegion {
        unsafe {
            // NOTE Rationale for zeroed:
            // This snippet is panic safe and will always be initlized by the union
            // function.
            let mut region: pixman_region32_t = mem::zeroed();
            // TODO This returns a bool. Can this fail?
            pixman_region32_intersect(&mut region, &mut self.region, &mut other.region);
            PixmanRegion { region }
        }
    }

    /// Determine if the pixman is empty.
    pub fn not_empty(&self) -> bool {
        unsafe { pixman_region32_not_empty(&self.region as *const _ as *mut _) != 0 }
    }
}

impl PartialEq for PixmanRegion {
    fn eq(&self, other: &PixmanRegion) -> bool {
        unsafe {
            let self_ptr = &self.region as *const _ as *mut _;
            let other_ptr = &other.region as *const _ as *mut _;
            pixman_region32_equal(self_ptr, other_ptr) != 0
        }
    }
}

impl Eq for PixmanRegion {}

impl Clone for PixmanRegion {
    fn clone(&self) -> Self {
        unsafe {
            let mut region: pixman_region32_t = mem::zeroed();
            pixman_region32_copy(&mut region, &self.region as *const _ as *mut _);
            PixmanRegion { region }
        }
    }
}

impl Drop for PixmanRegion {
    fn drop(&mut self) {
        unsafe { pixman_region32_fini(&mut self.region) }
    }
}
