$cfg = Get-Content lab/bench-inputs.json | ConvertFrom-Json
New-Item -ItemType Directory -Force bench-result | Out-Null
foreach ($v in $cfg.variants) {
  $exe = (Resolve-Path "bench/$($v.name)/app/sunshine-gui.exe" -ErrorAction SilentlyContinue)
  if (-not $exe) { $exe = (Resolve-Path "bench/$($v.name)/sunshine-gui.exe") }
  foreach ($mode in @("panic", "null-deref", "abort")) {
    $dir = Join-Path (Get-Location) "bench-result/crash-$($v.name)-$mode"
    New-Item -ItemType Directory -Force $dir | Out-Null
    $env:SUNSHINE_TEST_CRASH = $mode
    $env:SUNSHINE_CRASH_DIR = $dir
    $env:SUNSHINE_GUI_ALLOW_PARALLEL_INSTANCE = "1"
    $env:SUNSHINE_GUI_TEST_OFFSCREEN = "1"
    $env:WEBVIEW2_USER_DATA_FOLDER = (Join-Path (Get-Location) "bench/wv2-crash-$($v.name)")
    $p = Start-Process -FilePath $exe.Path -PassThru
    if (-not $p.WaitForExit(40000)) { "$($v.name) $mode : still running, killing"; $p.Kill() } else { "$($v.name) $mode : exit code $($p.ExitCode) " }
    Get-ChildItem $dir | ForEach-Object { "   $($_.Name) $($_.Length)" }
    Stop-Process -Name msedgewebview2 -Force -ErrorAction SilentlyContinue
  }
}
Remove-Item Env:SUNSHINE_TEST_CRASH
