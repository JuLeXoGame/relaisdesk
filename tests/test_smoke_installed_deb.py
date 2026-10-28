import os
import pathlib
import shutil
import signal
import subprocess
import tempfile
import unittest


SCRIPT = pathlib.Path(__file__).resolve().parents[1] / 'tools/smoke-installed-deb.sh'
FAKE = '''#!/usr/bin/env python3
import os, pathlib, sys, time
state = pathlib.Path(os.environ['SMOKE_TEST_STATE'])
mode = os.environ['SMOKE_TEST_MODE']
if sys.argv[1] == '--server':
    if mode == 'crash':
        print('Invalid vm isolate snapshot', flush=True)
        sys.exit(1)
    time.sleep(0.3)
    state.touch()
    while True:
        time.sleep(0.1)
if mode == 'stubborn':
    while not state.exists():
        time.sleep(0.01)
if mode in ('reject', 'stubborn'):
    print('Changing permanent password was rejected by daemon')
elif not state.exists():
    print('No such file or directory (os error 2)')
else:
    password = pathlib.Path(sys.argv[2])
    if password.stat().st_mode & 0o077:
        sys.exit('Password handoff file is not private')
    print('Done!')
'''

DELAYED_SETSID = '''#!/usr/bin/env python3
import os, pathlib, signal, sys, time
if os.environ['SMOKE_TEST_MODE'] == 'stubborn':
    signal.signal(signal.SIGTERM, signal.SIG_IGN)
    pathlib.Path(os.environ['SMOKE_TEST_STATE']).touch()
time.sleep(30)
os.execv(os.environ['SMOKE_TEST_SETSID'], ['setsid', *sys.argv[1:]])
'''


class InstalledDebSmokeTests(unittest.TestCase):
    def run_case(self, mode, delay_session=False):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            binary = root / 'rustdesk'
            binary.write_text(FAKE)
            binary.chmod(0o700)
            environment = dict(os.environ, RUNNER_TEMP=directory,
                               SMOKE_TEST_STATE=str(root / 'ready'), SMOKE_TEST_MODE=mode)
            if delay_session:
                starter = root / 'setsid'
                starter.write_text(DELAYED_SETSID)
                starter.chmod(0o700)
                environment['SMOKE_TEST_SETSID'] = shutil.which('setsid')
                environment['PATH'] = str(root) + os.pathsep + environment['PATH']
            with subprocess.Popen(['bash', str(SCRIPT), str(binary)], env=environment,
                                  text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                  start_new_session=True) as process:
                try:
                    stdout, stderr = process.communicate(timeout=8)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.communicate()
                    raise
                result = subprocess.CompletedProcess(process.args, process.returncode, stdout, stderr)
            self.assertFalse(list(root.glob('rustdesk-smoke-*')), 'Temporary password must be removed')
            self.assertNotIn('smoke-test-pass-123', result.stdout + result.stderr)
            return result

    def test_waits_for_the_desktop_ipc_instead_of_service_state(self):
        result = self.run_case('ready')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('Done!', result.stdout)

    def test_runtime_snapshot_crash_still_fails_with_diagnostics(self):
        result = self.run_case('crash')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Invalid vm isolate snapshot', result.stdout + result.stderr)

    def test_password_rejection_is_not_treated_as_a_readiness_retry(self):
        result = self.run_case('reject')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('rejected by daemon', result.stderr)

    def test_cleanup_does_not_wait_for_session_creation(self):
        result = self.run_case('reject', delay_session=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('rejected by daemon', result.stderr)

    def test_cleanup_terminates_a_starter_ignoring_sigterm(self):
        result = self.run_case('stubborn', delay_session=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('rejected by daemon', result.stderr)


if __name__ == '__main__':
    unittest.main()
