; Installeur Inno Setup de Namify (version Rust).
; Compilation : cargo build --release puis iscc installer\Namify.iss
; Le setup est cree dans installer\Output\Namify-Setup-<version>.exe
#define MyAppExe "Namify.exe"
#define MyAppName "Namify"
#define MyAppVersion "2.0.0"
#define MyAppURL "https://github.com/Les-Doubles-A/Namify"

[Setup]
; Meme AppId que la 1.x : l'installation met a jour l'ancienne version.
AppId={{C1D762E3-8798-51B0-9B45-0C9300A7B443}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher=Les-Doubles-A
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}/issues
AppUpdatesURL={#MyAppURL}/releases
VersionInfoVersion={#MyAppVersion}
VersionInfoDescription={#MyAppName} - renommage de fichiers par lot
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputDir=Output
OutputBaseFilename={#MyAppName}-Setup-{#MyAppVersion}
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
CloseApplications=yes
ChangesAssociations=yes
SetupIconFile=..\Namify.ico
UninstallDisplayIcon={app}\{#MyAppExe}
UninstallDisplayName={#MyAppName}

[Languages]
Name: "french"; MessagesFile: "compiler:Languages\French.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[CustomMessages]
french.SendTo=Ajouter Namify au menu « Envoyer vers » de l'explorateur
english.SendTo=Add Namify to the Explorer "Send to" menu
french.FolderMenu=Ajouter « Renommer avec Namify » au clic droit sur les dossiers
english.FolderMenu=Add "Rename with Namify" to the folder right-click menu
french.FolderVerb=Renommer avec Namify
english.FolderVerb=Rename with Namify
french.Integration=Intégration à l'explorateur :
english.Integration=Explorer integration:

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "sendto"; Description: "{cm:SendTo}"; GroupDescription: "{cm:Integration}"
Name: "foldermenu"; Description: "{cm:FolderMenu}"; GroupDescription: "{cm:Integration}"

[Files]
Source: "..\target\release\{#MyAppExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\assets\Phosphor-LICENSE.txt"; DestDir: "{app}\licenses"; Flags: ignoreversion

[InstallDelete]
; Restes eventuels de la version Python 1.x
Type: files; Name: "{app}\*.old.exe"
Type: files; Name: "{app}\*.new.exe"

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExe}"; WorkingDir: "{app}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExe}"; WorkingDir: "{app}"; Tasks: desktopicon
Name: "{usersendto}\{#MyAppName}"; Filename: "{app}\{#MyAppExe}"; Tasks: sendto

[Registry]
; Clic droit sur un dossier
Root: HKCU; Subkey: "Software\Classes\Directory\shell\Namify"; ValueType: string; ValueName: ""; ValueData: "{cm:FolderVerb}"; Flags: uninsdeletekey; Tasks: foldermenu
Root: HKCU; Subkey: "Software\Classes\Directory\shell\Namify"; ValueType: string; ValueName: "Icon"; ValueData: """{app}\{#MyAppExe}"",0"; Tasks: foldermenu
Root: HKCU; Subkey: "Software\Classes\Directory\shell\Namify\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExe}"" ""%1"""; Tasks: foldermenu
; Clic droit dans le fond d'un dossier ouvert
Root: HKCU; Subkey: "Software\Classes\Directory\Background\shell\Namify"; ValueType: string; ValueName: ""; ValueData: "{cm:FolderVerb}"; Flags: uninsdeletekey; Tasks: foldermenu
Root: HKCU; Subkey: "Software\Classes\Directory\Background\shell\Namify"; ValueType: string; ValueName: "Icon"; ValueData: """{app}\{#MyAppExe}"",0"; Tasks: foldermenu
Root: HKCU; Subkey: "Software\Classes\Directory\Background\shell\Namify\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#MyAppExe}"" ""%V"""; Tasks: foldermenu

[Run]
Filename: "{app}\{#MyAppExe}"; Description: "{cm:LaunchProgram,{#MyAppName}}"; WorkingDir: "{app}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
Type: files; Name: "{app}\*.old.exe"
Type: files; Name: "{app}\*.new.exe"
