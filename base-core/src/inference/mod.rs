pub mod extraction;
pub mod protocol;
pub mod fsm;
pub mod generator;

pub use extraction::{extract_blocks, BlockCluster, BlockType, MmioAccess, MmioAccessType, RawRegister};
pub use fsm::{extract_fsm, fsm_to_protocol, InferredFsm};
pub use generator::generate_spec;
pub use protocol::{heuristic_register_name, infer_protocol, InferredProtocol, RegisterRole};
