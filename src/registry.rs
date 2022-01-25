use std::collections::HashMap;

use sgrep_collector::Collector;

pub struct Registry<'a> {
    collectors: HashMap<&'a str, &'a dyn Collector>,
}

impl<'a> Registry<'a> {
    pub fn new() -> Self {
        Self {
            collectors: HashMap::new(),
        }
    }

    pub fn register(&mut self, collector: &'a dyn Collector) {
        self.collectors.insert(collector.name(), collector);
    }
}
