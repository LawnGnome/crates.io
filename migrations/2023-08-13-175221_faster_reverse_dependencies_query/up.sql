ALTER TABLE versions ADD column semver_no_prerelease semver_triple GENERATED ALWAYS AS (to_semver_no_prerelease(num)) STORED;
