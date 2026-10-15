param(
    [string]$Agent = (Join-Path $PSScriptRoot '..\target\debug\rd-agent.exe'),
    [int[]]$Counts = @(10, 50, 100)
)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$root = Join-Path $env:TEMP ("rd-scale-" + [guid]::NewGuid().ToString('N'))
$bare = Join-Path $root 'remote.git'
$seed = Join-Path $root 'seed'
$config = Join-Path $root 'config'
$work = Join-Path $root 'work'
$stdout = Join-Path $root 'stdout.txt'
$stderr = Join-Path $root 'stderr.txt'
if (!(Test-Path -LiteralPath $Agent)) { throw "Agent not found: $Agent" }
$maxCount = ($Counts | Measure-Object -Maximum).Maximum
if ($maxCount -lt 1 -or $maxCount -gt 100) { throw "Counts must be between 1 and 100" }
$originalConfig = [Environment]::GetEnvironmentVariable('RD_CONFIG_DIR', 'Process')
try {
    New-Item -ItemType Directory -Force $root, $work, $config | Out-Null
    & git init --quiet --bare --initial-branch=main $bare
    if ($LASTEXITCODE -ne 0) { throw "git init bare failed" }
    & git init --quiet --initial-branch=main $seed
    if ($LASTEXITCODE -ne 0) { throw "git init seed failed" }
    [System.IO.File]::WriteAllText((Join-Path $seed 'README.md'), "RD benchmark")
    & git -C $seed add README.md
    & git -C $seed -c user.name=RD-Benchmark -c user.email=rd@example.invalid commit --quiet -m initial
    if ($LASTEXITCODE -ne 0) { throw "git commit failed" }
    & git -C $seed remote add origin $bare
    & git -C $seed push --quiet -u origin main
    if ($LASTEXITCODE -ne 0) { throw "git push failed" }

    $repos = @()
    for ($i = 1; $i -le $maxCount; $i++) {
        $folder = Join-Path $work ("repo-{0:D3}" -f $i)
        & git clone --quiet --branch main $bare $folder
        if ($LASTEXITCODE -ne 0) { throw "git clone failed: $i" }
        $repos += @{
            id = ("repo-{0:D3}" -f $i)
            url = $bare
            path = $folder
            branch = 'main'
            mode = 'manual'
            enabled = $true
        }
    }
    $env:RD_CONFIG_DIR = $config
    foreach ($count in $Counts) {
        $settings = @{
            version = 1
            interval_minutes = 15
            repositories = @($repos | Select-Object -First $count)
        }
        [System.IO.File]::WriteAllText(
            (Join-Path $config 'config.json'),
            ($settings | ConvertTo-Json -Depth 8),
            (New-Object System.Text.UTF8Encoding($false))
        )
        foreach ($operation in @('list', 'check')) {
            $info = New-Object System.Diagnostics.ProcessStartInfo
            $info.FileName = (Resolve-Path -LiteralPath $Agent).Path
            $info.Arguments = $operation
            $info.UseShellExecute = $false
            $info.CreateNoWindow = $true
            $info.RedirectStandardOutput = $true
            $info.RedirectStandardError = $true
            $clock = [System.Diagnostics.Stopwatch]::StartNew()
            $process = [System.Diagnostics.Process]::Start($info)
            $outputTask = $process.StandardOutput.ReadToEndAsync()
            $errorTask = $process.StandardError.ReadToEndAsync()
            $peak = [long]0
            while (-not $process.HasExited) {
                $process.Refresh()
                $peak = [math]::Max($peak, [long]$process.WorkingSet64)
                Start-Sleep -Milliseconds 20
            }
            $process.WaitForExit()
            $peak = [math]::Max($peak, [long]$process.PeakWorkingSet64)
            $clock.Stop()
            $errorOutput = $errorTask.GetAwaiter().GetResult()
            $null = $outputTask.GetAwaiter().GetResult()
            if ($process.ExitCode -ne 0) {
                throw "$operation ($count) failed: $errorOutput"
            }
            [pscustomobject]@{
                Repositories = $count
                Operation = $operation
                ElapsedMs = [math]::Round($clock.Elapsed.TotalMilliseconds, 0)
                PeakProcessMB = if ($peak -gt 0) { [math]::Round($peak / 1MB, 2) } else { $null }
                ExitCode = $process.ExitCode
            }
        }
    }
} finally {
    if ($null -eq $originalConfig) {
        Remove-Item Env:\RD_CONFIG_DIR -ErrorAction SilentlyContinue
    } else {
        $env:RD_CONFIG_DIR = $originalConfig
    }
    if (Test-Path -LiteralPath $root) {
        Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
    }
}
