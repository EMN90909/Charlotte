; Charlotte Browser — Windows 7 SP1 ESR Legacy Installer (Inno Setup)
; Supports both 32-bit (x86) and 64-bit (x64) Windows 7 SP1 builds in ESR mode.
; For Windows 8, 8.1, 10, and 11, use Charlotte's custom Opera/Chrome-style installer (`charlotte-installer`).
;
; Build examples:
;   ISCC /DArch=x86 /DBundleDir=..\..\target\bundle-win7-x86 charlotte.iss
;   ISCC /DArch=x64 /DBundleDir=..\..\target\bundle-win7-x64 charlotte.iss
;
; IMPORTANT WIN7 RUNTIME REQUIREMENTS:
; - Compile Charlotte.exe with a Win7-compatible ABI toolchain (Rust 1.75 Win7 or YY-Thunks/VC-LTL5)
;   and audit the binary with `verify_win7_pe.py` so no Win8+/Win10+ DLL imports exist.
; - Bundle WebView2 Fixed Version 109.0.1518.78 ESR (or CEF 109 ESR) inside `{#BundleDir}\webview2-109-esr\`
;   because modern Evergreen WebView2 (>= 110) dropped Windows 7 support.

#ifndef Arch
  #define Arch "x86"
#endif

#ifndef BundleDir
  #define BundleDir "..\..\target\bundle"
#endif

#define AppName "Charlotte (Windows 7 ESR)"
#define AppVersion "0.4.0"
#define AppPublisher "Noethric"
#define AppExeName "Charlotte.exe"
#define CloudServerURL "https://api-charlotte-server.onrender.com"

[Setup]
AppId={{A9539175-16DA-4EBF-90A8-B392B2BD203A}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#AppPublisher}
DefaultDirName={autopf}\Charlotte
DefaultGroupName=Charlotte
OutputDir=..\..\target\installer
OutputBaseFilename=Charlotte-Setup-{#AppVersion}-win7-esr-{#Arch}
Compression=lzma2
SolidCompression=yes
WizardStyle=classic
; Target Windows 7 SP1 (NT 6.1 SP1) baseline
MinVersion=6.1sp1
#if Arch == "x64"
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
#else
ArchitecturesAllowed=x86 x64compatible
#endif
PrivilegesRequired=lowest

[Files]
; Win7-audited Charlotte binary + Fixed Version WebView2 109.0.1518.78 ESR / CEF 109 ESR runtime
Source: "{#BundleDir}\*"; DestDir: "{app}"; Flags: recursesubdirs ignoreversion

[Icons]
Name: "{autoprograms}\Charlotte"; Filename: "{app}\{#AppExeName}"; Parameters: "--esr-mode=win7 --low-ram-mode"
Name: "{autodesktop}\Charlotte"; Filename: "{app}\{#AppExeName}"; Parameters: "--esr-mode=win7 --low-ram-mode"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Noethric\Charlotte"; ValueType: string; ValueName: "RuntimeChannel"; ValueData: "win7-esr"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Noethric\Charlotte"; ValueType: string; ValueName: "Architecture"; ValueData: "{#Arch}"
Root: HKCU; Subkey: "Software\Noethric\Charlotte"; ValueType: string; ValueName: "CloudServerUrl"; ValueData: "{#CloudServerURL}"

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional shortcuts:"

[Run]
Filename: "{app}\{#AppExeName}"; Parameters: "--esr-mode=win7 --low-ram-mode"; Description: "Launch Charlotte (Windows 7 ESR)"; Flags: nowait postinstall skipifsilent

[Code]
function InitializeSetup(): Boolean;
var
  Version: TWindowsVersion;
begin
  GetWindowsVersionEx(Version);
  Result := True;
  if (Version.Major = 6) and (Version.Minor = 1) and (Version.ServicePackMajor < 1) then
  begin
    MsgBox('Charlotte Windows 7 ESR requires Windows 7 Service Pack 1 (SP1) and security update KB3063858.', mbError, MB_OK);
    Result := False;
  end
  else if (Version.Major > 6) or ((Version.Major = 6) and (Version.Minor >= 2)) then
  begin
    if MsgBox('You are running Windows 8 or newer. For the best experience, Charlotte recommends using the modern Charlotte Custom Installer (Opera/Chrome-style). Continue installing the Windows 7 ESR edition anyway?', mbConfirmation, MB_YESNO) = IDNO then
      Result := False;
  end;
end;
