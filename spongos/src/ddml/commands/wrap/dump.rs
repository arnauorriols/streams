use anyhow::Result;

use crate::{
    core::prp::PRP,
    ddml::{
        commands::{wrap::Context, Dump},
        io,
    },
};

impl<F: PRP, OS: io::OStream> Dump for Context<OS, F> {
    fn dump<'a>(&mut self, args: core::fmt::Arguments<'a>) -> Result<&mut Self> {
        println!(
            "dump: {}: ostream=[{}] spongos=[{:?}]",
            args,
            self.stream.dump(),
            self.spongos
        );

        Ok(self)
    }
}
