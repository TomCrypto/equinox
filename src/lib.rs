//! The Equinox stochastic photon mapper, see the README for more information.

#![deny(unsafe_code)]

macro_rules! export {
    [$( $module:ident ),+ $(,)?] => {
        $(
            mod $module;
            pub use self::$module::*;
        )+
    };
}

export![device, engine, scene, web];

/// GLSL shaders.
pub mod shaders {
    include!(concat!(env!("OUT_DIR"), "/glsl_shaders.rs"));
}
