Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Get-DriverBuildContext {
  param(
    [Parameter(Mandatory = $true)]
    [string]$ScriptPath
  )

  $scriptDir = Split-Path -Parent $ScriptPath
  $repoRoot = Resolve-Path (Join-Path $scriptDir "../..")

  return @{
    ScriptDir = $scriptDir
    RepoRoot = $repoRoot.Path
  }
}

function Invoke-CargoBuildManifest {
  param(
    [Parameter(Mandatory = $true)]
    [string]$RepoRoot,

    [Parameter(Mandatory = $true)]
    [string]$ManifestPath,

    [Parameter(Mandatory = $true)]
    [string]$Label,

    [ValidateSet("debug", "release")]
    [string]$BuildKind = "debug"
  )

  $cargoCommandParts = @("build")
  if ($BuildKind -eq "release") {
    $cargoCommandParts += "--release"
  }
  $cargoCommandParts += @("--manifest-path", $ManifestPath)

  Write-Host ">>> cargo build $Label ($BuildKind)..."
  Push-Location $RepoRoot
  try {
    & cargo @cargoCommandParts
    if ($LASTEXITCODE -ne 0) {
      throw "cargo build failed (exit code $LASTEXITCODE): $Label"
    }
  }
  finally {
    Pop-Location
  }
}

function Invoke-CargoBuildPackageTarget {
  param(
    [Parameter(Mandatory = $true)]
    [string]$RepoRoot,

    [Parameter(Mandatory = $true)]
    [string]$PackageName,

    [Parameter(Mandatory = $true)]
    [string]$Target,

    [Parameter(Mandatory = $true)]
    [string]$Label,

    [ValidateSet("debug", "release")]
    [string]$BuildKind = "debug"
  )

  $cargoCommandParts = @("build")
  if ($BuildKind -eq "release") {
    $cargoCommandParts += "--release"
  }
  $cargoCommandParts += @("-p", $PackageName, "--target", $Target)

  Write-Host ">>> cargo build $Label ($BuildKind / $Target)..."
  Push-Location $RepoRoot
  try {
    & cargo @cargoCommandParts
    if ($LASTEXITCODE -ne 0) {
      throw "cargo build failed (exit code $LASTEXITCODE): $Label"
    }
  }
  finally {
    Pop-Location
  }
}

function Assert-BuildArtifactExists {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path
  )

  if (-not (Test-Path $Path -PathType Leaf)) {
    throw "Build artifact not found: $Path"
  }
}

function Install-DriverUiBuildArtifact {
  param(
    [Parameter(Mandatory = $true)]
    [string]$ScriptDir,

    [Parameter(Mandatory = $true)]
    [string]$DriverType,

    [Parameter(Mandatory = $true)]
    [string]$SourcePath,

    [switch]$StageToBundle
  )

  if ($StageToBundle) {
    & (Join-Path $ScriptDir "install-driver-ui.ps1") -DriverType $DriverType -SourcePath $SourcePath -StageToBundle
    return
  }

  & (Join-Path $ScriptDir "install-driver-ui.ps1") -DriverType $DriverType -SourcePath $SourcePath
}

function Install-DriverRuntimeBuildArtifact {
  param(
    [Parameter(Mandatory = $true)]
    [string]$ScriptDir,

    [Parameter(Mandatory = $true)]
    [string]$DriverType,

    [Parameter(Mandatory = $true)]
    [string]$SourcePath,

    [string]$TargetFileName,

    [switch]$StageToBundle
  )

  if (-not [string]::IsNullOrWhiteSpace($TargetFileName)) {
    if ($StageToBundle) {
      & (Join-Path $ScriptDir "install-driver-runtime.ps1") -DriverType $DriverType -SourcePath $SourcePath -TargetFileName $TargetFileName -StageToBundle
      return
    }

    & (Join-Path $ScriptDir "install-driver-runtime.ps1") -DriverType $DriverType -SourcePath $SourcePath -TargetFileName $TargetFileName
    return
  }

  if ($StageToBundle) {
    & (Join-Path $ScriptDir "install-driver-runtime.ps1") -DriverType $DriverType -SourcePath $SourcePath -StageToBundle
    return
  }

  & (Join-Path $ScriptDir "install-driver-runtime.ps1") -DriverType $DriverType -SourcePath $SourcePath
}

function Invoke-DriverManifestBuildInstall {
  param(
    [Parameter(Mandatory = $true)]
    [string]$RepoRoot,

    [Parameter(Mandatory = $true)]
    [string]$ScriptDir,

    [Parameter(Mandatory = $true)]
    [string]$ManifestPath,

    [Parameter(Mandatory = $true)]
    [string]$Label,

    [Parameter(Mandatory = $true)]
    [string]$ArtifactPath,

    [Parameter(Mandatory = $true)]
    [string]$DriverType,

    [Parameter(Mandatory = $true)]
    [ValidateSet("ui", "runtime")]
    [string]$ArtifactKind,

    [string]$TargetFileName,

    [ValidateSet("debug", "release")]
    [string]$BuildKind = "debug",

    [switch]$StageToBundle
  )

  Invoke-CargoBuildManifest -RepoRoot $RepoRoot -ManifestPath $ManifestPath -Label $Label -BuildKind $BuildKind
  Assert-BuildArtifactExists -Path $ArtifactPath

  if ($ArtifactKind -eq "ui") {
    Install-DriverUiBuildArtifact -ScriptDir $ScriptDir -DriverType $DriverType -SourcePath $ArtifactPath -StageToBundle:$StageToBundle
    return
  }

  Install-DriverRuntimeBuildArtifact -ScriptDir $ScriptDir -DriverType $DriverType -SourcePath $ArtifactPath -TargetFileName $TargetFileName -StageToBundle:$StageToBundle
}

function Invoke-DriverPackageTargetBuildInstall {
  param(
    [Parameter(Mandatory = $true)]
    [string]$RepoRoot,

    [Parameter(Mandatory = $true)]
    [string]$ScriptDir,

    [Parameter(Mandatory = $true)]
    [string]$PackageName,

    [Parameter(Mandatory = $true)]
    [string]$Target,

    [Parameter(Mandatory = $true)]
    [string]$Label,

    [Parameter(Mandatory = $true)]
    [string]$ArtifactPath,

    [Parameter(Mandatory = $true)]
    [string]$DriverType,

    [string]$TargetFileName,

    [ValidateSet("debug", "release")]
    [string]$BuildKind = "debug",

    [switch]$StageToBundle
  )

  Invoke-CargoBuildPackageTarget -RepoRoot $RepoRoot -PackageName $PackageName -Target $Target -Label $Label -BuildKind $BuildKind
  Assert-BuildArtifactExists -Path $ArtifactPath

  Install-DriverRuntimeBuildArtifact -ScriptDir $ScriptDir -DriverType $DriverType -SourcePath $ArtifactPath -TargetFileName $TargetFileName -StageToBundle:$StageToBundle
}
