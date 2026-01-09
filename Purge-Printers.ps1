param([switch]$Force)

function Say($msg){ Write-Host $msg }
$doit = $Force.IsPresent

if (-not ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()
  ).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
  throw "请用【管理员】运行 PowerShell。"
}

Say "=== Purge Printers (Fixed Order) ==="
Say ("模式: " + ($(if($doit){"FORCE 删除"} else {"预览 Dry-Run"})))

# 0) 确保 Spooler 可用（关键）
Say "`n[确保 Print Spooler Running]"
if ($doit) {
  Set-Service Spooler -StartupType Automatic -ErrorAction SilentlyContinue
  Start-Service Spooler -ErrorAction SilentlyContinue
}
Get-Service Spooler | Format-List Status, StartType

# 1) 删除打印机队列（Spooler 必须运行）
Say "`n[删除打印机队列]"
$printers = Get-Printer -ErrorAction SilentlyContinue | Sort-Object Name
$printers | Select Name, DriverName, PortName | Format-Table -AutoSize

foreach ($p in $printers) {
  if ($doit) {
    try { Remove-Printer -Name $p.Name -ErrorAction Stop; Say "Removed Printer: $($p.Name)" }
    catch { Say "Failed Printer: $($p.Name) => $($_.Exception.Message)" }
  } else {
    Say "Dry-Run: Remove-Printer -Name `"$($p.Name)`""
  }
}

# 2) 删除第三方打印机驱动对象（建议保留 Microsoft 自带驱动，避免系统组件异常）
Say "`n[删除打印机驱动对象（跳过 Microsoft）]"
$drivers = Get-PrinterDriver -ErrorAction SilentlyContinue | Sort-Object Name
$drivers | Select Name, Manufacturer, MajorVersion | Format-Table -AutoSize

foreach ($d in $drivers) {
  if ($d.Manufacturer -match "Microsoft") { continue } # 只删第三方更稳
  if ($doit) {
    try { Remove-PrinterDriver -Name $d.Name -ErrorAction Stop; Say "Removed Driver: $($d.Name)" }
    catch { Say "Failed Driver: $($d.Name) => $($_.Exception.Message)" }
  } else {
    Say "Dry-Run: Remove-PrinterDriver -Name `"$($d.Name)`""
  }
}

# 3) 停止 Spooler -> 清空队列文件（这一步才应该停服务）
Say "`n[停止 Spooler 并清空队列文件]"
$spoolPath = Join-Path $env:WINDIR "System32\spool\PRINTERS"
if ($doit) {
  Stop-Service Spooler -Force -ErrorAction SilentlyContinue
  if (Test-Path $spoolPath) {
    Get-ChildItem $spoolPath -Force -ErrorAction SilentlyContinue | Remove-Item -Force -ErrorAction SilentlyContinue
  }
} else {
  Say "Dry-Run: Stop-Service Spooler -Force"
  Say "Dry-Run: Remove-Item $spoolPath\* -Force"
}

# 4) 删除 DriverStore 中的第三方打印相关驱动包（pnputil），然后再启动 Spooler
Say "`n[删除 DriverStore 中打印相关驱动包（第三方）]"
$pnplist = pnputil /enum-drivers
$blocks = ($pnplist -split "(\r?\n){2,}") | Where-Object { $_ -match "Published Name" }

# 只删第三方：Provider 不是 Microsoft
