#requires -version 5.1
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
try {
  $devices = @(Get-PnpDevice -PresentOnly -ErrorAction Stop | Where-Object {
    $_.Class -eq 'MEDIA' -and $_.Manufacturer -ieq 'VB-Audio Software' -and
      @($_.HardwareID) -icontains 'VBAudioVACWDM'
  })
  if ($devices.Count -ne 1 -or [string]$devices[0].InstanceId -notmatch '^ROOT\\[A-Za-z0-9_.&{}-]+\\[A-Za-z0-9_.&{}-]+$') {
    throw 'Could not identify exactly one VB-Cable device.'
  }
  $problem = @(Get-PnpDeviceProperty -InstanceId $devices[0].InstanceId -KeyName 'DEVPKEY_Device_ProblemCode' -ErrorAction Stop)
  if ($problem.Count -ne 1 -or [int]$problem[0].Data -ne 22) {
    throw 'The VB-Cable device is no longer disabled.'
  }
  & (Join-Path $env:SystemRoot 'System32\pnputil.exe') /enable-device $devices[0].InstanceId | Out-Host
  $code = $LASTEXITCODE
  if ($code -eq 0 -or $code -eq 3010) { exit $code }
  throw "Windows could not enable the VB-Cable device (exit $code)."
} catch {
  [Console]::Error.WriteLine($_.Exception.Message)
  exit 1
}
