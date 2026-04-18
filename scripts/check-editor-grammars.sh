#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

for file in \
  "$repo_root/editors/vscode/lintropy-query-syntax/package.json" \
  "$repo_root/editors/vscode/lintropy-query-syntax/language-configuration.json" \
  "$repo_root/editors/vscode/lintropy-query-syntax/syntaxes/lintropy-query.tmLanguage.json" \
  "$repo_root/editors/vscode/lintropy-query-syntax/syntaxes/lintropy-query-yaml.injection.tmLanguage.json" \
  "$repo_root/editors/textmate/Lintropy Query.tmbundle/Syntaxes/lintropy-query.tmLanguage.json" \
  "$repo_root/editors/textmate/Lintropy Query.tmbundle/Syntaxes/lintropy-query-yaml.injection.tmLanguage.json"
do
  node -e 'JSON.parse(require("fs").readFileSync(process.argv[1], "utf8"))' "$file"
done
