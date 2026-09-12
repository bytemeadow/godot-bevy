import fnmatch
import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
LEARNING = {
    "simple-node2d-movement",
    "two-way-sync-demo",
    "dodge-the-creeps-2d",
    "platformer-2d",
}
SURVIVORS = LEARNING | {"perf-test"}
SHARED_PATHS = (
    "godot-bevy/src/app.rs",
    "godot-bevy-macros/src/lib.rs",
    "examples/run_godot.rs",
    "addons/godot-bevy/plugin.gd",
    "Cargo.toml",
    "Cargo.lock",
    "xtask/src/main.rs",
    ".github/workflows/examples.yml",
)


def example_links(text):
    return set(re.findall(r"/tree/main/examples/([a-z0-9-]+)\)", text))


def paths(text):
    return re.findall(r"^\s+- ['\"]([^'\"]+)['\"]$", text, re.MULTILINE)


class ExamplesPortfolioTests(unittest.TestCase):
    def test_learning_projects_and_diagnostic_match_the_tree(self):
        book = (ROOT / "book/src/getting-started/examples.md").read_text()
        learning, separator, diagnostic = book.partition("## Diagnostic tool")
        self.assertTrue(
            separator, "performance diagnostics must be separate from learning projects"
        )
        self.assertEqual(example_links(learning), LEARNING)
        self.assertEqual(example_links(diagnostic), {"perf-test"})
        self.assertEqual(
            {
                path.parent.parent.name
                for path in (ROOT / "examples").glob("*/rust/Cargo.toml")
            },
            SURVIVORS,
        )
        workspace = (ROOT / "Cargo.toml").read_text()
        self.assertEqual(
            set(re.findall(r'"examples/([^/]+)/rust"', workspace)), SURVIVORS
        )
        for example in SURVIVORS:
            with self.subTest(example=example):
                readme = (ROOT / "examples" / example / "README.md").read_text()
                self.assertIn(
                    f"cargo run --manifest-path examples/{example}/rust/Cargo.toml", readme
                )
                self.assertNotIn("cargo build", readme)
        self.assertIn("#268", book)
        self.assertIn(
            "#268", (ROOT / "examples/simple-node2d-movement/README.md").read_text()
        )
        self.assertIn(
            "two-way-sync-demo",
            (ROOT / "book/src/transforms/sync-modes.md").read_text(),
        )
        self.assertNotRegex(
            (ROOT / "examples/perf-test/README.md").read_text(), r"~\d+ FPS"
        )

    def test_workflow_selects_every_survivor_for_shared_changes(self):
        workflow = (ROOT / ".github/workflows/examples.yml").read_text()
        filters = workflow.split("          filters: |\n", 1)[1].split("\n  build:", 1)[0]
        entries = dict(
            re.findall(
                r"^            ([\w-]+):\n((?:              - [^\n]+\n?)+)",
                filters,
                re.MULTILINE,
            )
        )
        self.assertEqual(set(entries), SURVIVORS)
        for example, entry in entries.items():
            with self.subTest(example=example):
                patterns = paths(entry)
                self.assertIn(f"examples/{example}/**", patterns)
                for changed in (*SHARED_PATHS, f"examples/{example}/rust/src/lib.rs"):
                    self.assertTrue(
                        any(fnmatch.fnmatchcase(changed, pattern) for pattern in patterns),
                        f"{changed} must select {example}",
                    )
        for event in ("push", "pull_request"):
            event_block = workflow.split(f"  {event}:\n", 1)[1].split("\n\n", 1)[0]
            event_block = re.split(r"\n  \w+:\n", event_block, maxsplit=1)[0]
            patterns = paths(event_block)
            changed_paths = (
                *SHARED_PATHS,
                *(f"examples/{name}/rust/src/lib.rs" for name in SURVIVORS),
            )
            for changed in changed_paths:
                self.assertTrue(
                    any(fnmatch.fnmatchcase(changed, pattern) for pattern in patterns),
                    f"{changed} must trigger {event}",
                )
        build = workflow.split("\n  build:\n", 1)[1].split("\n  export:\n", 1)[0]
        self.assertIn("example: ${{ fromJson(needs.changes.outputs.examples) }}", build)
        web = workflow.split("\n  build-web:\n", 1)[1]
        self.assertIn(
            "if: false && contains(needs.changes.outputs.examples, 'simple-node2d-movement')",
            web,
        )


if __name__ == "__main__":
    unittest.main()
