pub fn compose_sharded_path(random_part: &str, shard_levels: usize, shard_chars: usize) -> String {
    // Create the sharded directory structure using the random part
    let mut sharded_path = String::new();
    for i in 0..shard_levels {
        let start = i * shard_chars;
        if start < random_part.len() {
            let end = std::cmp::min(start + shard_chars, random_part.len());
            let dir_part = &random_part[start..end];
            sharded_path.push_str(dir_part);
            sharded_path.push('/');
        }
    }
    
    sharded_path
}

#[cfg(test)]
mod tests {
    use super::compose_sharded_path;

    #[test]
    fn compose_sharded_path_splits_into_levels() {
        let path = compose_sharded_path("ABCDEFGH", 2, 2);
        assert_eq!(path, "AB/CD/");
    }

    #[test]
    fn compose_sharded_path_stops_when_random_part_exhausted() {
        let path = compose_sharded_path("AB", 3, 2);
        assert_eq!(path, "AB/");
    }

    #[test]
    fn compose_sharded_path_zero_levels_is_empty() {
        let path = compose_sharded_path("ABCDEFG", 0, 2);
        assert_eq!(path, "");
    }
}
