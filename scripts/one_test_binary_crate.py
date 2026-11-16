"""The crate-level edits of scripts/one-test-binary.sh: refusing a
manifest it cannot fold, the manifest's one test target, and the
reproducing commands that named a per-file target."""

import os
import re

from one_test_binary_refuse import refuse


def refuse_targets(manifest, files, nextest):
    """A manifest with its own test or bench targets, or a nextest
    override naming one of the files' binaries, cannot be folded by
    this script: refused, naming the file and the line."""
    for at, line in enumerate(open(manifest).read().split("\n")):
        if line.strip() in ("[[test]]", "[[bench]]") or re.match(r"\s*(harness|required-features)\s*=", line):
            refuse(f"{manifest}:{at + 1}: `{line.strip()}`: an explicit target this script does not fold; fix: fold this crate by hand")
    if not os.path.isfile(nextest):
        return
    for at, line in enumerate(open(nextest).read().split("\n")):
        for name in re.findall(r"binary\(([a-z_0-9]+)\)", line):
            if f"{name}.rs" in files:
                refuse(f"{nextest}:{at + 1}: an override names binary({name}), which folding removes; fix: move the override to `binary(tests) & test(/^{name}::/)` first")


def write_manifest(manifest):
    """`autotests = false` after [package]'s last key, and the one test
    target at the end."""
    lines = open(manifest).read().rstrip("\n").split("\n")
    if lines.count("[package]") != 1:
        refuse(f"{manifest} has no single [package] table")
    start = lines.index("[package]")
    end = next((k for k in range(start + 1, len(lines)) if lines[k].startswith("[")), len(lines))
    last_key = max(k for k in range(start, end) if re.match(r"[a-z-]+(\.[a-z-]+)? *=", lines[k]))
    lines.insert(last_key + 1, "autotests = false")
    lines += ["", "# The integration tests as one binary (scripts/one-test-binary.sh)", "[[test]]", 'name = "tests"', 'path = "tests/main.rs"']
    open(manifest, "w").write("\n".join(lines) + "\n")


def rewrite_reproduce(package, files, roots):
    """`-p <package> --test <file>` names a target folding removes:
    everywhere under `roots` it becomes `-p <package> --test tests <file>::`,
    the test ids' prefix in the one binary. Returns the files changed."""
    modules = "|".join(re.escape(f[:-3]) for f in files)
    pattern = re.compile(rf"(-p {re.escape(package)} --test )({modules})\b(?!::)")
    changed = []
    for root in roots:
        for folder, _, names in os.walk(root):
            if "/target" in folder or "/." in folder:
                continue
            for name in names:
                if not name.endswith((".rs", ".md", ".sh", ".toml")):
                    continue
                path = os.path.join(folder, name)
                text = open(path).read()
                new = pattern.sub(lambda m: f"{m.group(1)}tests {m.group(2)}::", text)
                if new != text:
                    open(path, "w").write(new)
                    changed.append(path)
    return changed
