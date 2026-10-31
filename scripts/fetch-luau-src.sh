#!/bin/sh

set -eu

# `cargo tree -i luau0-src` to get tag
luau_tag=0.740
luau_repository=https://github.com/luau-lang/luau

vendor_directory=vendor
clone_directory="$vendor_directory/luau-full"
target_directory="$vendor_directory/luau"

cd "$(dirname "$0")/.."

rm -rf "$target_directory" "$clone_directory"
mkdir -p "$vendor_directory"
git clone --depth 1 --branch "$luau_tag" "$luau_repository" "$clone_directory"

mkdir -p "$target_directory"
cp -r "$clone_directory/Analysis" "$clone_directory/LICENSE.txt" "$target_directory/"

for header_directory in Common Ast Config Compiler VM Bytecode; do
    mkdir -p "$target_directory/$header_directory"
    cp -r "$clone_directory/$header_directory/include" "$target_directory/$header_directory/"
done

rm -rf "$clone_directory"
