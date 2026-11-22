#ifndef MyAppVersion
  #define MyAppVersion "0.2.0"
#endif

#define MyAppName "A9 Strip"
#define MyAppPublisher "m-labs"
#define MyAppURL "https://github.com/mlabsnz2022/a9-strip"

[Setup]
AppId={{8F6C2B4E-5A19-4C7A-9B3D-2E8F1A9C0B01}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}/releases
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
DefaultDirName={commoncf64}\VST3
DisableDirPage=yes
DisableProgramGroupPage=yes
LicenseFile=..\LICENSE
OutputDir=..\target\installer
OutputBaseFilename=A9-Strip-v{#MyAppVersion}-Windows-Setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=admin
UninstallDisplayName={#MyAppName} (VST3 / CLAP)

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Types]
Name: "full"; Description: "Full installation (VST3 + CLAP)"
Name: "vst3"; Description: "VST3 plugin only (Recommended)"
Name: "custom"; Description: "Custom installation"; Flags: iscustom

[Components]
Name: "vst3"; Description: "VST3 Plugin (C:\Program Files\Common Files\VST3)"; Types: full vst3 custom; Flags: fixed
Name: "clap"; Description: "CLAP Plugin (C:\Program Files\Common Files\CLAP)"; Types: full

[Files]
Source: "..\target\bundled\A9 Strip.vst3\*"; DestDir: "{commoncf64}\VST3\A9 Strip.vst3"; Components: vst3; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\target\bundled\A9 Strip.clap"; DestDir: "{commoncf64}\CLAP"; Components: clap; Flags: ignoreversion

[Messages]
FinishedLabel=Setup has finished installing [name] on your computer.%n%nRestart or rescan plugins in your DAW to load A9 Strip.
