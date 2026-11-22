#define AppName "Charlotte"
#define AppVersion "0.4.0"
#define AppPublisher "Noethric"
#define AppExeName "Charlotte.exe"

[Setup]
AppId={{A9539175-16DA-4EBF-90A8-B392B2BD203A}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#AppPublisher}
DefaultDirName={autopf}\Charlotte
DefaultGroupName=Charlotte
OutputDir=..\..\target\installer
OutputBaseFilename=Charlotte-Setup-{#AppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
MinVersion=6.1sp1
ArchitecturesAllowed=x86 x64compatible arm64
PrivilegesRequired=lowest

[Files]
Source: "..\..\target\bundle\*"; DestDir: "{app}"; Flags: recursesubdirs ignoreversion

[Icons]
Name: "{autoprograms}\Charlotte"; Filename: "{app}\{#AppExeName}"
Name: "{autodesktop}\Charlotte"; Filename: "{app}\{#AppExeName}"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional shortcuts:"

[Run]
Filename: "{app}\{#AppExeName}"; Description: "Launch Charlotte"; Flags: nowait postinstall skipifsilent
