$SYRE_PORT = 7048
$processes = Get-Process -Id (Get-NetTCPConnection -LocalPort $SYRE_PORT).OwningProcess 
$pids = $processes | Select-Object -Property Id
$pids | ForEach-Object {
    Stop-Process -Id $_.Id
}