"""Clean release workers need real frontend assets before Tauri's Rust macro."""
from pathlib import Path
import re
import unittest

WORKFLOW = Path(__file__).resolve().parents[2] / '.github/workflows/release.yml'


class ReleaseBuildOrderTests(unittest.TestCase):
    def test_every_native_job_builds_frontend_before_all_target_lint(self):
        source = WORKFLOW.read_text()
        for job in ['build-linux', 'build-windows', 'build-arch']:
            with self.subTest(job=job):
                block = re.search(r'^  ' + job + r':\n(.*?)(?=^  [\w-]+:|\Z)', source, re.M | re.S).group(1)
                self.assertIn('npm run build', block)
                install = block.index('npm ci')
                build = block.index('npm run build')
                lint = block.index('cargo +1.98.1 clippy')
                self.assertLess(install, build)
                self.assertLess(build, lint)
                self.assertIn('--all-targets', block[lint:])


if __name__ == '__main__':
    unittest.main()
