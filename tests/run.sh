#!/usr/bin/env bash
# Run everything that can be checked without logging in.
#
#     ./tests/run.sh
#
# Both keymap profiles are exercised, because `lua tests/test-config.lua` on
# its own only loads the default one and a duplicate bind in the other profile
# would sail straight through.
set -uo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.." || exit 1
failed=0

echo "==> Config, per keymap profile"
for profile in mac windows; do
    HYPR_KEYMAP="$profile" lua tests/test-config.lua || failed=1
done

echo
echo "==> Generated theme files match core/theme.lua"
lua tools/render-theme.lua --check || failed=1

echo
echo "==> Keybind interop with HyprMod"
./tests/test-interop.sh || failed=1

echo
echo "==> Package lists"
./tests/test-packages.sh || failed=1

echo
echo "==> Shell syntax"
while IFS= read -r script; do
    bash -n "$script" || { echo "    ✗ $script"; failed=1; }
done < <(find . -name '*.sh' -not -path './.git/*')
echo "  ok"

echo
echo "==> Python syntax"
while IFS= read -r script; do
    python3 -c "import ast,sys; ast.parse(open(sys.argv[1]).read())" "$script" \
        || { echo "    ✗ $script"; failed=1; }
done < <(find . -name '*.py' -not -path './.git/*')
echo "  ok"

echo
if ((failed)); then
    echo "FAILED"
    exit 1
fi
echo "All green."
