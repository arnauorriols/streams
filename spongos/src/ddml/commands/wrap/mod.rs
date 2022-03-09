//! Implementation of command traits for wrapping.
use anyhow::Result;

use crate::{
    core::spongos::Spongos,
    ddml::types::{
        Size,
        Uint16,
        Uint32,
        Uint64,
        Uint8,
    },
};

pub(crate) struct Context<F, OS> {
    spongos: Spongos<F>,
    stream: OS,
}

impl<F, OS> Context<F, OS> where F: Default {
    pub(crate) fn new(stream: OS) -> Self {
        Self {
            spongos: Spongos::<F>::init(),
            stream,
        }
    }

    pub(crate) fn stream(&self) -> &OS {
        &self.stream
    }
}

trait Wrap {
    fn wrapn<T>(&mut self, v: T) -> Result<&mut Self> where T: AsRef<[u8]>;
    fn wrap_u8(&mut self, u: Uint8) -> Result<&mut Self> {
        self.wrapn(&u.to_bytes())
    }
    fn wrap_u16(&mut self, u: Uint16) -> Result<&mut Self> {
        self.wrapn(&u.to_bytes())
    }
    fn wrap_u32(&mut self, u: Uint32) -> Result<&mut Self> {
        self.wrapn(&u.to_bytes())
    }
    fn wrap_u64(&mut self, u: Uint64) -> Result<&mut Self> {
        self.wrapn(&u.to_bytes())
    }
    fn wrap_size(&mut self, size: Size) -> Result<&mut Self> where {
        self.wrap_u8(Uint8::new(size.num_bytes()))?;
        size.encode(|byte| {
            self.wrap_u8(Uint8::new(byte))?;
            Ok(())
        })?;
        Ok(self)
    }
}

mod absorb;
mod absorb_external;
mod commit;
mod dump;
mod fork;
mod guard;
mod join;
mod mask;
mod repeated;
mod skip;
mod squeeze;

mod ed25519;
mod x25519;

// TODO: REMOVE
// use absorb::*;
// use absorb_external::*;
// use commit::*;
// use dump::*;
// use fork::*;
// use guard::*;
// use join::*;
// use mask::*;
// use repeated::*;
// use skip::*;
// use squeeze::*;
// use squeeze_external::*;

// use ed25519::*;
// use x25519::*;
