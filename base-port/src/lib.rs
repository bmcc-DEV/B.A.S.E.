//! Port package — mapa de endereços/drivers, inventário de fósseis, atlas MD.
//!
//! **Não** reescreve o OS completo. Gera o que o engenheiro precisa para *não*
//! começar do zero ao mapear HAL/drivers entre arquiteturas.

mod fossils;
mod map;
mod package;
mod platform;
mod usb_cross;
mod usb_probe;

pub use fossils::{FossilInventory, FossilKind, FossilRecord};
pub use map::{AddressDriverMap, MappedRegion, TranslationStrategy};
pub use package::{build_port_package, PortPackage, PortPackageOptions};
pub use platform::{
    build_platform_from_dtb_bytes, build_platform_from_dtb_info, build_platform_from_path,
    extract_fdt_blobs, DiscoveryStatus, PlatformInventory,
};
pub use usb_probe::{run_usb_hw_probe, UsbHwInventory, UsbProbeMode, UsbProbeOptions};
pub use usb_cross::{cross_usb_dt, cross_usb_dt_files, UsbDtCrossReport};
