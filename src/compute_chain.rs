use crate::compute_block::*;
use std::any::Any;
use anyhow::{Result, anyhow};

use std::collections::hash_map::HashMap;
use std::rc::Rc;
pub struct ComputeChain {
    pub blocks: HashMap<u16, Rc<dyn Any>>,
    pub device: wgpu::Device,
}

pub enum BlockDescriptor {
    Curve (CurveBlockDescriptor),
    Interval (IntervalBlockDescriptor),
}


impl ComputeChain {
    pub fn insert(&mut self, id: u16, block: Rc<dyn Any>) -> Result<()> {
        if self.blocks.contains_key(&id) {
            Err(anyhow!("a"))
        } else {
            self.blocks.insert(id, block);
            Ok(())
        }
    }
    pub fn create_from_descriptors(device: wgpu::Device, descriptors: Vec<BlockDescriptor>) -> Result<Self> {
        let blocks = HashMap::<u16, Rc<dyn Any>>::new();
        let mut chain = Self {
            device,
            blocks,
        };
        for (idx, descriptor) in descriptors.iter().enumerate() {
            let block: Rc<dyn Any> = match descriptor {
                BlockDescriptor::Curve(desc) => Rc::new(CurveBlock::new(&chain, desc)),
                BlockDescriptor::Interval(desc) => Rc::new(IntervalBlock::new(&chain, desc)),
            };
            chain.insert(idx as u16, block)?;
        }

        return Ok(chain);
    }
}


