# Windows UI measurement profile

Run a private copy of the Panel executable from a writable test directory.
Set `SUNSHINE_GUI_TEST_ISOLATION=1` and
`SUNSHINE_GUI_TEST_PROFILE=<name>` for that process. Use an ASCII name made
of letters, numbers, hyphens or underscores. Pass `--hidden` for a tray-only
start. Use a different name per run unless a test needs to reuse its profile.

The Panel stores configuration, application data, temporary files and the
WebView2 profile under `test-profiles/<name>` next to the executable. It
bypasses single-instance activation and disables startup reconciliation,
desktop integration, notification registrations and commands unrelated to
the measurement. The local UI proxy accepts read-only browser requests in
this mode. Keep the profile directory private to the test runner. Crash
reports use the private data directory even when `SUNSHINE_CRASH_DIR` is set.

Host-backed pages are outside the scope of isolated UI measurements.
Host configuration and authentication are resolved inside the test profile.
Leave `host/config/local-auth.token` absent in the measurement kit so the run
uses no installed host credentials.

Normal startup continues to use the installed configuration and integrations.
The switch is a measurement configuration for a controlled desktop session.
