; Windows installer, built by scripts/windows.ps1:
;   ISCC /DAppVersion=<version> /DStage=<staged folder> squoosh-desktop.iss
; Per-user installation without administrator rights (which also keeps
; drag and drop from Explorer working, as it is blocked to elevated processes).
; The image formats are offered in "Open with" without replacing the
; user's default application.

#ifndef AppVersion
  #error Define AppVersion
#endif
#ifndef Stage
  #error Define Stage
#endif

[Setup]
AppId={{6F0C8E62-3B7B-4E53-9A0B-6A1D2C5B7E41}
AppName=Squoosh Desktop
AppVersion={#AppVersion}
AppVerName=Squoosh Desktop {#AppVersion}
AppPublisher=Squoosh Desktop contributors
DefaultDirName={autopf}\Squoosh Desktop
DefaultGroupName=Squoosh Desktop
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.17763
LicenseFile={#Stage}\LICENSE
SetupIconFile=..\..\app\assets\icon.ico
UninstallDisplayIcon={app}\squoosh-desktop.exe
OutputBaseFilename=squoosh-desktop-{#AppVersion}-windows-x64-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ChangesAssociations=yes
CloseApplications=yes

[Languages]
; Every language of the application that Inno Setup ships a translation for.
Name: "en"; MessagesFile: "compiler:Default.isl"
Name: "fr"; MessagesFile: "compiler:Languages\French.isl"
Name: "de"; MessagesFile: "compiler:Languages\German.isl"
Name: "es"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "it"; MessagesFile: "compiler:Languages\Italian.isl"
Name: "ptbr"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"
Name: "nl"; MessagesFile: "compiler:Languages\Dutch.isl"
Name: "pl"; MessagesFile: "compiler:Languages\Polish.isl"
Name: "ru"; MessagesFile: "compiler:Languages\Russian.isl"
Name: "uk"; MessagesFile: "compiler:Languages\Ukrainian.isl"
Name: "tr"; MessagesFile: "compiler:Languages\Turkish.isl"
Name: "ja"; MessagesFile: "compiler:Languages\Japanese.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#Stage}\squoosh-desktop.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Stage}\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Stage}\THIRD_PARTY.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Stage}\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Stage}\LICENSE-APACHE-2.0"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Stage}\licenses\*"; DestDir: "{app}\licenses"; Flags: ignoreversion

[Icons]
Name: "{group}\Squoosh Desktop"; Filename: "{app}\squoosh-desktop.exe"
Name: "{autodesktop}\Squoosh Desktop"; Filename: "{app}\squoosh-desktop.exe"; Tasks: desktopicon

[Registry]
; ProgID and application registration used by "Open with".
Root: HKA; Subkey: "Software\Classes\SquooshDesktop.Image"; ValueType: string; ValueData: "Image (Squoosh Desktop)"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\SquooshDesktop.Image\DefaultIcon"; ValueType: string; ValueData: """{app}\squoosh-desktop.exe"",0"
Root: HKA; Subkey: "Software\Classes\SquooshDesktop.Image\shell\open\command"; ValueType: string; ValueData: """{app}\squoosh-desktop.exe"" ""%1"""
Root: HKA; Subkey: "Software\Classes\Applications\squoosh-desktop.exe"; ValueType: string; ValueName: "FriendlyAppName"; ValueData: "Squoosh Desktop"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\Applications\squoosh-desktop.exe\shell\open\command"; ValueType: string; ValueData: """{app}\squoosh-desktop.exe"" ""%1"""
Root: HKA; Subkey: "Software\Classes\.jpg\OpenWithProgids"; ValueType: string; ValueName: "SquooshDesktop.Image"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.jpeg\OpenWithProgids"; ValueType: string; ValueName: "SquooshDesktop.Image"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.png\OpenWithProgids"; ValueType: string; ValueName: "SquooshDesktop.Image"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.webp\OpenWithProgids"; ValueType: string; ValueName: "SquooshDesktop.Image"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.avif\OpenWithProgids"; ValueType: string; ValueName: "SquooshDesktop.Image"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.svg\OpenWithProgids"; ValueType: string; ValueName: "SquooshDesktop.Image"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.gif\OpenWithProgids"; ValueType: string; ValueName: "SquooshDesktop.Image"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.bmp\OpenWithProgids"; ValueType: string; ValueName: "SquooshDesktop.Image"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.tif\OpenWithProgids"; ValueType: string; ValueName: "SquooshDesktop.Image"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.tiff\OpenWithProgids"; ValueType: string; ValueName: "SquooshDesktop.Image"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\Applications\squoosh-desktop.exe\SupportedTypes"; ValueType: string; ValueName: ".jpg"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\squoosh-desktop.exe\SupportedTypes"; ValueType: string; ValueName: ".jpeg"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\squoosh-desktop.exe\SupportedTypes"; ValueType: string; ValueName: ".png"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\squoosh-desktop.exe\SupportedTypes"; ValueType: string; ValueName: ".webp"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\squoosh-desktop.exe\SupportedTypes"; ValueType: string; ValueName: ".avif"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\squoosh-desktop.exe\SupportedTypes"; ValueType: string; ValueName: ".svg"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\squoosh-desktop.exe\SupportedTypes"; ValueType: string; ValueName: ".gif"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\squoosh-desktop.exe\SupportedTypes"; ValueType: string; ValueName: ".bmp"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\squoosh-desktop.exe\SupportedTypes"; ValueType: string; ValueName: ".tif"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\squoosh-desktop.exe\SupportedTypes"; ValueType: string; ValueName: ".tiff"; ValueData: ""

[Run]
Filename: "{app}\squoosh-desktop.exe"; Description: "{cm:LaunchProgram,Squoosh Desktop}"; Flags: nowait postinstall skipifsilent
