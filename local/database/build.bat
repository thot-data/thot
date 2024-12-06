@echo off
setlocal enabledelayedexpansion
set root=..\..
set crate=\local\database
set release_dir=%root%\target\release
set crate_release_dir=target\release
set lang=%root%\lang

:: get target
for /f "tokens=* usebackq" %%o in (`rustc -Vv`) do (
  set v=%%o 
  if /i "!v:~0,5!"=="host:" set host=%%o
)

for /f "tokens=1,2 delims= " %%a in ("%host%") do (
  set target=%%b
)

set target_out=%release_dir%\syre-local-database-%target%.exe
set crate_target_out=%crate_release_dir%\syre-local-database-%target%.exe

:: build
if not exist "%release_dir%" mkdir "%release_dir%"
cargo build --release -F server
move %crate_release_dir%\syre-local-database.exe %crate_target_out%

:: copy to other directories
copy %crate_target_out% %target_out%
copy "%target_out%" "%lang%\python\src\syre\bin\"
copy "%target_out%" "%lang%\r\inst\"