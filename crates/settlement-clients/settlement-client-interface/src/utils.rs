use color_eyre::eyre::{eyre, Result};

/// Parse a list of blocks comma separated and assert that they're sorted in ascending order.
pub fn parse_block_numbers(blocks_to_settle: &str) -> Result<Vec<u64>> {
    let sanitized_blocks = blocks_to_settle.replace(' ', "");
    let block_numbers: Vec<u64> = sanitized_blocks
        .split(',')
        .map(|block_no| block_no.parse::<u64>())
        .collect::<Result<Vec<u64>, _>>()
        .map_err(|e| eyre!("Block numbers to settle list is not correctly formatted: {e}"))?;
    Ok(block_numbers)
}
