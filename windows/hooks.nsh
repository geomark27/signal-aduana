; Uninstall keeps what is useful to reinstall (config, preferences, cache, the token in the
; Credential Manager) and always removes the rest. Nothing is removed during an update.
!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    SetShellVarContext current
    Delete "$LOCALAPPDATA\${BUNDLEID}\notifications.json"
    Delete "$LOCALAPPDATA\${BUNDLEID}\notifications.simulated.json"
    RMDir /r "$LOCALAPPDATA\${BUNDLEID}\EBWebView"
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "${PRODUCTNAME}"
  ${EndIf}
!macroend
