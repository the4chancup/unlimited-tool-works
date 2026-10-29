# Every named UI Automation element of one process's main window, one per line:
#   <ControlType.Name> | <name> | <x>,<y> <width>x<height>
# in screen pixels. A name with a line break continues on the next lines. Reads only: no input
# is sent. Used by `draft_compare.py` (its docstring says why UI Automation: egui publishes its
# widgets through AccessKit, which Windows exposes there).
# Usage (Windows PowerShell 5.1): powershell -NoProfile -File scripts/uia_dump.ps1 <process id>
param([int]$ProcessId)
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
$A = [System.Windows.Automation.AutomationElement]
$T = [System.Windows.Automation.TreeScope]
$cond = New-Object System.Windows.Automation.PropertyCondition($A::ProcessIdProperty, $ProcessId)
# The main window, not a native dialog the process owns (class #32770).
$main = $null
foreach ($w in $A::RootElement.FindAll($T::Children, $cond)) {
  if ($w.Current.ClassName -ne '#32770') { $main = $w; break }
}
if (-not $main) { Write-Output "no window of process $ProcessId"; exit 1 }
foreach ($e in $main.FindAll($T::Descendants, [System.Windows.Automation.Condition]::TrueCondition)) {
  $c = $e.Current
  if ($c.Name) {
    $r = $c.BoundingRectangle
    Write-Output ("{0} | {1} | {2},{3} {4}x{5}" -f $c.ControlType.ProgrammaticName, $c.Name, [int]$r.X, [int]$r.Y, [int]$r.Width, [int]$r.Height)
  }
}
