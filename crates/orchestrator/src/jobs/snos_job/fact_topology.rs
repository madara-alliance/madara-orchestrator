//! Fact topology type and helpers.
//!
//! Ported from https://github.com/starkware-libs/cairo-lang/blob/master/src/starkware/cairo/bootloaders/fact_topology.py

use std::collections::HashMap;

use cairo_vm::types::builtin_name::BuiltinName;
use cairo_vm::vm::runners::cairo_pie::{BuiltinAdditionalData, CairoPie, PublicMemoryPage};
use utils::ensure;

use super::error::FactError;

pub const GPS_FACT_TOPOLOGY: &str = "gps_fact_topology";

/// Flattened fact tree
#[derive(Debug, Clone)]
pub struct FactTopology {
    /// List of pairs (n_pages, n_nodes)
    pub tree_structure: Vec<usize>,
    /// Page sizes (pages are leaf nodes)
    pub page_sizes: Vec<usize>,
}

/// Returns the fact topology from the additional data of the output builtin.
pub fn get_fact_topology(cairo_pie: &CairoPie, output_size: usize) -> Result<FactTopology, FactError> {
    if let Some(BuiltinAdditionalData::Output(additional_data)) = cairo_pie.additional_data.0.get(&BuiltinName::output)
    {
        let tree_structure = match additional_data.attributes.get(GPS_FACT_TOPOLOGY) {
            Some(tree_structure) => {
                ensure!(!tree_structure.is_empty(), FactError::TreeStructureEmpty);
                ensure!(tree_structure.len() % 2 == 0, FactError::TreeStructureLenOdd);
                ensure!(tree_structure.len() <= 10, FactError::TreeStructureTooLarge);
                ensure!(tree_structure.iter().all(|&x| x < 2 << 30), FactError::TreeStructureInvalid);
                tree_structure.clone()
            }
            None => {
                ensure!(additional_data.pages.is_empty(), FactError::OutputPagesLenUnexpected);
                vec![1, 0]
            }
        };
        let page_sizes = get_page_sizes(&additional_data.pages, output_size)?;
        Ok(FactTopology { tree_structure, page_sizes })
    } else {
        Err(FactError::OutputBuiltinNoAdditionalData)
    }
}

/// Returns the sizes of the program output pages, given the pages dictionary that appears
/// in the additional attributes of the output builtin.
pub fn get_page_sizes(pages: &HashMap<usize, PublicMemoryPage>, output_size: usize) -> Result<Vec<usize>, FactError> {
    let mut pages_list: Vec<(usize, usize, usize)> =
        pages.iter().map(|(&id, page)| (id, page.start, page.size)).collect();
    pages_list.sort();

    // The first page id is expected to be 1.
    let mut expected_page_id = 1;
    // We don't expect anything on its start value.
    let mut expected_page_start = None;

    let mut page_sizes = Vec::with_capacity(pages_list.len() + 1);
    // The size of page 0 is output_size if there are no other pages, or the start of page 1 otherwise.
    page_sizes.push(output_size);

    for (page_id, page_start, page_size) in pages_list {
        ensure!(page_id == expected_page_id, FactError::OutputPagesUnexpectedId(page_id, expected_page_id));

        if page_id == 1 {
            ensure!(
                page_start > 0 && page_start < output_size,
                FactError::OutputPagesInvalidStart(page_id, page_start, output_size)
            );
            page_sizes[0] = page_start;
        } else {
            ensure!(
                Some(page_start) == expected_page_start,
                FactError::OutputPagesUnexpectedStart(page_id, page_start, expected_page_start.unwrap_or_default(),) /* The unwrap here is fine as the assert is exactly for this reason */
            );
        }

        ensure!(
            page_size > 0 && page_size < output_size,
            FactError::OutputPagesInvalidSize(page_id, page_size, output_size)
        );
        expected_page_start = Some(page_start + page_size);
        expected_page_id += 1;

        page_sizes.push(page_size);
    }

    ensure!(
        pages.is_empty() || expected_page_start == Some(output_size),
        FactError::OutputPagesUncoveredOutput(expected_page_start.unwrap_or_default(), output_size) /* The unwrap here is fine as the assert is exactly for this reason */
    );
    Ok(page_sizes)
}
