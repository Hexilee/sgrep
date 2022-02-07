use std::path::Path;

use anyhow::anyhow;
use calamine::{open_workbook, DataType, Ods, Reader, Xls, Xlsb, Xlsx};
use rayon::prelude::*;
use tracing::instrument;

use crate::{Collector, Line};

const EXTERNSIONS: [&str; 4] = ["xls", "xlsb", "xlsx", "ods"];

#[derive(Debug, Clone)]
pub struct SheetCollector;

impl Collector for SheetCollector {
    fn name(&self) -> &'static str {
        "sheet"
    }

    fn accept_extension(&self, extension: Option<&str>) -> bool {
        extension
            .and_then(|e| EXTERNSIONS.contains(&e).then_some(()))
            .is_some()
    }

    #[instrument]
    fn collect(&self, path: &Path) -> anyhow::Result<Vec<Line>> {
        macro_rules! collect {
            ($t:tt) => {
                self.collect_sheet(open_workbook::<$t<_>, _>(path)?)
            };
        }
        match path.extension().and_then(|e| e.to_str()) {
            Some("xls") => collect!(Xls),
            Some("xlsx") => collect!(Xlsx),
            Some("xlsb") => collect!(Xlsb),
            Some("ods") => collect!(Ods),
            _ => Err(anyhow!("unsupported file: {:?}", path)),
        }
    }
}

impl SheetCollector {
    fn collect_sheet<R>(&self, mut sheet: R) -> anyhow::Result<Vec<Line>>
    where
        R: Reader,
        R::Error: Into<anyhow::Error>,
    {
        Ok(sheet
            .worksheets()
            .iter()
            .flat_map(|(name, page)| {
                let mut cells = page
                    .cells()
                    .par_bridge()
                    .filter_map(|(r, c, data)| match data {
                        DataType::String(s) => Some((r, c, s.clone())),
                        DataType::DateTime(_) => Some((r, c, data.as_datetime()?.to_string())),
                        DataType::Int(i) => Some((r, c, i.to_string())),
                        DataType::Float(f) => Some((r, c, f.to_string())),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                cells.sort_by(
                    |(r1, c1, _), (r2, c2, _)| {
                        if r1 != r2 {
                            r1.cmp(r2)
                        } else {
                            c1.cmp(c2)
                        }
                    },
                );
                cells.into_iter().map(move |(r, c, contents)| Line {
                    position: format!("{}({},{})", name, r, c),
                    line: contents,
                })
            })
            .collect())
    }
}
