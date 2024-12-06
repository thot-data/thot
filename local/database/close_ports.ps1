# Close syre database ports.
$SYRE_DB_PROCESS_NAME = "syre-local-database"
$SYRE_PUB_PORT = 7048

$OUT = Get-Process -Id (Get-NetTCPConnection -LocalPort $SYRE_PUB_PORT).OwningProcess # Handles, NPM(K), PM(K), WS(K), CPU(s), Id, SI, ProcessName
$PIDS = $OUT | Select-Object -Property Id, ProcessName
$PIDS | ForEach-Object -Process {if ($_.ProcessName -eq $SYRE_DB_PROCESS_NAME) {TASKKILL /F /PID $_.Id }}