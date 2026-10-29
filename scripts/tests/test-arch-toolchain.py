"""Exercise the real PKGBUILD with isolated Rust command doubles; no downloads."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

RECIPE = Path(__file__).resolve().parents[2] / 'packaging/arch/PKGBUILD'


class ArchToolchainTests(unittest.TestCase):
    def run_recipe(self, *, healthy=False, manager=True, download_ok=True, installed=False, broken=False, phase='prepare'):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            bindir = root / 'bin'
            bindir.mkdir()
            (root / 'kaze-2.0.2').mkdir()
            if installed:
                (root / 'installed').touch()
            tool = '''#!/bin/bash
printf '%s %s toolchain=%s\\n' "${0##*/}" "$*" "${RUSTUP_TOOLCHAIN:-}" >> "$TEST_ROOT/calls"
if [[ "$HEALTHY" == 1 ]]; then exit 0; fi
if [[ "$BROKEN" != 1 && "${RUSTUP_TOOLCHAIN:-}" == stable && -f "$TEST_ROOT/installed" ]]; then exit 0; fi
exit 1
'''
            commands = {'cargo': tool, 'rustc': tool, 'npm': '#!/bin/bash\nexit 0\n'}
            if manager:
                commands['rustup'] = '''#!/bin/bash
printf 'rustup %s\\n' "$*" >> "$TEST_ROOT/calls"
[[ "$*" == 'toolchain install stable --profile minimal --no-self-update' ]] || exit 9
[[ "$DOWNLOAD_OK" == 1 ]] || exit 1
/usr/bin/touch "$TEST_ROOT/installed"
'''
            for name, content in commands.items():
                path = bindir / name
                path.write_text(content)
                path.chmod(0o755)
            env = {**os.environ, 'PATH': str(bindir), 'TEST_ROOT': str(root),
                   'HEALTHY': str(int(healthy)), 'DOWNLOAD_OK': str(int(download_ok)),
                   'BROKEN': str(int(broken))}
            env.pop('RUSTUP_TOOLCHAIN', None)
            # Separate invocations model makepkg's separate prepare/build shells.
            phases = ['prepare', 'build'] if phase == 'both' else [phase]
            for current in phases:
                result = subprocess.run(['/bin/bash', '-c',
                    'source "$1"; error() { echo "$*" >&2; }; msg() { echo "$*"; }; ' + current,
                    'test', str(RECIPE)], cwd=root, env=env, capture_output=True, text=True)
                if result.returncode:
                    break
            return result, (root / 'calls').read_text()

    def test_empty_rustup_recovers_and_build_uses_stable(self):
        result, calls = self.run_recipe(phase='both')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(calls.count('rustup toolchain install'), 1)
        self.assertIn('cargo build --release --locked --features tauri/custom-protocol toolchain=stable', calls)

    def test_working_tools_need_no_manager_changes(self):
        for manager in (True, False):
            result, calls = self.run_recipe(healthy=True, manager=manager, phase='both')
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertNotIn('rustup ', calls)

    def test_existing_stable_is_reused_without_download(self):
        result, calls = self.run_recipe(installed=True, phase='both')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotIn('rustup ', calls)

    def test_missing_provider_has_actionable_error(self):
        result, _ = self.run_recipe(manager=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('pacman', result.stderr)

    def test_failed_download_stops_before_frontend_build(self):
        result, calls = self.run_recipe(download_ok=False, phase='both')
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn('cargo build', calls)

    def test_broken_proxies_still_fail_after_install(self):
        result, _ = self.run_recipe(broken=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('working Rust toolchain', result.stderr)


if __name__ == '__main__':
    unittest.main()
