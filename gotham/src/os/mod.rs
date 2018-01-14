#[cfg(not(windows))]
pub mod unix;
#[cfg(not(windows))]
pub use self::unix as current;

#[cfg(windows)]
pub mod windows;
#[cfg(windows)]
pub use self::windows as current;
