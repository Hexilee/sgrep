use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::anyhow;
use sgrep_collector::Collector;

#[derive(Clone)]
pub struct Registry {
    collectors: Arc<HashMap<&'static str, Box<dyn Collector>>>,
}

pub struct RegistryBuilder {
    collectors: Vec<Box<dyn Collector>>,
}

impl RegistryBuilder {
    pub fn register(mut self, collector: impl 'static + Collector) -> Self {
        self.collectors.push(box collector);
        self
    }

    pub fn build(self) -> anyhow::Result<Registry> {
        let mut collectors = HashMap::new();
        for collector in self.collectors {
            let name = collector.name();
            if collectors.insert(name, collector).is_some() {
                return Err(anyhow!("collector {} already registered", name));
            }
        }
        Ok(Registry {
            collectors: Arc::new(collectors),
        })
    }
}

impl Registry {
    pub fn builder() -> RegistryBuilder {
        RegistryBuilder {
            collectors: Vec::new(),
        }
    }

    pub fn get(&self, name: &str) -> Option<&dyn Collector> {
        self.collectors.get(name).map(|c| &**c)
    }

    pub fn must_get(&self, name: &str) -> &dyn Collector {
        self.get(name).unwrap_or_else(|| {
            panic!("collector {} not registered", name);
        })
    }

    pub fn collect(&self, path: impl AsRef<Path>) -> Option<(&'static str, String)> {
        self.collectors
            .values()
            .filter_map(|c| {
                let collector = c.as_ref();
                if collector.should_collect(path.as_ref()).ok()? {
                    let content = collector.collect(path.as_ref()).ok()?;
                    Some((collector.name(), content))
                } else {
                    None
                }
            })
            .next()
    }
}
