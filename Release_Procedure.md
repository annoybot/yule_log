# Release Process

This document describes the release procedure for the `yule_log` crates. The process ensures that all crate versions are consistent and published in the correct order.


## Steps for Releasing

1. **Manually bump versions**

   Update the version numbers in all `Cargo.toml` files:

   - `core/Cargo.toml`
   - `macros/Cargo.toml`
   - `integration_tests/Cargo.toml`

   Ensure all package versions and pinned dependency versions (`=x.y.z`) match.


2. **Run version consistency check script**

   Use the provided script to verify that all versions match:

   ```bash
   ./check_versions.sh
   ```
   
   If any mismatches are reported, fix the versions and re-run the script until it passes.


3. **Dry-run publish the `yule_log_macros` crate**

   ```bash
   cargo publish --dry-run -p yule_log_macros
   ```

   Fix any warnings or errors. Repeat the dry-run until it succeeds.


4. **Publish `yule_log_macros`**

   ```bash
   cargo publish -p yule_log_macros
   ```


5. **Dry-run publish the `yule_log` crate**

   ```bash
   cargo publish --dry-run -p yule_log
   ```

   Again, fix any issues and repeat until clean.


6. **Publish `yule_log`**

   ```bash
   cargo publish -p yule_log
   ```
   
## ⚠️ Notes

- The integration test crate is not actuall published (`publish = false`), but we maintain version consistency anyway.



End of release process.