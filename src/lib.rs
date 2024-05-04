//! The [`ctor`] crate reimplemented using procedural macros.

#![no_std]

/// Run a function on program startup.
#[macro_export]
macro_rules! ctor {
    // Case 1: Run a function at startup time.
    (
        $(#[$meta:meta])*
        $vis:vis unsafe fn $name:ident () $bl:block
    ) => {
        const _: () = {
            $(#[$meta])*
            $vis unsafe fn $name () {
                unsafe fn __this_thing_is_always_unsafe() {}
                __this_thing_is_always_unsafe();
                $bl
            }

            #[cfg(not(any(
                target_os = "linux",
                target_os = "android",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd",
                target_os = "dragonfly",
                target_os = "illumos",
                target_os = "haiku",
                target_os = "macos",
                target_os = "ios",
                target_os = "visionos",
                target_os = "tvos",
                windows
            )))]
            compile_error!("ctor! is not supported on the current target");

            #[used]
            #[allow(non_upper_case_globals, non_snake_case)]
            #[doc(hidden)]
            #[cfg_attr(
                any(target_os = "linux", target_os = "android"),
                link_section = ".init_array"
            )]
            #[cfg_attr(target_os = "freebsd", link_section = ".init_array")]
            #[cfg_attr(target_os = "netbsd", link_section = ".init_array")]
            #[cfg_attr(target_os = "openbsd", link_section = ".init_array")]
            #[cfg_attr(target_os = "dragonfly", link_section = ".init_array")]
            #[cfg_attr(target_os = "illumos", link_section = ".init_array")]
            #[cfg_attr(target_os = "haiku", link_section = ".init_array")]
            #[cfg_attr(
                any(
                    target_os = "macos",
                    target_os = "ios",
                    target_os = "visionos",
                    target_os = "tvos"
                ), 
                link_section = "__DATA,__mod_init_func"
            )]
            #[cfg_attr(windows, link_section = ".CRT$XCU")]
            static __rust_ctor_lite__ctor: unsafe extern "C" fn() -> usize = {
                #[cfg_attr(
                    any(target_os = "linux", target_os = "android"),
                    link_section = ".text.startup"
                )]
                unsafe extern "C" fn ctor() -> usize {
                    $name ();
                    0
                }

                ctor
            };
        };
    };

    // Case 2: Initialize a constant at bootup time.
    (
        $(#[$meta:meta])*
        $vis:vis unsafe static $(mut)? $name:ident:$ty:ty = $e:expr;
    ) => {
        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        $vis struct $name<T> {
            _data: ::core::marker::PhantomData<T>
        }

        $(#[$meta:meta])*
        $vis static $name: $name<$ty> = $name {
            _data: ::core::marker::PhantomData::<$ty>
        };

        const _: () = {
            use ::core::cell::UnsafeCell;
            use ::core::mem::MaybeUninit;
            use ::core::ops::Deref;

            struct SyncSlot(UnsafeCell<MaybeUninit<$ty>>);
            unsafe impl Sync for SyncSlot {}

            static STORAGE: SyncSlot = {
                SyncSlot(UnsafeCell::new(MaybeUninit::uninit()))
            };

            impl Deref for $name<$ty> {
                type Target = $ty;

                fn deref(&self) -> &$ty {
                    // SAFETY: This will always be initialized.
                    unsafe {
                        &*(&*STORAGE.0.get()).as_ptr()
                    }
                }
            }

            $crate::ctor! {
                unsafe fn init_storage() {
                    let val = $e;

                    // SAFETY: We are the only ones who can write into STORAGE.
                    unsafe {
                        *STORAGE.0.get() = MaybeUninit::new(val);
                    }
                }
            }

            fn __assert_type_is_sync() {
                fn __must_be_sync<T: Sync>() {}
                __must_be_sync::<$ty>();
            }
        };
    }
}

/// Run a function on program shutdown.
#[macro_export]
macro_rules! dtor {
    (
        $(#[$meta:meta])*
        $vis:vis unsafe fn $name:ident () $bl:block
    ) => {
        const _: () = {
            $(#[$meta])*
            $vis unsafe fn $name () {
                unsafe fn __this_thing_is_always_unsafe() {}
                __this_thing_is_always_unsafe();
                $bl
            }

            // Link directly to atexit in order to avoid a libc dependency.
            #[cfg(not(any(
                target_os = "macos",
                target_os = "ios",
                target_os = "visionos",
                target_os = "tvos"
            )))]
            #[inline(always)]
            unsafe fn __do_atexit(cb: unsafe extern fn()) {
                extern "C" {
                    fn atexit(cb: unsafe extern fn());
                }
                atexit(cb);
            }

            // For platforms that have __cxa_atexit, we register the dtor as scoped to dso_handle
            #[cfg(any(
                target_os = "macos",
                target_os = "ios",
                target_os = "visionos",
                target_os = "tvos"
            ))]
            #[inline(always)]
            unsafe fn __do_atexit(cb: unsafe extern fn(_: *const u8)) {
                extern "C" {
                    static __dso_handle: *const u8;
                    fn __cxa_atexit(
                        cb: unsafe extern fn(_: *const u8),
                        arg: *const u8,
                        dso_handle: *const u8
                    );
                }
                __cxa_atexit(cb, ::core::ptr::null(), __dso_handle);
            }

            #[cfg(not(any(
                target_os = "macos",
                target_os = "ios",
                target_os = "visionos",
                target_os = "tvos"
            )))]
            #[cfg_attr(
                any(
                    target_os = "linux",
                    target_os = "android"
                ),
                link_section = ".text.exit"
            )]
            unsafe extern "C" fn __run_destructor() { $name() };
            #[cfg(any(
                target_os = "macos",
                target_os = "ios",
                target_os = "visionos",
                target_os = "tvos"
            ))]
            unsafe extern "C" fn __run_destructor(_: *const u8) { $name() };

            $crate::ctor! {
                unsafe fn register_dtor() {
                    __do_atexit(__run_destructor);
                }
            }
        };
    };
}
