<#
  RD Agent idle resource benchmark. Run from an interactive Windows session.
  Example: .\scripts\measure-agent.ps1 -DurationSeconds 300
  This does not start RD or change any repository.
#>
param(
    [ValidateRange(10, 86400)][int]$DurationSeconds = 300,
    [ValidateRange(250, 10000)][int]$SampleIntervalMs = 1000,
    [string]$ProcessName = "rd-tray",
    [string]$OutputPath = "rd-performance.json"
)
$ErrorActionPreference = "Stop"
$existing = @(Get-Process -Name $ProcessName -ErrorAction SilentlyContinue)
if ($existing.Count -eq 0) { throw "RD Agent를 먼저 실행하세요 ($ProcessName)." }
if ($existing.Count -ne 1) { throw "에이전트가 여러 개입니다. 측정을 중지합니다." }
$pidToMeasure = $existing[0].Id
Write-Host "RD PID=$pidToMeasure | $DurationSeconds seconds | sample $SampleIntervalMs ms"
$samples = New-Object System.Collections.Generic.List[object]
$sw = [System.Diagnostics.Stopwatch]::StartNew()
while ($sw.Elapsed.TotalSeconds -lt $DurationSeconds) {
    $process = Get-Process -Id $pidToMeasure -ErrorAction SilentlyContinue
    if (!$process) { throw "측정 중 RD Agent가 종료되었습니다." }
    $samples.Add([pscustomobject]@{
        at_seconds = [math]::Round($sw.Elapsed.TotalSeconds, 2)
        ram_mb = [math]::Round($process.WorkingSet64 / 1MB, 2)
        cpu_seconds = [math]::Round($process.CPU, 4)
        threads = $process.Threads.Count
        handles = $process.HandleCount
    })
    Start-Sleep -Milliseconds $SampleIntervalMs
}
$sw.Stop()
if ($samples.Count -lt 2) { throw "CPU 사용량 계산을 위한 표본이 부족합니다." }
$rams = @($samples | ForEach-Object { $_.ram_mb } | Sort-Object)
$elapsed = $samples[-1].at_seconds - $samples[0].at_seconds
$cpu = $samples[-1].cpu_seconds - $samples[0].cpu_seconds
$cpuPercent = if ($elapsed -gt 0) {
    [math]::Round(100 * $cpu / ($elapsed * [Environment]::ProcessorCount), 4)
} else { 0 }
$report = [ordered]@{
    process = $ProcessName
    pid = $pidToMeasure
    duration_seconds = [math]::Round($sw.Elapsed.TotalSeconds, 2)
    sample_count = $samples.Count
    ram_working_set_average_mb = [math]::Round(($samples | Measure-Object -Property ram_mb -Average).Average, 2)
    ram_working_set_peak_mb = $rams[-1]
    cpu_percent_machine_average = $cpuPercent
    processor_count = [Environment]::ProcessorCount
    measurements = @($samples)
}
$report | ConvertTo-Json -Depth 5 | Set-Content -Path $OutputPath -Encoding UTF8
Write-Host "RAM average $($report.ram_working_set_average_mb)MB | peak $($report.ram_working_set_peak_mb)MB | CPU $cpuPercent% (host normalized)"
Write-Host "Report: $OutputPath"
