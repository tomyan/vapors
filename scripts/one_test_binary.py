"""The rewriting half of scripts/one-test-binary.sh, which says what it does."""

import os
import re
import sys

PATH_ATTRIBUTE = re.compile(r'\s*#\[path = "([^"]+)"\]\s*$')
PLAIN_MOD = re.compile(r"^mod ([a-z_0-9]+);$")


def refuse(message):
    sys.exit(f"one-test-binary: {message}")


def module_of(relative):
    """support/surface.rs is `support_surface`."""
    return relative[: -len(".rs")].replace("/", "_").replace("-", "_")


def read_lines(path):
    return open(path).read().split("\n")


def scan(tests, files):
    """For each file: its plain shared modules and its #[path] includes
    under tests/ (relative path, the name it gives)."""
    shared, includes = set(), {}
    for file in files:
        lines = read_lines(os.path.join(tests, file))
        includes[file] = []
        for at, line in enumerate(lines):
            plain = PLAIN_MOD.match(line)
            attribute = PATH_ATTRIBUTE.match(lines[at - 1]) if at > 0 else None
            if plain and attribute:
                relative = os.path.normpath(attribute.group(1))
                if not relative.startswith(".."):
                    includes[file].append((relative, plain.group(1)))
            elif plain:
                name = plain.group(1)
                if not os.path.isfile(os.path.join(tests, name, "mod.rs")):
                    refuse(f"{tests}/{file}:{at + 1} declares `mod {name};` with no {tests}/{name}/mod.rs; fix: give it a #[path] or a mod.rs")
                shared.add(name)
    return shared, includes


def names(text, name):
    """Whether `text`'s code (its comment lines aside) uses `name` as a
    path's first segment."""
    code = "\n".join(line for line in text.split("\n") if not line.lstrip().startswith("//"))
    return re.search(rf"(?<![A-Za-z0-9_:]){name}::", code) is not None


def rewrite_file(tests, file, shared, lifted):
    lines = read_lines(os.path.join(tests, file))
    text = "\n".join(lines)
    out, skip = [], False
    for at, line in enumerate(lines):
        if skip:
            skip = False
            continue
        attribute = PATH_ATTRIBUTE.match(line)
        following = PLAIN_MOD.match(lines[at + 1]) if attribute and at + 1 < len(lines) else None
        if attribute and following and os.path.normpath(attribute.group(1)) in lifted:
            name, module = following.group(1), module_of(os.path.normpath(attribute.group(1)))
            skip = True
            if names(text, name):
                out.append(f"use crate::{module};" if name == module else f"use crate::{module} as {name};")
            continue
        plain = PLAIN_MOD.match(line)
        if plain and plain.group(1) in shared:
            if names(text, plain.group(1)):
                out.append(f"use crate::{plain.group(1)};")
            continue
        out.append(line)
    open(os.path.join(tests, file), "w").write("\n".join(out))


def rewrite_included(tests, relative, includes, lifted):
    """A file several test files include, lifted to the crate's root: a
    `use crate::<name>` or `use super::<name>` naming what its includers
    call another included file (its sibling in the including module,
    before) becomes that file's module. One naming anything else of its
    includer's cannot be lifted, and is refused."""
    aliases = {}
    for file, pairs in includes.items():
        if relative not in [r for r, _ in pairs]:
            continue
        for other, name in pairs:
            if aliases.setdefault(name, other) != other:
                refuse(f"files including {tests}/{relative} call both {aliases[name]} and {other} `{name}`; fix: name them alike")
    path = os.path.join(tests, relative)

    def renamed(match):
        name = match.group(2)
        if name in aliases and aliases[name] in lifted:
            return f"use crate::{module_of(aliases[name])}"
        if match.group(1) == "super":
            refuse(f"{path} names `super::{name}` of the files including it, which a module at the crate's root cannot; fix: move {name} into a support file")
        return match.group(0)

    text = open(path).read()
    open(path, "w").write(re.sub(r"(?m)^use (crate|super)::([a-z_0-9A-Z]+)", renamed, text))


def rewrite_kept(tests, relative):
    """A file one test file includes stays its module: a `use crate::`
    line, which named that test file's crate root, now names its module."""
    path = os.path.join(tests, relative)
    text = open(path).read()
    open(path, "w").write(re.sub(r"(?m)^use crate::", "use super::", text))


GUARD = '''
/// Every `.rs` file directly under tests/ is a module above, so none is
/// silently left out of the binary.
#[test]
fn every_test_file_is_a_module() {
    let tests = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let main = std::fs::read_to_string(tests.join("main.rs")).expect("tests/main.rs reads");
    let mut missing = Vec::new();
    for entry in std::fs::read_dir(&tests).expect("tests/ lists") {
        let name = entry.expect("an entry").file_name().to_string_lossy().into_owned();
        let Some(module) = name.strip_suffix(".rs") else { continue };
        if module != "main" && !main.lines().any(|line| line == format!("mod {module};")) {
            missing.push(name);
        }
    }
    assert!(
        missing.is_empty(),
        "tests/{missing:?} not declared in tests/main.rs, so never run; fix: add `mod <name>;` there, \\
         and in the file `use crate::<module>;` for each shared module it declared (see \\
         scripts/one-test-binary.sh); reproduce: cargo test -p {} --test tests every_test_file_is_a_module",
        env!("CARGO_PKG_NAME")
    );
}
'''


def write_main(tests, files, shared, lifted):
    lines = [
        "//! The crate's integration tests as one binary (docs/plans/crate-structure.md,",
        "//! section 5, step 1): one link of the crate's closure, not one a file.",
        "//! Written by scripts/one-test-binary.sh. A file under tests/ runs only",
        "//! when this declares it: `every_test_file_is_a_module` refuses one it",
        "//! does not.",
        "",
    ]
    lines += [f"mod {name};" for name in sorted(shared)]
    for relative in sorted(lifted):
        lines += [f'#[path = "{relative}"]', f"mod {module_of(relative)};"]
    lines.append("")
    lines += [f"mod {file[:-3]};" for file in files]
    open(os.path.join(tests, "main.rs"), "w").write("\n".join(lines) + "\n" + GUARD)


def write_manifest(manifest):
    text = open(manifest).read()
    marker = "[package]\n"
    if text.count(marker) != 1:
        refuse(f"{manifest} has no single [package] table")
    start = text.index(marker) + len(marker)
    end = text.find("\n[", start)
    end = len(text) if end < 0 else end + 1
    package = text[start:end].rstrip("\n") + "\nautotests = false\n"
    target = "\n# The integration tests as one binary (scripts/one-test-binary.sh)\n[[test]]\nname = \"tests\"\npath = \"tests/main.rs\"\n"
    open(manifest, "w").write(text[:start] + package + text[end:].rstrip("\n") + "\n" + target)


def main():
    tests, manifest = sys.argv[1], sys.argv[2]
    files = sorted(f for f in os.listdir(tests) if f.endswith(".rs"))
    shared, includes = scan(tests, files)
    counts = {}
    for pairs in includes.values():
        for relative, _ in pairs:
            counts[relative] = counts.get(relative, 0) + 1
    lifted = {relative for relative, count in counts.items() if count > 1}
    for relative in sorted(lifted):
        if module_of(relative) in shared or f"{module_of(relative)}.rs" in files:
            refuse(f"{tests}/{relative}'s module name {module_of(relative)} is taken; fix: rename the file")
        rewrite_included(tests, relative, includes, lifted)
    for relative in sorted(set(counts) - lifted):
        rewrite_kept(tests, relative)
    for file in files:
        rewrite_file(tests, file, shared, lifted)
    write_main(tests, files, shared, lifted)
    write_manifest(manifest)


main()
