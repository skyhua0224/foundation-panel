param([string]$Exe, [int]$Port, [string]$Profile)
Add-Type @"
using System; using System.Runtime.InteropServices;
public static class W { [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h); }
"@
$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $Exe
$psi.WorkingDirectory = Split-Path $Exe
$psi.UseShellExecute = $false
$psi.EnvironmentVariables["SUNSHINE_GUI_TEST_OFFSCREEN"] = "1"
$psi.EnvironmentVariables["SUNSHINE_GUI_ALLOW_PARALLEL_INSTANCE"] = "1"
$psi.EnvironmentVariables["WEBVIEW2_USER_DATA_FOLDER"] = $Profile
$psi.EnvironmentVariables["WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS"] = "--remote-debugging-port=$Port"
$t0 = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$p = [System.Diagnostics.Process]::Start($psi)
$win = -1
while ($sw.ElapsedMilliseconds -lt 60000) {
  $p.Refresh()
  if ($p.HasExited) { break }
  $h = $p.MainWindowHandle
  if ($h -ne [IntPtr]::Zero -and [W]::IsWindowVisible($h)) { $win = $sw.Elapsed.TotalMilliseconds; break }
  Start-Sleep -Milliseconds 2
}
@{ pid = $p.Id; t0 = $t0; windowMs = $win; exited = $p.HasExited } | ConvertTo-Json -Compress
