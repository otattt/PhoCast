; PhoCast All-in-One Installer Script
; Bundles PhoCast Client and the DirectShow Webcam Driver into a single setup.exe

#define MyAppName "PhoCast"
#define MyAppVersion "1.0.0"
#define MyAppPublisher "PhoCast Team"
#define MyAppURL "https://github.com/otattt/PhoCast"
#define MyAppExeName "PhoCast.exe"

[Setup]
AppId={{D37E6924-4B2E-4D61-A79B-7C0094B1B3C0}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
OutputDir=..\dist
OutputBaseFilename=PhoCast-Setup-v1.0.0
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=admin
SetupIconFile=icon.ico

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
; Main Executable
Source: "..\client-pc\rust\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
; Driver DLLs
Source: "..\driver\UnityCaptureFilter64.dll"; DestDir: "{app}\driver"; Flags: ignoreversion
Source: "..\driver\UnityCaptureFilter32.dll"; DestDir: "{app}\driver"; Flags: ignoreversion
; Documentation
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
; Automatically register DirectShow Virtual Camera as 'PhoCast Camera' during installation
Filename: "regsvr32.exe"; Parameters: "/s ""{app}\driver\UnityCaptureFilter64.dll"" ""/i:UnityCaptureName=PhoCast Camera"""; StatusMsg: "Registering PhoCast 64-bit Camera Driver..."; Flags: runhidden
Filename: "regsvr32.exe"; Parameters: "/s ""{app}\driver\UnityCaptureFilter32.dll"" ""/i:UnityCaptureName=PhoCast Camera"""; StatusMsg: "Registering PhoCast 32-bit Camera Driver..."; Flags: runhidden
; Launch app after install
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
; Automatically unregister DirectShow Virtual Camera during uninstallation
Filename: "regsvr32.exe"; Parameters: "/u /s ""{app}\driver\UnityCaptureFilter64.dll"""; Flags: runhidden
Filename: "regsvr32.exe"; Parameters: "/u /s ""{app}\driver\UnityCaptureFilter32.dll"""; Flags: runhidden
