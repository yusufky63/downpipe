; Downpipe's additions to Tauri's NSIS installer (bundle.windows.nsis.installerHooks).
;
; The app registers downpipe-native-host with the browsers for the current
; user at every start (docs/BROWSER_BRIDGE.md). Uninstalling removes that
; registration again, but only if it still starts the host in this install
; folder: a registration that a build from source or another install has
; taken over since is left alone.
;
; The host is copied out before Tauri deletes the files and run only once the
; uninstall went through: after NSIS_HOOK_PREUNINSTALL the uninstaller can
; still stop (the user declines to close a running Downpipe), and then the
; browsers have to keep their registration.
;
; Updates ($UpdateMode, /UPDATE) skip both steps, like Tauri keeps the
; shortcuts: the new version puts the host back at the same path.

!macro NSIS_HOOK_PREUNINSTALL
  ${If} $UpdateMode <> 1
    InitPluginsDir
    CopyFiles /SILENT "$INSTDIR\downpipe-native-host.exe" "$PLUGINSDIR"
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
  ${AndIf} ${FileExists} "$PLUGINSDIR\downpipe-native-host.exe"
    ; nsExec runs it without a console window. The exit code is ignored: a
    ; registration left behind only makes the browsers report a missing bridge.
    nsExec::Exec '"$PLUGINSDIR\downpipe-native-host.exe" uninstall --only-if-host "$INSTDIR\downpipe-native-host.exe"'
    Pop $0
    ; The default install folder, %LOCALAPPDATA%\Downpipe, also holds the
    ; manifests, so Tauri's own RMDir could not remove it while they were there.
    RMDir "$INSTDIR"
  ${EndIf}
!macroend
