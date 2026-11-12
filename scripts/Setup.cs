using System;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Diagnostics;
using System.IO;
using System.IO.Compression;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Threading;
using System.Windows.Forms;
using Microsoft.Win32;

[assembly: AssemblyTitle("gnomcode Setup")]
[assembly: AssemblyProduct("gnomcode")]
[assembly: AssemblyCompany("gnomcode")]
[assembly: AssemblyVersion("0.9.4.0")]
[assembly: AssemblyFileVersion("0.9.4.0")]

public static class DeviceTheme
{
    public static void Apply(Control root) {
        bool light = false;
        using (RegistryKey key = Registry.CurrentUser.OpenSubKey(@"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")) { light = key != null && Convert.ToInt32(key.GetValue("AppsUseLightTheme", 0)) == 1; }
        if (!light) return;
        root.BackColor = Color.FromArgb(246, 247, 248);
        root.ForeColor = Color.FromArgb(12, 17, 25);
        foreach (Control child in root.Controls) {
            if (child is Label) child.ForeColor = child.ForeColor.R > 240 && child.ForeColor.G < 120 ? Color.FromArgb(212, 16, 46) : Color.FromArgb(74, 85, 101);
        }
    }
}

public class BrandProgressBar : Control
{
    private int value;
    public ProgressBarStyle Style { get; set; }
    public int Value { get { return value; } set { this.value = Math.Max(0, Math.Min(100, value)); Invalidate(); } }
    public BrandProgressBar() { DoubleBuffered = true; AccessibleName = "Installation progress"; }
    protected override void OnPaint(PaintEventArgs e) {
        using (Brush track = new SolidBrush(Color.FromArgb(36, 36, 40))) e.Graphics.FillRectangle(track, ClientRectangle);
        using (Brush fill = new SolidBrush(Color.FromArgb(255, 37, 68))) e.Graphics.FillRectangle(fill, 0, 0, Width * value / 100, Height);
    }
}

public class SetupForm : Form
{
    private Label lblTitle;
    private Label lblStatus;
    private BrandProgressBar progressBar;
    private System.Windows.Forms.Timer timer;
    private int step = 0;
    private string installDir;
    private string exeDest;
    private TextBox installPath;
    private CheckBox desktopShortcut, startMenuShortcut, startAtLogin, explorerMenu;
    private Button installButton, browseButton, cancelButton;

    [DllImport("user32.dll")]
    public static extern int SendMessage(IntPtr hWnd, int Msg, int wParam, int lParam);
    [DllImport("user32.dll")]
    public static extern bool ReleaseCapture();

    public SetupForm(bool preview = false)
    {
        this.FormBorderStyle = FormBorderStyle.FixedDialog;
        this.MaximizeBox = false;
        this.MinimizeBox = false;
        this.AutoScaleMode = AutoScaleMode.Dpi;
        this.StartPosition = FormStartPosition.CenterScreen;
        this.ClientSize = new Size(560, 470);
        this.BackColor = Color.FromArgb(7, 7, 8); // #070708
        this.ForeColor = Color.FromArgb(246, 247, 248); // #f6f7f8
        this.DoubleBuffered = true;
        this.Text = "gnomcode Setup";
        this.Font = new Font("Segoe UI", 9);
        using (Stream icon = Assembly.GetExecutingAssembly().GetManifestResourceStream("gnomcode.ico")) {
            if (icon != null) using (var embedded = new Icon(icon)) this.Icon = (Icon)embedded.Clone();
        }

        installDir = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Gnomcode");
        using (RegistryKey key = Registry.CurrentUser.OpenSubKey(@"Software\Microsoft\Windows\CurrentVersion\Uninstall\Gnomcode")) {
            if (key != null) installDir = key.GetValue("InstallLocation", installDir).ToString();
        }
        exeDest = Path.Combine(installDir, "gnomcode.exe");

        // Draggable form
        this.MouseDown += (s, e) => {
            if (e.Button == MouseButtons.Left) {
                ReleaseCapture();
                SendMessage(Handle, 0xA1, 0x2, 0);
            }
        };

        // Title
        lblTitle = new Label();
        lblTitle.Text = "gnomcode";
        lblTitle.Font = new Font("Segoe UI", 15, FontStyle.Bold);
        lblTitle.ForeColor = Color.White;
        lblTitle.Location = new Point(88, 38);
        lblTitle.AutoSize = true;
        this.Controls.Add(lblTitle);

        Label lblSub = new Label();
        lblSub.Text = "Your agents. One view.";
        lblSub.Font = new Font("Segoe UI", 9.5f, FontStyle.Regular);
        lblSub.ForeColor = Color.FromArgb(169, 180, 194); // #a9b4c2
        lblSub.Location = new Point(90, 72);
        lblSub.AutoSize = true;
        this.Controls.Add(lblSub);

        // Status
        lblStatus = new Label();
        lblStatus.Text = "Choose where to install gnomcode for your Windows account.";
        lblStatus.Font = new Font("Segoe UI", 9.0f, FontStyle.Regular);
        lblStatus.ForeColor = Color.FromArgb(169, 180, 194);
        lblStatus.Location = new Point(36, 105);
        lblStatus.Size = new Size(490, 36);
        this.Controls.Add(lblStatus);

        // Progress Bar
        progressBar = new BrandProgressBar();
        progressBar.Location = new Point(36, 372);
        progressBar.Size = new Size(488, 10);
        progressBar.Style = ProgressBarStyle.Continuous;
        progressBar.Value = 0;
        this.Controls.Add(progressBar);

        // Version badge
        Label lblVer = new Label();
        lblVer.Text = "v0.9.4  ·  64-bit Native";
        lblVer.Font = new Font("Segoe UI", 8.0f, FontStyle.Regular);
        lblVer.ForeColor = Color.FromArgb(141, 153, 170);
        lblVer.Location = new Point(36, 431);
        lblVer.AutoSize = true;
        this.Controls.Add(lblVer);

        Label pathLabel = new Label { Text = "Install location", Location = new Point(36, 146), AutoSize = true };
        Controls.Add(pathLabel);
        installPath = new TextBox { Name = "installPath", Text = installDir, Location = new Point(36, 170), Size = new Size(382, 26), BackColor = SystemColors.Window, ForeColor = SystemColors.WindowText };
        Controls.Add(installPath);
        browseButton = new Button { Text = "Browse...", Location = new Point(428, 168), Size = new Size(96, 30) };
        browseButton.Click += (s, e) => { using (var dialog = new FolderBrowserDialog { Description = "Choose a dedicated gnomcode installation folder", SelectedPath = installPath.Text }) { if (dialog.ShowDialog(this) == DialogResult.OK) installPath.Text = dialog.SelectedPath; } };
        Controls.Add(browseButton);
        desktopShortcut = AddOption("desktopShortcut", "Create a desktop shortcut", 216, true);
        startMenuShortcut = AddOption("startMenuShortcut", "Add to the Start menu", 248, true);
        startAtLogin = AddOption("startAtLogin", "Start gnomcode when I sign in to Windows", 280, false);
        explorerMenu = AddOption("explorerMenu", "Add Open with Gnomcode to Explorer folders", 312, true);
        Controls.Add(new Label { Text = "Windows 11: available under Show more options.", Location = new Point(58, 341), AutoSize = true });
        installButton = new Button { Name = "installButton", Text = "Install", Location = new Point(324, 398), Size = new Size(96, 32) };
        installButton.Click += (s, e) => BeginInstall();
        Controls.Add(installButton);
        cancelButton = new Button { Text = "Cancel", Location = new Point(428, 398), Size = new Size(96, 32) };
        cancelButton.Click += (s, e) => Close();
        Controls.Add(cancelButton);
        foreach (Button button in new[] { browseButton, installButton, cancelButton }) { button.ForeColor = SystemColors.ControlText; button.BackColor = SystemColors.Control; button.UseVisualStyleBackColor = true; }
        AcceptButton = installButton;
        CancelButton = cancelButton;

        timer = new System.Windows.Forms.Timer();
        timer.Interval = 120;
        timer.Tick += OnTick;
        DeviceTheme.Apply(this);
        if (preview) lblStatus.Text = "Choose installation options. Preview does not install.";
        if (preview) installButton.Enabled = false;
    }

    private CheckBox AddOption(string name, string text, int top, bool selected) {
        var option = new CheckBox { Name = name, Text = text, Location = new Point(36, top), Size = new Size(488, 28), Checked = selected };
        Controls.Add(option);
        return option;
    }

    private void BeginInstall() {
        try {
            installDir = Path.GetFullPath(installPath.Text.Trim());
            InstallerIntegration.ValidateInstallDirectory(installDir);
            exeDest = Path.Combine(installDir, "gnomcode.exe");
            foreach (Control control in new Control[] { installPath, browseButton, desktopShortcut, startMenuShortcut, startAtLogin, explorerMenu, installButton, cancelButton }) control.Enabled = false;
            ControlBox = false;
            timer.Start();
        } catch (Exception ex) { MessageBox.Show(this, ex.Message, "Choose install location", MessageBoxButtons.OK, MessageBoxIcon.Warning); }
    }

    protected override void OnPaint(PaintEventArgs e)
    {
        base.OnPaint(e);
        Graphics g = e.Graphics;
        g.SmoothingMode = SmoothingMode.AntiAlias;

        // Draw border
        using (Pen p = new Pen(Color.FromArgb(36, 36, 40), 1.5f)) {
            g.DrawRectangle(p, 0, 0, this.Width - 1, this.Height - 1);
        }

        // Draw the solid triangle-and-square gnomcode mark.
        using (Stream mark = Assembly.GetExecutingAssembly().GetManifestResourceStream("gnomcode-mark.png")) {
            if (mark != null) using (Image image = Image.FromStream(mark)) g.DrawImage(image, 36, 34, 42, 42);
        }
    }

    private void OnTick(object sender, EventArgs e)
    {
        try {
            step++;
            if (step == 2) {
                lblStatus.Text = "Creating installation directory...";
                progressBar.Value = 20;
                Directory.CreateDirectory(installDir);
            } else if (step == 4) {
                lblStatus.Text = "Installing gnomcode engine...";
                progressBar.Value = 45;
                ExtractEmbeddedFiles();
            } else if (step == 6) {
                lblStatus.Text = "Registering Windows Add or Remove Programs...";
                progressBar.Value = 65;
                RegisterUninstallEntry();
            } else if (step == 8) {
                lblStatus.Text = "Creating desktop and Start menu shortcuts...";
                progressBar.Value = 80;
                CreateShortcuts();
            } else if (step == 10) {
                lblStatus.Text = "Registering autostart and system tray...";
                progressBar.Value = 92;
                ConfigureAutostart();
                InstallerIntegration.ConfigureExplorer(Registry.CurrentUser, installDir, explorerMenu.Checked);
            } else if (step == 12) {
                lblStatus.Text = "Launching gnomcode...";
                progressBar.Value = 100;
            } else if (step == 14) {
                timer.Stop();
                if (Environment.GetEnvironmentVariable("GNOMCODE_SETUP_NO_LAUNCH") != "1") LaunchGnomcode();
                this.Close();
            }
        } catch (Exception ex) {
            timer.Stop();
            Environment.ExitCode = 1;
            lblStatus.Text = "Installation failed. Close setup and try again.";
            ControlBox = true;
            cancelButton.Enabled = true;
            MessageBox.Show(this, ex.Message, "gnomcode Setup", MessageBoxButtons.OK, MessageBoxIcon.Error);
        }
    }

    private void ExtractEmbeddedFiles()
    {
        foreach (Process process in Process.GetProcessesByName("gnomcode")) {
            try {
                if (String.Equals(process.MainModule.FileName, exeDest, StringComparison.OrdinalIgnoreCase)) {
                    process.Kill();
                    process.WaitForExit(3000);
                }
            } catch {}
            process.Dispose();
        }
        InstallerIntegration.StopTrayProcesses(installDir);

        Assembly asm = Assembly.GetExecutingAssembly();
        foreach (string resName in asm.GetManifestResourceNames()) {
            if (resName == "desktop.zip") {
                using(Stream desktop=asm.GetManifestResourceStream(resName)) InstallerIntegration.ExtractDesktop(desktop,installDir);
                continue;
            }
            string fileName = resName;
            if (resName.StartsWith("payload.")) {
                fileName = resName.Substring("payload.".Length);
            }
            string dest = Path.Combine(installDir, fileName);
            using (Stream s = asm.GetManifestResourceStream(resName))
            using (FileStream fs = new FileStream(dest, FileMode.Create, FileAccess.Write)) {
                s.CopyTo(fs);
            }
        }

        // Copy setup binary itself as Gnomcode-Uninstall.exe
        string currentExe = Process.GetCurrentProcess().MainModule.FileName;
        string uninstallerDest = Path.Combine(installDir, "Gnomcode-Uninstall.exe");
        File.Copy(currentExe, uninstallerDest, true);
    }

    private void RegisterUninstallEntry()
    {

        using(RegistryKey appId=Registry.CurrentUser.CreateSubKey(@"Software\Classes\AppUserModelId\app.gnomcode.desktop")) {
            appId.SetValue("DisplayName","Gnomcode");
            appId.SetValue("IconUri",Path.Combine(installDir,"gnomcode.ico"));
        }
        using (RegistryKey key = Registry.CurrentUser.CreateSubKey(@"Software\Microsoft\Windows\CurrentVersion\Uninstall\Gnomcode")) {
            if (key != null) {
                string uninstallExe = Path.Combine(installDir, "Gnomcode-Uninstall.exe");
                string ico = Path.Combine(installDir, "gnomcode.ico");
                key.SetValue("DisplayName", "gnomcode");
                key.SetValue("DisplayVersion", "0.9.4");
                key.SetValue("Publisher", "gnomcode");
                key.SetValue("DisplayIcon", ico + ",0");
                key.SetValue("InstallLocation", installDir);
                key.SetValue("UninstallString", "\"" + uninstallExe + "\" --uninstall");
                key.SetValue("QuietUninstallString", "\"" + uninstallExe + "\" --uninstall --quiet");
                key.SetValue("EstimatedSize", 16500, RegistryValueKind.DWord);
                key.SetValue("NoModify", 1, RegistryValueKind.DWord);
                key.SetValue("NoRepair", 1, RegistryValueKind.DWord);
                key.SetValue("URLInfoAbout", "https://gnomcode.vercel.app");
                key.SetValue("HelpLink", "https://gnomcode.vercel.app/docs");
            }
        }
    }

    private void CreateShortcuts()
    {
        Type shellType = Type.GetTypeFromProgID("WScript.Shell");
        dynamic shell = Activator.CreateInstance(shellType);
        string vbs = Path.Combine(installDir, "open-console.vbs");
        string ico = Path.Combine(installDir, "gnomcode.ico");

        // Replace legacy shortcuts without changing the installation or data directory.
        foreach (string folder in new string[] { Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory), Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), @"Microsoft\Windows\Start Menu\Programs") }) {
            foreach (string name in new string[] { "Gnomcode.lnk", "Gnomcode Console.lnk" }) { string old = Path.Combine(folder, name); if (File.Exists(old)) File.Delete(old); }
        }

        // Desktop shortcut
        string desktopDir = Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory);
        string desktopLnk = Path.Combine(desktopDir, "gnomcode.lnk");
        if (desktopShortcut.Checked) {
            dynamic scDesktop = shell.CreateShortcut(desktopLnk);
            scDesktop.TargetPath = Path.Combine(installDir,"desktop","gnomcode-desktop.exe");
            scDesktop.Arguments = "";
            scDesktop.IconLocation = ico + ",0";
            scDesktop.Description = "gnomcode";
            scDesktop.WorkingDirectory = installDir;
            scDesktop.Save();
        }

        // Start Menu shortcut
        string startMenuDir = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), @"Microsoft\Windows\Start Menu\Programs");
        string startLnk = Path.Combine(startMenuDir, "gnomcode.lnk");
        if (startMenuShortcut.Checked) {
            dynamic scStart = shell.CreateShortcut(startLnk);
            scStart.TargetPath = Path.Combine(installDir,"desktop","gnomcode-desktop.exe");
            scStart.Arguments = "";
            scStart.IconLocation = ico + ",0";
            scStart.Description = "gnomcode";
            scStart.WorkingDirectory = installDir;
            scStart.Save();
        }

        // Add to User PATH
        string curPath = Environment.GetEnvironmentVariable("Path", EnvironmentVariableTarget.User) ?? "";
        if (!Array.Exists(curPath.Split(';'), part => String.Equals(part.Trim(), installDir, StringComparison.OrdinalIgnoreCase))) {
            Environment.SetEnvironmentVariable("Path", curPath + ";" + installDir, EnvironmentVariableTarget.User);
        }
    }

    private void ConfigureAutostart()
    {
        using (RegistryKey key = Registry.CurrentUser.CreateSubKey(@"Software\Microsoft\Windows\CurrentVersion\Run")) {
            if (key != null) {
                string vbs = Path.Combine(installDir, "run-tray.vbs");
                if (startAtLogin.Checked) key.SetValue("Gnomcode", "wscript.exe \"" + vbs + "\"");
                else key.DeleteValue("Gnomcode", false);
            }
        }
    }

    private void LaunchGnomcode()
    {
        try {
            string runTray = Path.Combine(installDir, "run-tray.vbs");
            string openConsole = Path.Combine(installDir, "open-console.vbs");

            string desktop=Path.Combine(installDir,"desktop","gnomcode-desktop.exe");
            if(File.Exists(desktop)) {
                Process.Start(new ProcessStartInfo {FileName=desktop,UseShellExecute=false,CreateNoWindow=true});
                return;
            }
            if (File.Exists(runTray)) {
                System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo {
                    FileName = "wscript.exe",
                    Arguments = "\"" + runTray + "\"",
                    UseShellExecute = false,
                    CreateNoWindow = true
                });
            }

            if (File.Exists(openConsole)) {
                System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo {
                    FileName = "wscript.exe",
                    Arguments = "\"" + openConsole + "\"",
                    UseShellExecute = false,
                    CreateNoWindow = true
                });
            }
        } catch {}
    }
}

public static class InstallerIntegration
{
    public static string DesktopPayloadPath(string installDir,string entry) {
        if(String.IsNullOrEmpty(entry) || Path.IsPathRooted(entry) || entry.IndexOf(':') >= 0) throw new IOException("Invalid desktop payload path");
        string root=Path.GetFullPath(Path.Combine(installDir,"desktop"))+Path.DirectorySeparatorChar;
        string dest=Path.GetFullPath(Path.Combine(root,entry));
        if(!dest.StartsWith(root,StringComparison.OrdinalIgnoreCase)) throw new IOException("Desktop payload escapes install folder");
        if(File.Exists(dest) && (File.GetAttributes(dest)&FileAttributes.ReparsePoint)!=0) throw new IOException("Desktop payload cannot replace linked files");
        for(string dir=Path.GetDirectoryName(dest);dir!=null && dir.Length>=root.Length-1;dir=Path.GetDirectoryName(dir)) {
            if(Directory.Exists(dir) && (File.GetAttributes(dir)&FileAttributes.ReparsePoint)!=0) throw new IOException("Desktop payload cannot use linked directories");
        }
        return dest;
    }

    public static void ExtractDesktop(Stream stream,string installDir) {
        using(var archive=new ZipArchive(stream,ZipArchiveMode.Read)) {
            foreach(var entry in archive.Entries) {
                if(entry.Name.Length==0)continue;
                string dest=DesktopPayloadPath(installDir,entry.FullName);
                Directory.CreateDirectory(Path.GetDirectoryName(dest));
                using(Stream source=entry.Open()) using(Stream output=File.Create(dest)) source.CopyTo(output);
            }
        }
    }

    public static void ValidateInstallDirectory(string installDir) {
        if (installDir.StartsWith(@"\\") || installDir == Path.GetPathRoot(installDir)) throw new IOException("Choose a dedicated folder on a local drive.");
        if (!Directory.Exists(installDir) || Directory.GetFileSystemEntries(installDir).Length == 0) return;
        bool native = File.Exists(Path.Combine(installDir, "gnomcode.exe")) &&
            (File.Exists(Path.Combine(installDir, "Gnomcode-Uninstall.exe")) || File.Exists(Path.Combine(installDir, "open-console.vbs")));
        bool desktop = File.Exists(Path.Combine(installDir, "gnomcode-desktop.exe")) && File.Exists(Path.Combine(installDir, "Update.exe"));
        if (!native && !desktop) throw new IOException("Choose an empty folder or an existing gnomcode installation.");
    }

    public static void RemovePayloadFiles(Assembly payload, string installDir) {
        using(Stream stream=payload.GetManifestResourceStream("desktop.zip")) {
            if(stream!=null)using(var archive=new ZipArchive(stream,ZipArchiveMode.Read)) {
                foreach(var entry in archive.Entries) {
                    if(entry.Name.Length==0)continue;
                    string dest=DesktopPayloadPath(installDir,entry.FullName);
                    if(File.Exists(dest))File.Delete(dest);
                }
            }
        }
        foreach (string resource in payload.GetManifestResourceNames()) {
            string name = resource.StartsWith("payload.") ? resource.Substring(8) : resource;
            if (Path.GetFileName(name) != name) continue;
            string file = Path.Combine(installDir, name);
            if (File.Exists(file)) File.Delete(file);
        }
    }

    public static void StopTrayProcesses(string installDir) {
        string desktop=Path.GetFullPath(Path.Combine(installDir,"desktop"))+Path.DirectorySeparatorChar;
        foreach(Process process in Process.GetProcesses()) {
            try {
                if(process.ProcessName!="gnomcode-desktop" && process.ProcessName!="gnomcode")continue;
                string exe=process.MainModule.FileName;
                if(exe.StartsWith(desktop,StringComparison.OrdinalIgnoreCase)) {process.Kill();process.WaitForExit(3000);}
            }catch{} finally{process.Dispose();}
        }
        string tray = Path.Combine(installDir, "tray.ps1").Replace("'", "''");
        string profile = Path.Combine(installDir, "AppProfile").Replace("'", "''");
        string script = "Get-CimInstance Win32_Process | Where-Object { $_.ProcessId -ne $PID -and $_.CommandLine -and ((($_.Name -eq 'powershell.exe' -or $_.Name -eq 'pwsh.exe') -and $_.CommandLine.IndexOf('" + tray + "', [StringComparison]::OrdinalIgnoreCase) -ge 0) -or (($_.Name -eq 'msedge.exe' -or $_.Name -eq 'chrome.exe') -and $_.CommandLine.IndexOf('" + profile + "', [StringComparison]::OrdinalIgnoreCase) -ge 0)) } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }";
        using (Process cleanup = Process.Start(new ProcessStartInfo {
            FileName = "powershell.exe", Arguments = "-NoProfile -EncodedCommand " + Convert.ToBase64String(System.Text.Encoding.Unicode.GetBytes(script)),
            UseShellExecute = false, CreateNoWindow = true, WindowStyle = ProcessWindowStyle.Hidden
        })) { if (cleanup != null) cleanup.WaitForExit(5000); }
    }

    public static void ConfigureExplorer(RegistryKey user, string installDir, bool enabled) {
        foreach (string target in new[] { @"Directory\shell\Gnomcode", @"Directory\Background\shell\Gnomcode" }) {
            string path = @"Software\Classes\" + target;
            if (!enabled) { user.DeleteSubKeyTree(path, false); continue; }
            using (RegistryKey key = user.CreateSubKey(path)) {
                key.SetValue("", "Open with Gnomcode");
                key.SetValue("Icon", "\"" + Path.Combine(installDir, "gnomcode.ico") + "\"");
                using (RegistryKey command = key.CreateSubKey("command")) {
                    string powershell = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.System), @"WindowsPowerShell\v1.0\powershell.exe");
                    command.SetValue("", "\"" + powershell + "\" -NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -File \"" + Path.Combine(installDir, "open-folder.ps1") + "\" -Folder \"" + (target.Contains("Background") ? "%V" : "%1") + "\\.\"");
                }
            }
        }
    }
}

public class UninstallForm : Form
{
    private Label lblTitle;
    private Label lblStatus;
    private BrandProgressBar progressBar;
    private System.Windows.Forms.Timer timer;
    private int step = 0;
    private string installDir;
    private bool isQuiet;

    [DllImport("user32.dll")]
    public static extern int SendMessage(IntPtr hWnd, int Msg, int wParam, int lParam);
    [DllImport("user32.dll")]
    public static extern bool ReleaseCapture();

    public UninstallForm(bool quiet)
    {
        this.isQuiet = quiet;
        installDir = Path.GetDirectoryName(Assembly.GetExecutingAssembly().Location);

        this.FormBorderStyle = FormBorderStyle.None;
        this.StartPosition = FormStartPosition.CenterScreen;
        this.Size = new Size(480, 240);
        this.BackColor = Color.FromArgb(7, 7, 8); // #070708
        this.ForeColor = Color.FromArgb(246, 247, 248); // #f6f7f8
        this.DoubleBuffered = true;
        this.Text = "gnomcode";

        this.MouseDown += (s, e) => {
            if (e.Button == MouseButtons.Left) {
                ReleaseCapture();
                SendMessage(Handle, 0xA1, 0x2, 0);
            }
        };

        lblTitle = new Label();
        lblTitle.Text = "Uninstall gnomcode";
        lblTitle.Font = new Font("Segoe UI", 14, FontStyle.Bold);
        lblTitle.ForeColor = Color.White;
        lblTitle.Location = new Point(88, 38);
        lblTitle.AutoSize = true;
        this.Controls.Add(lblTitle);

        Label lblSub = new Label();
        lblSub.Text = "Removing native engine, tray services, and shortcuts...";
        lblSub.Font = new Font("Segoe UI", 9.0f, FontStyle.Regular);
        lblSub.ForeColor = Color.FromArgb(169, 180, 194);
        lblSub.Location = new Point(90, 70);
        lblSub.AutoSize = true;
        this.Controls.Add(lblSub);

        lblStatus = new Label();
        lblStatus.Text = "Preparing uninstallation...";
        lblStatus.Font = new Font("Segoe UI", 9.0f, FontStyle.Regular);
        lblStatus.ForeColor = Color.FromArgb(255, 97, 119); // Red-tinted status
        lblStatus.Location = new Point(36, 125);
        lblStatus.Size = new Size(408, 22);
        this.Controls.Add(lblStatus);

        progressBar = new BrandProgressBar();
        progressBar.Location = new Point(36, 152);
        progressBar.Size = new Size(408, 14);
        progressBar.Style = ProgressBarStyle.Continuous;
        progressBar.Value = 10;
        this.Controls.Add(progressBar);

        timer = new System.Windows.Forms.Timer();
        timer.Interval = 140;
        DeviceTheme.Apply(this);
        timer.Tick += OnTick;
        timer.Start();
    }

    protected override void OnPaint(PaintEventArgs e)
    {
        base.OnPaint(e);
        Graphics g = e.Graphics;
        g.SmoothingMode = SmoothingMode.AntiAlias;

        using (Pen p = new Pen(Color.FromArgb(36, 36, 40), 1.5f)) {
            g.DrawRectangle(p, 0, 0, this.Width - 1, this.Height - 1);
        }

        using (Stream mark = Assembly.GetExecutingAssembly().GetManifestResourceStream("gnomcode-mark.png")) {
            if (mark != null) using (Image image = Image.FromStream(mark)) g.DrawImage(image, 36, 34, 42, 42);
        }
    }

    private void OnTick(object sender, EventArgs e)
    {
        step++;
        if (step == 2) {
            lblStatus.Text = "Stopping gnomcode processes...";
            progressBar.Value = 25;
            KillGnomcodeProcesses();
        } else if (step == 4) {
            lblStatus.Text = "Removing autostart and environment settings...";
            progressBar.Value = 50;
            RemoveAutostartAndPath();
        } else if (step == 6) {
            lblStatus.Text = "Removing desktop and Start menu shortcuts...";
            progressBar.Value = 75;
            RemoveShortcuts();
        } else if (step == 8) {
            lblStatus.Text = "Unregistering from Windows Add or Remove Programs...";
            progressBar.Value = 90;
            RemoveUninstallRegistration();
        } else if (step == 10) {
            lblStatus.Text = "Purging installation files...";
            progressBar.Value = 100;
            ScheduleFileCleanup();
        } else if (step == 12) {
            timer.Stop();
            if (!isQuiet) {
                MessageBox.Show("gnomcode was successfully uninstalled from your computer.", "gnomcode Uninstall", MessageBoxButtons.OK, MessageBoxIcon.Information);
            }
            this.Close();
        }
    }

    private void KillGnomcodeProcesses()
    {
        try {
            foreach (Process p in Process.GetProcessesByName("gnomcode")) {
                try { if (String.Equals(p.MainModule.FileName, Path.Combine(installDir, "gnomcode.exe"), StringComparison.OrdinalIgnoreCase)) { p.Kill(); p.WaitForExit(2000); } } catch {}
                p.Dispose();
            }
            InstallerIntegration.StopTrayProcesses(installDir);
        } catch {}
    }

    private void RemoveAutostartAndPath()
    {
        try {
            using (RegistryKey key = Registry.CurrentUser.OpenSubKey(@"Software\Microsoft\Windows\CurrentVersion\Run", true)) {
                if (key != null) {
                    key.DeleteValue("Gnomcode", false);
                }
            }
        } catch {}

        try {
            string curPath = Environment.GetEnvironmentVariable("Path", EnvironmentVariableTarget.User) ?? "";
            if (curPath.Contains(installDir)) {
                string[] parts = curPath.Split(new char[] { ';' }, StringSplitOptions.RemoveEmptyEntries);
                string newPath = "";
                foreach (string part in parts) {
                    if (!String.Equals(part.Trim(), installDir.Trim(), StringComparison.OrdinalIgnoreCase)) {
                        newPath += (newPath.Length > 0 ? ";" : "") + part;
                    }
                }
                Environment.SetEnvironmentVariable("Path", newPath, EnvironmentVariableTarget.User);
            }
        } catch {}
    }

    private void RemoveShortcuts()
    {
        try {
            string desktopLnk = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory), "gnomcode.lnk");
            if (File.Exists(desktopLnk)) File.Delete(desktopLnk);

            string startLnk = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), @"Microsoft\Windows\Start Menu\Programs\gnomcode.lnk");
            if (File.Exists(startLnk)) File.Delete(startLnk);
        } catch {}
    }

    private void RemoveUninstallRegistration()
    {
        try {
            InstallerIntegration.ConfigureExplorer(Registry.CurrentUser, installDir, false);
            Registry.CurrentUser.DeleteSubKeyTree(@"Software\Classes\AppUserModelId\app.gnomcode.desktop",false);
            Registry.CurrentUser.DeleteSubKeyTree(@"Software\Microsoft\Windows\CurrentVersion\Uninstall\Gnomcode", false);
        } catch {}
    }

    private void ScheduleFileCleanup()
    {
        try {
            // Remove only shipped files; never recursively delete a user-selected directory.
            string currentExe = Path.GetFullPath(Assembly.GetExecutingAssembly().Location);
            if (!String.Equals(Path.GetFileName(currentExe), "Gnomcode-Uninstall.exe", StringComparison.OrdinalIgnoreCase)) return;
            InstallerIntegration.RemovePayloadFiles(Assembly.GetExecutingAssembly(), installDir);
            string cleanup = "Start-Sleep -Seconds 2; Remove-Item -LiteralPath '" + currentExe.Replace("'", "''") + "' -Force; if (-not (Get-ChildItem -LiteralPath '" + installDir.Replace("'", "''") + "' -Force)) { Remove-Item -LiteralPath '" + installDir.Replace("'", "''") + "' }";
            Process.Start(new ProcessStartInfo {
                FileName = "powershell.exe",
                Arguments = "-NoProfile -EncodedCommand " + Convert.ToBase64String(System.Text.Encoding.Unicode.GetBytes(cleanup)),
                UseShellExecute = false,
                CreateNoWindow = true,
                WindowStyle = ProcessWindowStyle.Hidden
            });
        } catch {}
    }
}

public class Program
{
    [STAThread]
    static void Main(string[] args)
    {
        Application.EnableVisualStyles();
        Application.SetCompatibleTextRenderingDefault(false);

        string exeName = Process.GetCurrentProcess().MainModule.FileName;
        bool isUninstall = false;
        bool isQuiet = false;
        bool isPreview = false;

        foreach (string arg in args) {
            string a = arg.ToLowerInvariant();
            if (a == "--preview") isPreview = true;
            if (a == "--uninstall" || a == "/uninstall" || a == "-uninstall") {
                isUninstall = true;
            }
            if (a == "--quiet" || a == "/quiet" || a == "-quiet" || a == "-q" || a == "/q") {
                isQuiet = true;
            }
        }

        if (exeName.ToLowerInvariant().Contains("uninstall")) {
            isUninstall = true;
        }

        if (isUninstall) {
            if (!String.Equals(Path.GetFileName(exeName), "Gnomcode-Uninstall.exe", StringComparison.OrdinalIgnoreCase)) {
                MessageBox.Show("Use Installed apps in Windows Settings to uninstall gnomcode.", "gnomcode Setup", MessageBoxButtons.OK, MessageBoxIcon.Information);
                Environment.ExitCode = 1;
                return;
            }
            if (!isQuiet) {
                DialogResult dr = MessageBox.Show(
                    "Are you sure you want to completely remove gnomcode from your computer?",
                    "gnomcode Uninstall",
                    MessageBoxButtons.YesNo,
                    MessageBoxIcon.Question
                );
                if (dr != DialogResult.Yes) {
                    return;
                }
            }
            Application.Run(new UninstallForm(isQuiet));
        } else {
            Application.Run(new SetupForm(isPreview));
        }
    }
}
