CREATE INDEX
    versions_num_partial_idx
ON
    versions (split_part(num, '+', 1));
