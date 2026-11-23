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


class AndroidReleaseTests(unittest.TestCase):
    """The phone's in-app updater needs a signed APK and android.json on every release."""

    def block(self, job):
        source = WORKFLOW.read_text()
        return re.search(r'^  ' + job + r':\n(.*?)(?=^  [\w-]+:|\Z)', source, re.M | re.S).group(1)

    def test_android_job_builds_a_signed_apk(self):
        block = self.block('build-android')
        self.assertIn('targets: aarch64-linux-android', block)
        self.assertIn('npx tauri android build --apk --target aarch64', block)
        self.assertIn('secrets.ANDROID_KEYSTORE_BASE64', block)
        self.assertIn('rm -f "$KAZE_KEYSTORE_PATH"', block, 'the key never outlives the build')
        for uses in re.findall(r'uses: (\S+)', block):
            self.assertRegex(uses, r'@[0-9a-f]{40}$', 'actions are pinned by SHA')

    def test_publish_waits_for_android_and_writes_the_manifest(self):
        block = self.block('publish-release')
        self.assertIn('- build-android', block)
        self.assertIn('release-upload/android.json', block)
        self.assertIn('sha256sum', block)
        self.assertLess(block.index('android.json'), block.index('Upload artifacts to GitHub release'))


if __name__ == '__main__':
    unittest.main()
