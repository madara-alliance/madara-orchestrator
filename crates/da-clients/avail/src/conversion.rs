pub(crate) fn get_bytes_from_state_diff(state_diff: Vec<Vec<u8>>) -> Vec<u8> {
    state_diff.into_iter().flatten().collect()
}
