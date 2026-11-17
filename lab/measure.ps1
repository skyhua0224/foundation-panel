param([string]$Exe, [int]$Port, [string]$Profile, [string]$Mode = "direct", [int]$ObserveMs = 8000)
Add-Type @"
using System; using System.Text; using System.Collections.Generic; using System.Runtime.InteropServices;
public static class W {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc p, IntPtr l);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  public static List<string> Snapshot(uint pid) {
    var o = new List<string>();
    EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p); if (p == pid && IsWindowVisible(h)) {
      var t = new StringBuilder(256); GetWindowText(h, t, 256); var c = new StringBuilder(256); GetClassName(h, c, 256); RECT r; GetWindowRect(h, out r);
      o.Add(c + "|" + t + "|" + (r.R - r.L) + "x" + (r.B - r.T) + "@" + r.L + "," + r.T); } return true; }, IntPtr.Zero);
    return o;
  }
}
"@
$env:SUNSHINE_GUI_TEST_OFFSCREEN = "1"
$env:SUNSHINE_GUI_ALLOW_PARALLEL_INSTANCE = "1"
$env:WEBVIEW2_USER_DATA_FOLDER = $Profile
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=$Port"
$name = [IO.Path]::GetFileNameWithoutExtension($Exe)
$t0 = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
$sw = [System.Diagnostics.Stopwatch]::StartNew()
if ($Mode -eq "runas") { Start-Process -FilePath runas -ArgumentList "/trustlevel:0x20000", "`"$Exe`"" -WindowStyle Hidden | Out-Null } else { Start-Process -FilePath $Exe -WorkingDirectory (Split-Path $Exe) | Out-Null }
$seen = @{}
$pidApp = 0
while ($sw.ElapsedMilliseconds -lt $ObserveMs) {
  if ($pidApp -eq 0) { $pr = Get-Process -Name $name -ErrorAction SilentlyContinue | Select-Object -First 1; if ($pr) { $pidApp = $pr.Id; $seen["_process_seen"] = $sw.Elapsed.TotalMilliseconds } }
  if ($pidApp -ne 0) { foreach ($w in [W]::Snapshot([uint32]$pidApp)) { if (-not $seen.ContainsKey($w)) { $seen[$w] = $sw.Elapsed.TotalMilliseconds } } }
  Start-Sleep -Milliseconds 5
}
@{ pid = $pidApp; t0 = $t0; mode = $Mode; windows = $seen } | ConvertTo-Json -Compress
