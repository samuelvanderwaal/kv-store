use super::KvEngine;
use crate::Result;

pub struct KvSled;

impl KvEngine for KvSled {
    fn get(&mut self, _key: String) -> Result<Option<String>> {
        todo!();
    }

    fn set(&mut self, _key: String, _value: String) -> Result<()> {
        todo!();
    }

    fn rm(&mut self, _key: String) -> Result<()> {
        todo!();
    }
}
