"""The quality gate must fail even when Qlty misclassifies a failed process."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'check-quality.sh'

class QualityGateTests(unittest.TestCase):
    def run_gate(self, cargo_status, qlty_body):
        with tempfile.TemporaryDirectory(prefix='kaze-quality-gate-') as directory:
            root = Path(directory)
            for name, body in [('cargo', f'exit {cargo_status}\n'), ('qlty', qlty_body)]:
                file = root / name
                file.write_text('#!/bin/sh\n' + body)
                file.chmod(0o700)
            return subprocess.run(['bash', str(SCRIPT)], env=dict(os.environ, PATH=str(root)+os.pathsep+os.environ['PATH']), capture_output=True, text=True)

    def test_independent_clippy_failure_is_not_masked(self):
        self.assertEqual(self.run_gate(42, 'exit 0\n').returncode, 42)

    def test_qlty_misclassified_clippy_failure_fails_gate(self):
        result = self.run_gate(0, "printf '# .qlty/out/invoke-test.yaml:\\npluginName: clippy\\nexitCode: 101\\nexitResult: EXIT_RESULT_SUCCESS\\n' >&2\nexit 0\n")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('did not execute successfully', result.stderr)

    def test_success_remains_successful(self):
        self.assertEqual(self.run_gate(0, "printf '# .qlty/out/invoke-test.yaml:\\npluginName: clippy\\nexitCode: 0\\n' >&2\nexit 0\n").returncode, 0)

if __name__ == '__main__':
    unittest.main()
