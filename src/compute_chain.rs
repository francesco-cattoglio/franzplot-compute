use crate::compute_block::*;
use std::any::Any;
use anyhow::{Result, anyhow};

use std::collections::hash_map::HashMap;
use std::rc::Rc;
pub struct ComputeChain {
    pub blocks: HashMap<u16, Rc<dyn ComputeBlock>>,
}

#[derive(Debug)]
pub enum BlockDescriptor {
    Curve (CurveBlockDescriptor),
    Interval (IntervalBlockDescriptor),
}


impl ComputeChain {
    pub fn run_chain(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Compute Encoder this time"),
        });
        self.blocks[&1].encode(&mut encoder);
        let compute_queue = encoder.finish();
        queue.submit(&[compute_queue]);
    }

    pub fn insert(&mut self, id: u16, block: Rc<dyn ComputeBlock>) -> Result<()> {
        if self.blocks.contains_key(&id) {
            Err(anyhow!("a"))
        } else {
            self.blocks.insert(id, block);
            Ok(())
        }
    }
    pub fn create_from_descriptors(device: &wgpu::Device, descriptors: Vec<BlockDescriptor>) -> Result<Self> {
        let blocks = HashMap::<u16, Rc<dyn ComputeBlock>>::new();
        let mut chain = Self {
            blocks,
        };
        // right now descriptors need to be in the "correct" order, so that all blocks that depend
        // on something are encountered after the blocks they depend on.
        for (idx, descriptor) in descriptors.iter().enumerate() {
            let block: Rc<dyn ComputeBlock> = match descriptor {
                BlockDescriptor::Curve(desc) => Rc::new(CurveBlock::new(&chain, device, desc)),
                BlockDescriptor::Interval(desc) => Rc::new(IntervalBlock::new(&chain, device, desc)),
            };
            chain.insert(idx as u16, block)?;
        }

        return Ok(chain);
    }
}


