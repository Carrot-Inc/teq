@echo off
rem teq launcher 2: the teq this build pins, with nothing installed. Reads the first line of the
rem teq.lock beside it (the compiler's version) and the lock's binaries line of this machine's
rem classifier (URL, sha1, size), then runs that binary with the arguments as given: TEQ when set;
rem else teq's cache, bin\<sha1>\teq-<version>-<classifier>.exe; else coursier's copy of the URL,
rem copied into the cache; else fetched with curl.exe into the cache. A file is taken only with the
rem pinned size, and placed only with the pinned sha1. A release before 0.1.7, the first served, is
rem refused, and so is a lock without this machine's binary, an empty table among them (a release
rem not published yet). Written by sbt-teq's teqExportAll, which replaces it only while it is
rem unedited (docs/TARGETS.md, "The launchers"). No variable is
rem expanded inside parentheses, where a parenthesis in a path would end them; a FOR variable is,
rem since cmd substitutes it after it has read the line.
setlocal EnableExtensions DisableDelayedExpansion
rem A variable named ERRORLEVEL would stand in the place of the last exit code.
set "ERRORLEVEL="
if defined TEQ goto override

set "lock=%~dp0teq.lock"
set "message=no teq.lock beside %~f0: export the build with sbt teqExportAll"
if not exist "%lock%" goto fail
set "first="
for /f "usebackq delims=" %%l in ("%lock%") do (set "first=%%l" & goto first)
:first
set "message=%lock%'s first line is not teq: <version>"
if not defined first goto fail
if not "%first:~0,5%"=="teq: " goto fail
set "version=%first:~5%"
if not defined version goto fail
set "version=%version:"=%"
rem The releases before 0.1.7 are served no longer, whatever URL the lock gives; a SNAPSHOT is no
rem release. Its leading numbers, split at a dot, a hyphen or a plus.
if "%version:~-9%"=="-SNAPSHOT" goto served
set "major="
set "minor="
set "patch="
for /f "tokens=1-3 delims=.-+" %%a in ("%version%") do (set "major=%%a" & set "minor=%%b" & set "patch=%%c")
if not defined patch goto served
if not "%major%"=="0" goto served
if "%minor%"=="0" goto old
if not "%minor%"=="1" goto served
for %%p in (0 1 2 3 4 5 6) do if "%patch%"=="%%p" goto old
goto served
:old
set "message=%lock% pins teq %version%, and releases before 0.1.7 are not served: pin 0.1.7 or later (sbt teqExportAll), or set TEQ to a local binary"
goto fail
:served

set "arch=%PROCESSOR_ARCHITECTURE%"
if defined PROCESSOR_ARCHITEW6432 set "arch=%PROCESSOR_ARCHITEW6432%"
set "classifier=windows-%arch%"
if /i "%arch%"=="AMD64" set "classifier=windows-x86_64"
if /i "%arch%"=="ARM64" set "classifier=windows-aarch_64"
rem The binaries block opens the lock, so the first line led by the classifier and a colon is its.
set "url="
set "sha1="
set "size="
set "more="
for /f "usebackq tokens=1-4*" %%a in ("%lock%") do if "%%a"=="%classifier%:" (set "url=%%b" & set "sha1=%%c" & set "size=%%d" & set "more=%%e" & goto pinned)
:pinned
set "message=%lock% pins no binary of teq %version% for %classifier%: set TEQ to a teq %version% binary, or export the build (sbt teqExportAll) once the release %version% serves one"
if not defined url goto fail
set "message=%lock%'s binaries line for %classifier% is not <url> <sha1> <size>"
if not defined size goto fail
if defined more goto fail
rem The line in double quotes, as the lock writes a URL of other characters than a plain word's:
rem the quotes go, and no URL holds what the lock would escape.
set "url=%url:"=%"
set "size=%size:"=%"
if "%sha1:~39,1%"=="" goto fail
if not "%sha1:~40%"=="" goto fail
rem A token left once the digits are taken as delimiters is a character of another kind.
for /f "delims=0123456789abcdefABCDEF" %%x in ("%sha1%") do goto fail
for /f "delims=0123456789" %%x in ("%size%") do goto fail

set "root=%TEQ_CACHE_DIR%"
if not defined root if defined XDG_CACHE_HOME set "root=%XDG_CACHE_HOME%\teq"
if not defined root if defined LOCALAPPDATA set "root=%LOCALAPPDATA%\teq"
set "message=no LOCALAPPDATA to find teq's cache in"
if not defined root goto fail
set "dir=%root%\bin\%sha1%"
set "binary=%dir%\teq-%version%-%classifier%.exe"
set "got="
if exist "%binary%" for %%f in ("%binary%") do set "got=%%~zf"
if "%got%"=="%size%" goto run

set "message=cannot create %dir%"
if not exist "%dir%\" mkdir "%dir%" 2>nul
if not exist "%dir%\" goto fail
rem A partial file of its own: its name claimed by a directory beside it, which mkdir makes for one
rem launcher alone; two launchers started in one second draw the same random numbers, the second
rem the next.
set "tries=0"
set "message=cannot create a file in %dir%"
:claim
set /a "tries+=1"
if %tries% gtr 64 goto fail
set "part=%dir%\.teq.%RANDOM%%RANDOM%.part"
mkdir "%part%.d" 2>nul || goto claim

rem Coursier's copy of the URL, <cache>\<scheme>\<host>\<path>, a port's colon, a version's plus
rem and an at, a space, a comma and a semicolon escaped as its CachePath.escape does; a URL with
rem another of the characters it escapes finds no copy and is fetched.
set "from="
set "coursier=%COURSIER_CACHE%"
if not defined coursier if defined LOCALAPPDATA set "coursier=%LOCALAPPDATA%\Coursier\cache\v1"
if not defined coursier goto fetch
for /f "tokens=1,* delims=:" %%s in ("%url%") do (set "scheme=%%s" & set "place=%%t")
set "place=%place:~2%"
rem A percent sign cannot stand in the replacement of a substitution, whose expansion it would end:
rem these run under delayed expansion, and the endlocal line carries the result out, its variables
rem expanded before it runs.
setlocal EnableDelayedExpansion
set "place=!place::=%%3A!"
set "place=!place:+=%%2B!"
set "place=!place:@=%%40!"
set "place=!place: =%%20!"
set "place=!place:,=%%2C!"
set "place=!place:;=%%3B!"
set "place=!place:/=\!"
endlocal & set "copy=%coursier%\%scheme%\%place%"
rem Taken only when its size and sha1 are the pinned ones; any failure of this step, a copy of
rem other bytes among them, leaves it for the fetch.
if not exist "%copy%" goto fetch
set "got="
for %%f in ("%copy%") do set "got=%%~zf"
if not "%got%"=="%size%" goto fetch
certutil -hashfile "%copy%" SHA1 >"%part%.sha1" 2>nul
set "digest="
for /f "usebackq skip=1 delims=" %%h in ("%part%.sha1") do if not defined digest set "digest=%%h"
del "%part%.sha1" 2>nul
if not defined digest goto fetch
set "digest=%digest: =%"
if /i not "%digest%"=="%sha1%" goto fetch
copy /b /y "%copy%" "%part%" >nul 2>nul
if errorlevel 1 goto fetch
set "source=%copy%"
set "from=coursier"
goto verify

:fetch
set "message=%url% is not https, which teq fetches over from every host but this one"
if /i "%url:~0,8%"=="https://" goto curl
if /i "%url:~0,11%"=="http://127." goto curl
if /i "%url:~0,17%"=="http://localhost:" goto curl
if /i "%url:~0,17%"=="http://localhost/" goto curl
if /i "%url:~0,12%"=="http://[::1]" goto curl
goto discard
:curl
set "message=no curl.exe to fetch teq with"
where /q curl.exe || goto discard
>&2 echo teq: fetching teq %version% for %classifier% from %url%
curl.exe --disable --silent --show-error --location --proto-redir =https --max-filesize %size% --connect-timeout 15 --speed-limit 1024 --speed-time 60 --max-time 1800 --write-out "%%{http_code}" --output "%part%" "%url%" >"%part%.status" 2>"%part%.log"
set "code=%errorlevel%"
set "status="
set /p status=<"%part%.status"
del "%part%.status" 2>nul
set "message=%url% has more than the %size% bytes the lock pins: refused"
if "%code%"=="63" goto discard
if not "%code%"=="0" goto unfetched
set "message=GET %url% answered %status%"
if not "%status%"=="" if not "%status%"=="000" if not "%status%"=="200" goto discard
del "%part%.log" 2>nul
set "source=%url%"

:verify
set "got="
for %%f in ("%part%") do set "got=%%~zf"
rem Run directly: the child cmd.exe of FOR /F would expand the path's percent signs once more.
certutil -hashfile "%part%" SHA1 >"%part%.sha1" 2>nul
set "digest="
for /f "usebackq skip=1 delims=" %%h in ("%part%.sha1") do if not defined digest set "digest=%%h"
del "%part%.sha1" 2>nul
if not defined digest set "digest=nothing"
set "digest=%digest: =%"
set "message=%source% has sha1 %digest% and %got% bytes where the lock pins %sha1% and %size% bytes: refused"
if not "%got%"=="%size%" goto unverified
if /i not "%digest%"=="%sha1%" goto unverified
rem Another launcher may have placed it first, and Windows refuses to replace a running binary.
move /y "%part%" "%binary%" >nul 2>nul
del "%part%" 2>nul
rmdir "%part%.d" 2>nul
set "got="
if exist "%binary%" for %%f in ("%binary%") do set "got=%%~zf"
set "message=cannot move %part% to %binary%"
if not "%got%"=="%size%" goto fail

:run
"%binary%" %*
exit /b %errorlevel%

:override
set "message=TEQ names %TEQ%, which is no program"
if exist "%TEQ%" goto teq
where /q "%TEQ%" 2>nul || goto fail
:teq
"%TEQ%" %*
exit /b %errorlevel%

:unverified
rem Coursier's bytes changed under the copy: the fetch, which writes the partial file anew.
if not "%from%"=="coursier" goto discard
set "from="
goto fetch

:unfetched
set "message=GET %url% failed"
for /f "usebackq delims=" %%l in ("%part%.log") do set "message=GET %url% failed: %%l"
del "%part%.log" 2>nul

:discard
del "%part%" 2>nul
rmdir "%part%.d" 2>nul

:fail
setlocal EnableDelayedExpansion
>&2 echo(teq: !message!
exit /b 1
