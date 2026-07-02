#!/bin/bash
set -e

REPO_ROOT=$(pwd)

# Ensure cargo-clone is installed.
if ! command -v cargo-clone &> /dev/null; then
    echo "cargo-clone could not be found. Installing it now..."
    cargo install cargo-clone
fi

# Test utility crates that must be cloned and imported from the actual repository.
# We would not need this hack if we published test utility crates to crates.io.
# Formatted as: "crate_name|version|path"
test_utility_crates=(
    "battler-test-utils|0.1.0|$REPO_ROOT/battler-test-utils"
    "test-utils|0.1.0|$REPO_ROOT/battler-wamp/test-utils"
)

# List of crates with their specific versions and flags.
# Formatted as: "crate_name|version|test_flags|add_crates_policy"
crates_config=(
    "battler|0.9|--no-default-features|all"
    "battler-choice|0.3|--no-default-features|none"
    "battler-data|0.3|--no-default-features|none"
    "battler-local-data|0.1|--no-default-features|all"
    "battler-prng|0.3|--no-default-features --features alloc|none"
    "battler-wamp|0.5||testonly"
    "battler-wamp-uri|0.1||none"
    "battler-wamp-values|~0.2.2||none"
    "battler-wamprat|0.7||testonly"
    "battler-wamprat-error|0.2||none"
    "battler-wamprat-message|~0.1.3||none"
    "battler-wamprat-schema|0.4||testonly"
    "battler-wamprat-uri|0.5||none"
    "serde-struct-tuple|~0.1.3||none"
    "serde-struct-tuple-enum|0.1||none"
)

# Set up a clean temporary directory.
TMP_DIR=$(mktemp -d)
echo "📂 Created temporary test directory at: $TMP_DIR"
echo "------------------------------------------------"

# Copy environment and configuration of our repository.
[[ -f "$REPO_ROOT/.cargo" ]] && cp -r "$REPO_ROOT/.cargo" "$TMP_DIR/.cargo"

# Clone all crates.
for item in "${crates_config[@]}"; do
    IFS='|' read -r crate version rest <<< "$item"

    echo -e "\n=> Cloning $crate ($version)..."

    if ! cargo clone "$crate@$version" -- "$TMP_DIR/$crate" &> /dev/null; then
        echo "   ❌ Failed to download $crate version $version. Is it published?"
        exit 1
    fi
done

# Package all test utility crates.
for item in "${test_utility_crates[@]}"; do
    IFS='|' read -r crate version path <<< "$item"

    echo -e "\n=> Cloning test utility $crate ($version; $path)..."

    (cargo package -p "$crate" --allow-dirty && cp -r "$REPO_ROOT/target/package/$crate-$version" "$TMP_DIR/$crate")

    # Move any dependency over to our local clones.
    for item in "${crates_config[@]}"; do
        IFS='|' read -r other_crate rest <<< "$item"
        if grep -qF "[dependencies.$other_crate]" "$TMP_DIR/$crate/Cargo.toml"; then
            (
                cd "$TMP_DIR/$crate"
                cargo remove "$other_crate"
                cargo add "$other_crate" --path "../$other_crate"
            )
        fi
    done
done

# Coordinate dependencies within our local clones.
for item in "${crates_config[@]}"; do
    IFS='|' read -r crate version flags add_crates_policy <<< "$item"

    echo -e "\n=> Modifying dependencies for $crate ($version): $add_crates_policy..."

    # Add local crates as a dependency.
    if [[ "$add_crates_policy" == "all" ]]; then
        for item in "${crates_config[@]}"; do
            IFS='|' read -r other_crate rest <<< "$item"
            if grep -qF "[dependencies.$other_crate]" "$TMP_DIR/$crate/Cargo.toml"; then
                (
                    cd "$TMP_DIR/$crate"
                    cargo remove "$other_crate"
                    cargo add "$other_crate" --path "../$other_crate"
                )
            fi
        done
    fi

    # Add local test utility crates as a dev-dependency.
    if [[ "$add_crates_policy" != "none" ]]; then
        for item in "${test_utility_crates[@]}"; do
            IFS='|' read -r test_crate version path <<< "$item"
            (
                cd "$TMP_DIR/$crate"
                cargo add --dev "$test_crate" --path "../$test_crate"
            )
        done
    fi
    
done


# Run tests.
for item in "${crates_config[@]}"; do
    IFS='|' read -r crate version flags rest <<< "$item"
    
    echo -e "\n=> Running tests for $crate ($version)..."

    # Use 'eval' to properly handle spaces within the flags string.
    if (cd "$TMP_DIR/$crate" && eval "cargo test $flags"); then
        echo "   ✅ $crate passed."

    else
        echo "   ❌ $crate failed."
        exit 1
    fi
    
done

# Cleanup
rm -rf "$TMP_DIR"
echo "------------------------------------------------"
echo "🎉 Cleaned up temp files. All downloads and tests complete!"
