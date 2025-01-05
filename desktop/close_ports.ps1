# Close syre database ports.
$SYRE_SERVER_PROCESS_NAME = "trunk"
$SYRE_SERVER_PORT = 1420

$OUT = Get-Process -Id (Get-NetTCPConnection -LocalPort $SYRE_SERVER_PORT).OwningProcess # Handles, NPM(K), PM(K), WS(K), CPU(s), Id, SI, ProcessName
$PIDS = $OUT | Select-Object -Property Id, ProcessName
$PIDS | ForEach-Object -Process {if ($_.ProcessName -eq $SYRE_SERVER_PROCESS_NAME) {TASKKILL /F /PID $_.Id }}