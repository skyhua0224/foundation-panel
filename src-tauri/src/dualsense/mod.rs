//! Native DualSense support of the Foundation host.
//!
//! The host builds the DualSense path into itself; its only external
//! dependency is the signed usbip-win2 transport. The Panel therefore installs
//! nothing for it. It reads what the host and the system report (`status`) and
//! drives the host's tuning profile (`tuning`, schema ALK-DS5T/1).

mod status;
mod tuning;

pub use status::*;
pub use tuning::*;
