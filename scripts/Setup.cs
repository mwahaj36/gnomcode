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

[assembly: AssemblyTitle("spawnt Setup")]
[assembly: AssemblyProduct("spawnt")]
[assembly: AssemblyCompany("spawnt")]
[assembly: AssemblyVersion("1.5.0.0")]
[assembly: AssemblyFileVersion("1.5.0.0")]

public static class DeviceTheme
{
    public static void Apply(Control root) {
        bool light = false;
        using (RegistryKey key = Registry.CurrentUser.OpenSubKey(@"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")) { light = key != null && Convert.ToInt32(key.GetValue("AppsUseLightTheme", 0)) == 1; }
        if (!light) return;
        root.BackColor = Color.FromArgb(246, 247, 248);
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

    [DllImport("user32.dll")]
    public static extern int SendMessage(IntPtr hWnd, int Msg, int wParam, int lParam);
    [DllImport("user32.dll")]
    public static extern bool ReleaseCapture();

    public SetupForm(bool preview = false)
    {
        this.FormBorderStyle = FormBorderStyle.None;
        this.StartPosition = FormStartPosition.CenterScreen;
        this.Size = new Size(480, 270);
        this.BackColor = Color.FromArgb(7, 7, 8); // #070708
        this.ForeColor = Color.FromArgb(246, 247, 248); // #f6f7f8
        this.DoubleBuffered = true;
        this.Text = "spawnt";

        installDir = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "FleetHUD");
        exeDest = Path.Combine(installDir, "fleethud.exe");

        // Draggable form
        this.MouseDown += (s, e) => {
            if (e.Button == MouseButtons.Left) {
                ReleaseCapture();
                SendMessage(Handle, 0xA1, 0x2, 0);
            }
        };

        // Title
        lblTitle = new Label();
        lblTitle.Text = "spawnt";
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
        lblStatus.Text = "Initializing setup...";
        lblStatus.Font = new Font("Segoe UI", 9.0f, FontStyle.Regular);
        lblStatus.ForeColor = Color.FromArgb(255, 97, 119); // #ff6177
        lblStatus.Location = new Point(36, 138);
        lblStatus.Size = new Size(408, 22);
        this.Controls.Add(lblStatus);

        // Progress Bar
        progressBar = new BrandProgressBar();
        progressBar.Location = new Point(36, 166);
        progressBar.Size = new Size(408, 14);
        progressBar.Style = ProgressBarStyle.Continuous;
        progressBar.Value = 10;
        this.Controls.Add(progressBar);

        // Version badge
        Label lblVer = new Label();
        lblVer.Text = "v1.5.0  ·  64-bit Native";
        lblVer.Font = new Font("Segoe UI", 8.0f, FontStyle.Regular);
        lblVer.ForeColor = Color.FromArgb(141, 153, 170);
        lblVer.Location = new Point(36, 225);
        lblVer.AutoSize = true;
        this.Controls.Add(lblVer);

        timer = new System.Windows.Forms.Timer();
        timer.Interval = 120;
        timer.Tick += OnTick;
        DeviceTheme.Apply(this);
        if (!preview) timer.Start();
        else { lblStatus.Text = "Installing spawnt"; progressBar.Value = 60; }
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

        // Draw the solid triangle-and-square spawnt mark.
        using (Stream mark = Assembly.GetExecutingAssembly().GetManifestResourceStream("spawnt-mark.png")) {
            if (mark != null) using (Image image = Image.FromStream(mark)) g.DrawImage(image, 36, 34, 42, 42);
        }
    }

    private void OnTick(object sender, EventArgs e)
    {
        step++;
        if (step == 2) {
            lblStatus.Text = "Creating installation directory...";
            progressBar.Value = 20;
            Directory.CreateDirectory(installDir);
        } else if (step == 4) {
            lblStatus.Text = "Installing spawnt engine...";
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
        } else if (step == 12) {
            lblStatus.Text = "Launching spawnt...";
            progressBar.Value = 100;
        } else if (step == 14) {
            timer.Stop();
            if (Environment.GetEnvironmentVariable("FLEETHUD_SETUP_NO_LAUNCH") != "1") LaunchFleetHUD();
            this.Close();
        }
    }

    private void ExtractEmbeddedFiles()
    {
        try {
            foreach (Process process in Process.GetProcessesByName("fleethud")) {
                try {
                    if (String.Equals(process.MainModule.FileName, exeDest, StringComparison.OrdinalIgnoreCase)) {
                        process.Kill();
                        process.WaitForExit(3000);
                    }
                } catch {}
                process.Dispose();
            }
            using (Process cleanup = Process.Start(new ProcessStartInfo {
                FileName = "powershell.exe",
                Arguments = "-NoProfile -Command \"Get-CimInstance Win32_Process | Where-Object { (($_.Name -like 'powershell*' -or $_.Name -like 'pwsh*') -and $_.ProcessId -ne $PID -and $_.CommandLine -like ('*' + $env:LOCALAPPDATA + '\\FleetHUD\\tray.ps1*')) -or ($_.CommandLine -like ('*' + $env:LOCALAPPDATA + '\\FleetHUD\\AppProfile*')) } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }\"",
                UseShellExecute = false,
                CreateNoWindow = true,
                WindowStyle = ProcessWindowStyle.Hidden
            })) {
                if (cleanup != null) cleanup.WaitForExit(5000);
            }

            Assembly asm = Assembly.GetExecutingAssembly();
            foreach (string resName in asm.GetManifestResourceNames()) {
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

            // Copy setup binary itself as FleetHUD-Uninstall.exe
            string currentExe = Process.GetCurrentProcess().MainModule.FileName;
            string uninstallerDest = Path.Combine(installDir, "FleetHUD-Uninstall.exe");
            if (File.Exists(currentExe)) {
                try {
                    File.Copy(currentExe, uninstallerDest, true);
                } catch {}
            }
        } catch (Exception ex) {
            timer.Stop();
            lblStatus.Text = "Installation failed";
            MessageBox.Show(ex.Message, "spawnt Setup", MessageBoxButtons.OK, MessageBoxIcon.Error);
        }
    }

    private void RegisterUninstallEntry()
    {
        try {
            using (RegistryKey key = Registry.CurrentUser.CreateSubKey(@"Software\Microsoft\Windows\CurrentVersion\Uninstall\FleetHUD")) {
                if (key != null) {
                    string uninstallExe = Path.Combine(installDir, "FleetHUD-Uninstall.exe");
                    string ico = Path.Combine(installDir, "fleethud.ico");
                    key.SetValue("DisplayName", "spawnt");
                    key.SetValue("DisplayVersion", "1.5.0");
                    key.SetValue("Publisher", "spawnt");
                    key.SetValue("DisplayIcon", ico + ",0");
                    key.SetValue("InstallLocation", installDir);
                    key.SetValue("UninstallString", "\"" + uninstallExe + "\" --uninstall");
                    key.SetValue("QuietUninstallString", "\"" + uninstallExe + "\" --uninstall --quiet");
                    key.SetValue("EstimatedSize", 16500, RegistryValueKind.DWord);
                    key.SetValue("NoModify", 1, RegistryValueKind.DWord);
                    key.SetValue("NoRepair", 1, RegistryValueKind.DWord);
                    key.SetValue("URLInfoAbout", "https://fleethud.vercel.app");
                    key.SetValue("HelpLink", "https://fleethud.vercel.app/docs");
                }
            }
        } catch {}
    }

    private void CreateShortcuts()
    {
        try {
            Type shellType = Type.GetTypeFromProgID("WScript.Shell");
            dynamic shell = Activator.CreateInstance(shellType);
            string vbs = Path.Combine(installDir, "open-console.vbs");
            string ico = Path.Combine(installDir, "fleethud.ico");

            // Replace legacy shortcuts without changing the installation or data directory.
            foreach (string folder in new string[] { Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory), Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), @"Microsoft\Windows\Start Menu\Programs") }) {
                foreach (string name in new string[] { "FleetHUD.lnk", "FleetHUD Console.lnk" }) { string old = Path.Combine(folder, name); if (File.Exists(old)) File.Delete(old); }
            }

            // Desktop shortcut
            string desktopDir = Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory);
            string desktopLnk = Path.Combine(desktopDir, "spawnt.lnk");
            dynamic scDesktop = shell.CreateShortcut(desktopLnk);
            scDesktop.TargetPath = "wscript.exe";
            scDesktop.Arguments = "\"" + vbs + "\"";
            scDesktop.IconLocation = ico + ",0";
            scDesktop.Description = "spawnt";
            scDesktop.WorkingDirectory = installDir;
            scDesktop.Save();

            // Start Menu shortcut
            string startMenuDir = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), @"Microsoft\Windows\Start Menu\Programs");
            string startLnk = Path.Combine(startMenuDir, "spawnt.lnk");
            dynamic scStart = shell.CreateShortcut(startLnk);
            scStart.TargetPath = "wscript.exe";
            scStart.Arguments = "\"" + vbs + "\"";
            scStart.IconLocation = ico + ",0";
            scStart.Description = "spawnt";
            scStart.WorkingDirectory = installDir;
            scStart.Save();

            // Add to User PATH
            string curPath = Environment.GetEnvironmentVariable("Path", EnvironmentVariableTarget.User) ?? "";
            if (!curPath.Contains(installDir)) {
                Environment.SetEnvironmentVariable("Path", curPath + ";" + installDir, EnvironmentVariableTarget.User);
            }
        } catch {}
    }

    private void ConfigureAutostart()
    {
        try {
            using (RegistryKey key = Registry.CurrentUser.OpenSubKey(@"Software\Microsoft\Windows\CurrentVersion\Run", true)) {
                if (key != null) {
                    string vbs = Path.Combine(installDir, "run-tray.vbs");
                    key.SetValue("FleetHUD", "wscript.exe \"" + vbs + "\"");
                }
            }
        } catch {}
    }

    private void LaunchFleetHUD()
    {
        try {
            string runTray = Path.Combine(installDir, "run-tray.vbs");
            string openConsole = Path.Combine(installDir, "open-console.vbs");

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
        installDir = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "FleetHUD");

        this.FormBorderStyle = FormBorderStyle.None;
        this.StartPosition = FormStartPosition.CenterScreen;
        this.Size = new Size(480, 240);
        this.BackColor = Color.FromArgb(7, 7, 8); // #070708
        this.ForeColor = Color.FromArgb(246, 247, 248); // #f6f7f8
        this.DoubleBuffered = true;
        this.Text = "spawnt";

        this.MouseDown += (s, e) => {
            if (e.Button == MouseButtons.Left) {
                ReleaseCapture();
                SendMessage(Handle, 0xA1, 0x2, 0);
            }
        };

        lblTitle = new Label();
        lblTitle.Text = "Uninstall spawnt";
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

        using (Stream mark = Assembly.GetExecutingAssembly().GetManifestResourceStream("spawnt-mark.png")) {
            if (mark != null) using (Image image = Image.FromStream(mark)) g.DrawImage(image, 36, 34, 42, 42);
        }
    }

    private void OnTick(object sender, EventArgs e)
    {
        step++;
        if (step == 2) {
            lblStatus.Text = "Stopping spawnt processes...";
            progressBar.Value = 25;
            KillFleetHUDProcesses();
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
                MessageBox.Show("spawnt was successfully uninstalled from your computer.", "spawnt Uninstall", MessageBoxButtons.OK, MessageBoxIcon.Information);
            }
            this.Close();
        }
    }

    private void KillFleetHUDProcesses()
    {
        try {
            foreach (Process p in Process.GetProcessesByName("fleethud")) {
                try { if (String.Equals(p.MainModule.FileName, Path.Combine(installDir, "fleethud.exe"), StringComparison.OrdinalIgnoreCase)) { p.Kill(); p.WaitForExit(2000); } } catch {}
                p.Dispose();
            }
            using (Process cleanup = Process.Start(new ProcessStartInfo {
                FileName = "powershell.exe",
                Arguments = "-NoProfile -Command \"Get-CimInstance Win32_Process | Where-Object { (($_.Name -like 'powershell*' -or $_.Name -like 'pwsh*') -and $_.ProcessId -ne $PID -and $_.CommandLine -like ('*' + $env:LOCALAPPDATA + '\\FleetHUD\\tray.ps1*')) -or ($_.CommandLine -like ('*' + $env:LOCALAPPDATA + '\\FleetHUD\\AppProfile*')) } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }\"",
                UseShellExecute = false,
                CreateNoWindow = true,
                WindowStyle = ProcessWindowStyle.Hidden
            })) {
                if (cleanup != null) cleanup.WaitForExit(5000);
            }
        } catch {}
    }

    private void RemoveAutostartAndPath()
    {
        try {
            using (RegistryKey key = Registry.CurrentUser.OpenSubKey(@"Software\Microsoft\Windows\CurrentVersion\Run", true)) {
                if (key != null) {
                    key.DeleteValue("FleetHUD", false);
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
            string desktopLnk = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory), "spawnt.lnk");
            if (File.Exists(desktopLnk)) File.Delete(desktopLnk);

            string startLnk = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), @"Microsoft\Windows\Start Menu\Programs\spawnt.lnk");
            if (File.Exists(startLnk)) File.Delete(startLnk);
        } catch {}
    }

    private void RemoveUninstallRegistration()
    {
        try {
            Registry.CurrentUser.DeleteSubKeyTree(@"Software\Microsoft\Windows\CurrentVersion\Uninstall\FleetHUD", false);
        } catch {}
    }

    private void ScheduleFileCleanup()
    {
        try {
            // Delete all files in installDir except the currently executing uninstaller
            string currentExe = Process.GetCurrentProcess().MainModule.FileName;
            if (Directory.Exists(installDir)) {
                foreach (string f in Directory.GetFiles(installDir)) {
                    if (!String.Equals(f, currentExe, StringComparison.OrdinalIgnoreCase)) {
                        try { File.Delete(f); } catch {}
                    }
                }
                foreach (string d in Directory.GetDirectories(installDir)) {
                    try { Directory.Delete(d, true); } catch {}
                }
            }

            // Schedule self-deletion of the remaining uninstaller exe and parent directory
            Process.Start(new ProcessStartInfo {
                FileName = "cmd.exe",
                Arguments = "/c timeout /t 2 /nobreak > NUL & rmdir /s /q \"" + installDir + "\"",
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
            if (!isQuiet) {
                DialogResult dr = MessageBox.Show(
                    "Are you sure you want to completely remove spawnt from your computer?",
                    "spawnt Uninstall",
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
