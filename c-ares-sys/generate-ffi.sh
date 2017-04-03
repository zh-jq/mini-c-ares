#!/bin/bash

if ! which bindgen > /dev/null 2>&1
then
  echo "bindgen is not in the path"
  exit 1
fi

# Prepare for bindgen, and do it.
(cd c-ares && ./buildconf && ./configure)
bindgen --whitelist-function="ares.*" --whitelist-type="ares.*" --whitelist-type="apattern" --no-recursive-whitelist --no-unstable-rust --output=src/ffi.rs c-ares/ares.h

# bindgen converts 'char' to 'c_schar' - see https://github.com/servo/rust-bindgen/issues/603.
# Since c-ares never uses 'signed char' we can compensate with simple search-and-replace.
sed -i 's/c_schar/c_char/g' src/ffi.rs

# Apply manual patches.
patch -p0 < ffi.patch

# Generate constants.
./generate-constants.pl > src/constants.rs
