//! Manager that is called when a seat is created or destroyed.
//! Pass a struct that implements this trait to the `Compositor` during
//! initialization.

use libc;

use std::{env, panic};
use std::process::abort;

use super::{KeyboardHandler, KeyboardWrapper, PointerHandler, PointerWrapper, TabletPadHandler,
            TabletPadWrapper, TabletToolHandler, TabletToolWrapper, TouchHandler, TouchWrapper};
use compositor::{compositor_handle, CompositorHandle};
use types::input::{InputDevice, Keyboard, KeyboardHandle, Pointer, PointerHandle, TabletPad,
                   TabletPadHandle, TabletTool, TabletToolHandle, Touch, TouchHandle};
use utils::safe_as_cstring;

use wayland_sys::server::signal::wl_signal_add;
use wlroots_sys::{wlr_input_device, wlr_input_device_type, wlr_keyboard_set_keymap,
                  wlr_keyboard_set_repeat_info, xkb_context_new, xkb_context_unref,
                  xkb_keymap_new_from_names, xkb_keymap_unref, xkb_rule_names};
use wlroots_sys::xkb_context_flags::*;
use wlroots_sys::xkb_keymap_compile_flags::*;

/// Handles input addition and removal.
pub trait InputManagerHandler {
    /// Callback triggered when an input device is added.
    ///
    /// # Panics
    /// Any panic in this function will cause the process to abort.
    fn input_added(&mut self, CompositorHandle, &mut InputDevice) {}

    /// Callback triggered when a keyboard device is added.
    ///
    /// # Panics
    /// Any panic in this function will cause the process to abort.
    fn keyboard_added(&mut self, CompositorHandle, KeyboardHandle) -> Option<Box<KeyboardHandler>> {
        None
    }

    /// Callback triggered when a pointer device is added.
    ///
    /// # Panics
    /// Any panic in this function will cause the process to abort.
    fn pointer_added(&mut self, CompositorHandle, PointerHandle) -> Option<Box<PointerHandler>> {
        None
    }

    /// Callback triggered when a touch device is added.
    ///
    /// # Panics
    /// Any panic in this function will cause the process to abort.
    fn touch_added(&mut self, CompositorHandle, TouchHandle) -> Option<Box<TouchHandler>> {
        None
    }

    /// Callback triggered when a tablet tool is added.
    ///
    ///
    /// # Panics
    /// Any panic in this function will cause the process to abort.
    fn tablet_tool_added(&mut self,
                         CompositorHandle,
                         TabletToolHandle)
                         -> Option<Box<TabletToolHandler>> {
        None
    }

    /// Callback triggered when a tablet pad is added.
    ///
    ///
    /// # Panics
    /// Any panic in this function will cause the process to abort.
    fn tablet_pad_added(&mut self,
                        CompositorHandle,
                        TabletPadHandle)
                        -> Option<Box<TabletPadHandler>> {
        None
    }
}

wayland_listener!(InputManager, Box<InputManagerHandler>, [
    add_listener => add_notify: |this: &mut InputManager, data: *mut libc::c_void,| unsafe {
        let compositor = match compositor_handle() {
            Some(handle) => handle,
            None => return
        };
        let data = data as *mut wlr_input_device;
        let ref mut manager = this.data;
        use self::wlr_input_device_type::*;
        let mut dev = InputDevice::from_ptr(data);
        let res = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            match dev.dev_type() {
                WLR_INPUT_DEVICE_KEYBOARD => {
                    // Boring setup that we won't make the user do
                    add_keyboard(&mut dev);
                    let mut keyboard = match Keyboard::new_from_input_device(data) {
                        Some(dev) => dev,
                        None => {
                            wlr_log!(L_ERROR, "Device {:#?} was not a keyboard!", dev);
                            abort()
                        }
                    };
                    let keyboard_handle = keyboard.weak_reference();
                    if let Some(keyboard_handler) = manager.keyboard_added(compositor.clone(),
                                                                           keyboard_handle) {
                        let mut keyboard = KeyboardWrapper::new((keyboard,
                                                                 keyboard_handler));
                        wl_signal_add(&mut (*dev.dev_union().keyboard).events.key as *mut _ as _,
                                    keyboard.key_listener() as *mut _ as _);
                        wl_signal_add(&mut (*dev.dev_union().keyboard).events.modifiers
                                      as *mut _ as _,
                                      keyboard.modifiers_listener() as *mut _ as _);
                        wl_signal_add(&mut (*dev.dev_union().keyboard).events.keymap as *mut _ as _,
                                      keyboard.keymap_listener() as *mut _ as _);
                        wl_signal_add(&mut (*dev.dev_union().keyboard).events.repeat_info
                                      as *mut _ as _,
                                      keyboard.repeat_listener() as *mut _ as _);
                        wl_signal_add(&mut (*dev.as_ptr()).events.destroy as *mut _ as _,
                                      keyboard.on_destroy_listener() as _);
                        (*data).data = Box::into_raw(keyboard) as _;
                    }
                },
                WLR_INPUT_DEVICE_POINTER => {
                    let pointer = match Pointer::new_from_input_device(data) {
                        Some(dev) => dev,
                        None => {
                            wlr_log!(L_ERROR, "Device {:#?} was not a pointer!", dev);
                            abort()
                        }
                    };
                    let pointer_handle = pointer.weak_reference();
                    if let Some(pointer_handler) = manager.pointer_added(compositor.clone(),
                                                                         pointer_handle) {
                        let mut pointer = PointerWrapper::new((pointer, pointer_handler));
                        wl_signal_add(&mut (*dev.dev_union().pointer).events.motion as *mut _ as _,
                                    pointer.motion_listener() as *mut _ as _);
                        wl_signal_add(&mut (*dev.dev_union().pointer)
                                      .events.motion_absolute as *mut _ as _,
                                    pointer.motion_absolute_listener() as *mut _ as _);
                        wl_signal_add(&mut (*dev.dev_union().pointer).events.button as *mut _ as _,
                                    pointer.button_listener() as *mut _ as _);
                        wl_signal_add(&mut (*dev.dev_union().pointer).events.axis as *mut _ as _,
                                    pointer.axis_listener() as *mut _ as _);
                        wl_signal_add(&mut (*dev.as_ptr()).events.destroy as *mut _ as _,
                                      pointer.on_destroy_listener() as _);
                        (*data).data = Box::into_raw(pointer) as _;
                    }
                },
                WLR_INPUT_DEVICE_TOUCH => {
                    let touch = match Touch::new_from_input_device(data) {
                        Some(dev) => dev,
                        None => {
                            wlr_log!(L_ERROR, "Device {:#?} was not a touch", dev);
                            abort()
                        }
                    };
                    let touch_handle = touch.weak_reference();
                    if let Some(touch_handler) = manager.touch_added(compositor.clone(),
                                                                     touch_handle) {
                        let mut touch = TouchWrapper::new((touch, touch_handler));
                        wl_signal_add(&mut (*dev.dev_union().touch).events.down as *mut _ as _,
                                      touch.down_listener() as *mut _ as _);
                        wl_signal_add(&mut (*dev.dev_union().touch).events.up as *mut _ as _,
                                      touch.up_listener() as *mut _ as _);
                        wl_signal_add(&mut (*dev.dev_union().touch).events.motion as *mut _ as _,
                                      touch.motion_listener() as *mut _ as _);
                        wl_signal_add(&mut (*dev.dev_union().touch).events.cancel as *mut _ as _,
                                      touch.cancel_listener() as *mut _ as _);
                        wl_signal_add(&mut (*dev.as_ptr()).events.destroy as *mut _ as _,
                                      touch.on_destroy_listener() as _);
                        (*data).data = Box::into_raw(touch) as _;
                    }
                },
                WLR_INPUT_DEVICE_TABLET_TOOL => {
                    let tablet_tool = match TabletTool::new_from_input_device(data) {
                        Some(dev) => dev,
                        None => {
                            wlr_log!(L_ERROR, "Device {:#?}, was not a tablet tool", dev);
                            abort()
                        }
                    };
                    let tablet_tool_handle = tablet_tool.weak_reference();
                    if let Some(tablet_tool_handler) = manager.tablet_tool_added(compositor.clone(),
                                                                         tablet_tool_handle) {
                        let mut tablet_tool = TabletToolWrapper::new((tablet_tool,
                                                                      tablet_tool_handler));
                        let tool_ptr = &mut (*dev.dev_union().tablet_tool);
                        wl_signal_add(&mut tool_ptr.events.axis as *mut _ as _,
                                      tablet_tool.axis_listener() as *mut _ as _);
                        wl_signal_add(&mut tool_ptr.events.proximity as *mut _ as _,
                                      tablet_tool.proximity_listener() as *mut _ as _);
                        wl_signal_add(&mut tool_ptr.events.tip as *mut _ as _,
                                      tablet_tool.tip_listener() as *mut _ as _);
                        wl_signal_add(&mut tool_ptr.events.button as *mut _ as _,
                                      tablet_tool.button_listener() as *mut _ as _);
                        wl_signal_add(&mut (*dev.as_ptr()).events.destroy as *mut _ as _,
                                      tablet_tool.on_destroy_listener() as _);
                        (*data).data = Box::into_raw(tablet_tool) as _;
                    }
                },
                WLR_INPUT_DEVICE_TABLET_PAD => {
                    let tablet_pad = match TabletPad::new_from_input_device(data) {
                        Some(dev) => dev,
                        None => {
                            wlr_log!(L_ERROR, "Device {:#?}, was not a tablet pad", dev);
                            abort()
                        }
                    };
                    let tablet_pad_handle = tablet_pad.weak_reference();
                    if let Some(tablet_pad_handler) = manager.tablet_pad_added(compositor.clone(),
                                                                       tablet_pad_handle) {
                        let mut tablet_pad = TabletPadWrapper::new((tablet_pad,
                                                                    tablet_pad_handler));
                        let pad_ptr = &mut (*dev.dev_union().tablet_pad);
                        wl_signal_add(&mut pad_ptr.events.button as *mut _ as _,
                                      tablet_pad.button_listener() as *mut _ as _);;
                        wl_signal_add(&mut pad_ptr.events.ring as *mut _ as _,
                                      tablet_pad.ring_listener() as *mut _ as _);;
                        wl_signal_add(&mut pad_ptr.events.strip as *mut _ as _,
                                      tablet_pad.strip_listener() as *mut _ as _);;
                        wl_signal_add(&mut (*dev.as_ptr()).events.destroy as *mut _ as _,
                                      tablet_pad.on_destroy_listener() as _);
                        (*data).data = Box::into_raw(tablet_pad) as _;
                    }
                }
            }
            manager.input_added(compositor, &mut dev)
        }));
        match res {
            Ok(_) => {},
            // NOTE
            // Either Wayland or wlroots does not handle failure to set up input correctly.
            // Calling wl_display_terminate does not work if input is incorrectly set up.
            //
            // Instead, execution keeps going with an eventual segfault (if lucky).
            //
            // To fix this, we abort the process if there was a panic in input setup.
            Err(_) => abort()
        }
    };
]);

pub(crate) unsafe fn add_keyboard(dev: &mut InputDevice) {
    // Set the XKB settings
    let rules = safe_as_cstring(env::var("XKB_DEFAULT_RULES").unwrap_or("".into()));
    let model = safe_as_cstring(env::var("XKB_DEFAULT_MODEL").unwrap_or("".into()));
    let layout = safe_as_cstring(env::var("XKB_DEFAULT_LAYOUT").unwrap_or("".into()));
    let variant = safe_as_cstring(env::var("XKB_DEFAULT_VARIANT").unwrap_or("".into()));
    let options = safe_as_cstring(env::var("XKB_DEFAULT_OPTIONS").unwrap_or("".into()));
    wlr_log!(L_DEBUG, "Using xkb rules: {:?}", rules);
    wlr_log!(L_DEBUG, "Using xkb model: {:?}", model);
    wlr_log!(L_DEBUG, "Using xkb layout: {:?}", layout);
    wlr_log!(L_DEBUG, "Using xkb variant: {:?}", variant);
    wlr_log!(L_DEBUG, "Using xkb options: {:?}", options);
    let rules = xkb_rule_names { rules: rules.into_raw(),
                                 model: model.into_raw(),
                                 layout: layout.into_raw(),
                                 variant: variant.into_raw(),
                                 options: options.into_raw() };
    let context = xkb_context_new(XKB_CONTEXT_NO_FLAGS);
    if context.is_null() {
        panic!("Failed to create XKB context");
    }
    let xkb_map = xkb_keymap_new_from_names(context, &rules, XKB_KEYMAP_COMPILE_NO_FLAGS);
    if xkb_map.is_null() {
        panic!("Could not create xkb map");
    }
    wlr_keyboard_set_keymap(dev.dev_union().keyboard, xkb_map);
    xkb_keymap_unref(xkb_map);
    xkb_context_unref(context);
    wlr_keyboard_set_repeat_info(dev.dev_union().keyboard, 25, 600);
}
