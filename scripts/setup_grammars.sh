#!/bin/bash
# Setup Tree-sitter Community Grammars
# Downloads established community grammars and extracts their query files

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
GRAMMARS_DIR="$PROJECT_ROOT/tree-sitter-grammars"
QUERIES_DIR="$PROJECT_ROOT/queries"

echo "🚀 Setting up Tree-sitter Community Grammars"
echo "📁 Project root: $PROJECT_ROOT"
echo "📁 Grammars will be downloaded to: $GRAMMARS_DIR"
echo "📁 Queries will be extracted to: $QUERIES_DIR"

# Create directories
mkdir -p "$GRAMMARS_DIR"
mkdir -p "$QUERIES_DIR"

# Tier 1 - Core Languages
declare -A TIER1_GRAMMARS=(
    ["rust"]="https://github.com/tree-sitter/tree-sitter-rust"
    ["python"]="https://github.com/tree-sitter/tree-sitter-python"
    ["javascript"]="https://github.com/tree-sitter/tree-sitter-javascript"
    ["typescript"]="https://github.com/tree-sitter/tree-sitter-typescript"
    ["java"]="https://github.com/tree-sitter/tree-sitter-java"
    ["go"]="https://github.com/tree-sitter/tree-sitter-go"
    ["c"]="https://github.com/tree-sitter/tree-sitter-c"
    ["cpp"]="https://github.com/tree-sitter/tree-sitter-cpp"
    ["c-sharp"]="https://github.com/tree-sitter/tree-sitter-c-sharp"
    ["php"]="https://github.com/tree-sitter/tree-sitter-php"
    ["ruby"]="https://github.com/tree-sitter/tree-sitter-ruby"
    ["swift"]="https://github.com/alex-pinkus/tree-sitter-swift"
    ["kotlin"]="https://github.com/fwcd/tree-sitter-kotlin"
)

# Tier 2 - Scripting/Systems
declare -A TIER2_GRAMMARS=(
    ["bash"]="https://github.com/tree-sitter/tree-sitter-bash"
    ["powershell"]="https://github.com/tree-sitter/tree-sitter-powershell"
    ["lua"]="https://github.com/tree-sitter-grammars/tree-sitter-lua"
    ["zig"]="https://github.com/tree-sitter/tree-sitter-zig"
    ["haskell"]="https://github.com/tree-sitter/tree-sitter-haskell"
    ["ocaml"]="https://github.com/tree-sitter/tree-sitter-ocaml"
    ["objc"]="https://github.com/tree-sitter-grammars/tree-sitter-objc"
)

# Tier 3 - Web/Data/Markup
declare -A TIER3_GRAMMARS=(
    ["html"]="https://github.com/tree-sitter/tree-sitter-html"
    ["css"]="https://github.com/tree-sitter/tree-sitter-css"
    ["scss"]="https://github.com/tree-sitter-grammars/tree-sitter-scss"
    ["json"]="https://github.com/tree-sitter/tree-sitter-json"
    ["yaml"]="https://github.com/tree-sitter/tree-sitter-yaml"
    ["toml"]="https://github.com/tree-sitter/tree-sitter-toml"
    ["xml"]="https://github.com/tree-sitter-grammars/tree-sitter-xml"
    ["markdown"]="https://github.com/tree-sitter-grammars/tree-sitter-markdown"
    ["graphql"]="https://github.com/bkegley/tree-sitter-graphql"
    ["sql"]="https://github.com/tjdevries/tree-sitter-sql"
    ["proto"]="https://github.com/tree-sitter/tree-sitter-proto"
)

# Tier 4 - Build/DevOps/Config
declare -A TIER4_GRAMMARS=(
    ["dockerfile"]="https://github.com/camdencheek/tree-sitter-dockerfile"
    ["make"]="https://github.com/tree-sitter-grammars/tree-sitter-make"
    ["cmake"]="https://github.com/uyha/tree-sitter-cmake"
    ["nix"]="https://github.com/tree-sitter/tree-sitter-nix"
    ["hcl"]="https://github.com/tree-sitter-grammars/tree-sitter-hcl"
    ["ini"]="https://github.com/justinmk/tree-sitter-ini"
)

download_grammar() {
    local name="$1"
    local url="$2"
    local tier="$3"

    echo "📦 [$tier] Downloading $name from $url"

    local target_dir="$GRAMMARS_DIR/$name"

    if [ -d "$target_dir" ]; then
        echo "   ↻ Updating existing grammar: $name"
        (cd "$target_dir" && git pull)
    else
        echo "   ⬇ Cloning grammar: $name"
        git clone --depth 1 "$url" "$target_dir"
    fi

    # Extract queries if they exist
    local queries_src="$target_dir/queries"
    local queries_dest="$QUERIES_DIR/$name"

    if [ -d "$queries_src" ]; then
        echo "   📋 Extracting queries for $name"
        mkdir -p "$queries_dest"
        cp -r "$queries_src"/* "$queries_dest/" 2>/dev/null || true

        # List available query files
        if [ -n "$(ls -A "$queries_dest" 2>/dev/null)" ]; then
            echo "   ✅ Available queries: $(ls "$queries_dest" | tr '\n' ' ')"
        else
            echo "   ⚠️ No query files found in $queries_src"
        fi
    else
        echo "   ⚠️ No queries directory found for $name"
        # Create basic query files for important languages
        mkdir -p "$queries_dest"
        create_basic_queries "$name" "$queries_dest"
    fi
}

create_basic_queries() {
    local lang="$1"
    local dest="$2"

    case "$lang" in
        "rust"|"python"|"javascript"|"typescript"|"java")
            echo "   🔧 Creating basic query templates for $lang"

            # Basic imports query
            cat > "$dest/imports.scm" << 'EOF'
; Basic import detection - will be enhanced with community patterns
; This is a fallback template
EOF

            # Basic functions query
            cat > "$dest/functions.scm" << 'EOF'
; Basic function detection - will be enhanced with community patterns
; This is a fallback template
EOF
            ;;
    esac
}

# Download Tier 1 (highest priority)
echo ""
echo "🥇 Downloading Tier 1 Grammars (Core Languages)"
for name in "${!TIER1_GRAMMARS[@]}"; do
    download_grammar "$name" "${TIER1_GRAMMARS[$name]}" "Tier 1"
done

# Download Tier 2 (if requested)
if [[ "${1:-}" == "--full" || "${1:-}" == "--tier2" ]]; then
    echo ""
    echo "🥈 Downloading Tier 2 Grammars (Scripting/Systems)"
    for name in "${!TIER2_GRAMMARS[@]}"; do
        download_grammar "$name" "${TIER2_GRAMMARS[$name]}" "Tier 2"
    done
fi

# Download Tier 3 (if requested)
if [[ "${1:-}" == "--full" || "${1:-}" == "--tier3" ]]; then
    echo ""
    echo "🥉 Downloading Tier 3 Grammars (Web/Data/Markup)"
    for name in "${!TIER3_GRAMMARS[@]}"; do
        download_grammar "$name" "${TIER3_GRAMMARS[$name]}" "Tier 3"
    done
fi

# Download Tier 4 (if requested)
if [[ "${1:-}" == "--full" || "${1:-}" == "--tier4" ]]; then
    echo ""
    echo "🔧 Downloading Tier 4 Grammars (Build/DevOps/Config)"
    for name in "${!TIER4_GRAMMARS[@]}"; do
        download_grammar "$name" "${TIER4_GRAMMARS[$name]}" "Tier 4"
    done
fi

echo ""
echo "🎉 Grammar setup complete!"
echo "📊 Summary:"
echo "   📁 Grammars downloaded to: $GRAMMARS_DIR"
echo "   📋 Queries extracted to: $QUERIES_DIR"
echo "   🔍 Available languages: $(ls "$QUERIES_DIR" | wc -l)"

echo ""
echo "🚀 Next steps:"
echo "   1. Update Cargo.toml with additional tree-sitter parsers"
echo "   2. Test the enhanced query engine"
echo "   3. Add language-specific query refinements"

echo ""
echo "Usage examples:"
echo "   ./setup_grammars.sh              # Download Tier 1 only"
echo "   ./setup_grammars.sh --tier2      # Download Tier 1 + 2"
echo "   ./setup_grammars.sh --full       # Download all tiers"