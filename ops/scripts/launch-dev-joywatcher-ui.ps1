# Launch JoyWatcher registration UI with a temporary launch-context JSON for manual verification.
# Usage examples:
#   .\ops\scripts\launch-dev-joywatcher-ui.ps1 -Build
#   .\ops\scripts\launch-dev-joywatcher-ui.ps1 -Build -NoLaunch
#   .\ops\scripts\launch-dev-joywatcher-ui.ps1 -DriverId joywatcher-dev -Endpoint localhost -UserId 0

param(
  [switch]$Build,
  [switch]$NoLaunch,
  [switch]$Wait,
  [string]$DriverId,
  [string]$Endpoint = "localhost",
  [int]$UserId = 0,
  [string]$Notes = "",
  [string]$SessionId,
  [string]$InputJsonPath,
  [string]$OutputJsonPath,
  [string]$DriverUiPath,
  [string]$AppRoot
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $PSCommandPath
$repoRoot = Resolve-Path (Join-Path $scriptDir "../..")

if ([string]::IsNullOrWhiteSpace($SessionId)) {
  $SessionId = [guid]::NewGuid().ToString()
}

if ([string]::IsNullOrWhiteSpace($AppRoot)) {
  $AppRoot = $repoRoot.Path
}

if ($Build) {
  & (Join-Path $scriptDir "build-dev-joywatcher-ui.ps1")
}

function Resolve-UiPath {
  param(
    [string]$ExplicitPath,
    [string]$RootPath
  )

  if (-not [string]::IsNullOrWhiteSpace($ExplicitPath)) {
    return (Resolve-Path $ExplicitPath).Path
  }

  $candidates = @(
    (Join-Path $RootPath "ops\driver-ui\joywatcher\registration-ui.exe"),
    (Join-Path $RootPath "target\debug\driver_ui_joywatcher.exe")
  )

  foreach ($candidate in $candidates) {
    if (Test-Path $candidate -PathType Leaf) {
      return (Resolve-Path $candidate).Path
    }
  }

  throw "JoyWatcher registration UI executable not found. Run .\ops\scripts\build-dev-joywatcher-ui.ps1 first or pass -DriverUiPath."
}

$resolvedUiPath = Resolve-UiPath -ExplicitPath $DriverUiPath -RootPath $AppRoot

if ([string]::IsNullOrWhiteSpace($InputJsonPath)) {
  $InputJsonPath = Join-Path $env:TEMP "kt_iot_hub_driver_ui_input_joywatcher_$SessionId.json"
}

if ([string]::IsNullOrWhiteSpace($OutputJsonPath)) {
  $OutputJsonPath = Join-Path $env:TEMP "kt_iot_hub_driver_ui_result_joywatcher_$SessionId.json"
}

if (Test-Path $OutputJsonPath) {
  Remove-Item $OutputJsonPath -Force
}

$driverBlock = [ordered]@{
  driverType = "joywatcher"
}

if (-not [string]::IsNullOrWhiteSpace($DriverId)) {
  $driverBlock.driverId = $DriverId
}

$context = [ordered]@{
  schemaVersion = 1
  requestId = "req-$SessionId"
  generatedAt = [DateTimeOffset]::UtcNow.ToString("o")
  direction = "host-to-driver"
  session = [ordered]@{
    sessionId = $SessionId
    mode = "create-or-edit"
    outputJsonPath = $OutputJsonPath
  }
  driver = $driverBlock
  context = [ordered]@{
    scanGroups = @()
    existingDriverIds = @("joywatcher1", "joywatcher2")
    driverSettings = [ordered]@{
      endpoint = $Endpoint
      user_id = $UserId
      password = ""
      notes = $Notes
    }
  }
}

$inputDir = Split-Path -Parent $InputJsonPath
if (-not [string]::IsNullOrWhiteSpace($inputDir) -and -not (Test-Path $inputDir)) {
  New-Item -ItemType Directory -Path $inputDir -Force | Out-Null
}

$context | ConvertTo-Json -Depth 10 | Set-Content -Path $InputJsonPath -Encoding UTF8

$arguments = @(
  "--driver-ui-mode",
  "--session-id", $SessionId,
  "--driver-type", "joywatcher",
  "--input-json", $InputJsonPath,
  "--output-json", $OutputJsonPath
)

if (-not [string]::IsNullOrWhiteSpace($DriverId)) {
  $arguments += @("--driver-id", $DriverId)
}

Write-Host ">>> JoyWatcher UI launch context prepared" -ForegroundColor Green
Write-Host "  UI EXE      : $resolvedUiPath"
Write-Host "  Session ID  : $SessionId"
Write-Host "  Input JSON  : $InputJsonPath"
Write-Host "  Output JSON : $OutputJsonPath"
Write-Host "  Endpoint    : $Endpoint"
Write-Host "  User ID     : $UserId"
Write-Host ""

if ($NoLaunch) {
  Write-Host ">>> -NoLaunch specified. UI was not started." -ForegroundColor Yellow
  return
}

$process = Start-Process -FilePath $resolvedUiPath -ArgumentList $arguments -PassThru
Write-Host ">>> JoyWatcher UI started (PID=$($process.Id))" -ForegroundColor Cyan

if ($Wait) {
  $process.WaitForExit()
  Write-Host ">>> JoyWatcher UI exited with code $($process.ExitCode)" -ForegroundColor Cyan
  if (Test-Path $OutputJsonPath) {
    Write-Host ">>> Output JSON created: $OutputJsonPath" -ForegroundColor Green
  } else {
    Write-Host ">>> Output JSON not created yet: $OutputJsonPath" -ForegroundColor Yellow
  }
}
