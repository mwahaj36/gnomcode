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
[assembly: AssemblyVersion("1.2.1.0")]
[assembly: AssemblyFileVersion("1.2.1.0")]

public class SetupForm : Form
{
    private Label lblTitle;
    private Label lblStatus;
    private ProgressBar progressBar;
    private System.Windows.Forms.Timer timer;
    private int step = 0;
    private string installDir;
    private string exeDest;

    [DllImport("user32.dll")]
    public static extern int SendMessage(IntPtr hWnd, int Msg, int wParam, int lParam);
    [DllImport("user32.dll")]
    public static extern bool ReleaseCapture();

    public SetupForm()
    {
        this.FormBorderStyle = FormBorderStyle.None;
        this.StartPosition = FormStartPosition.CenterScreen;
        this.Size = new Size(480, 270);
        this.BackColor = Color.FromArgb(12, 17, 25); // #0C1119
        this.ForeColor = Color.FromArgb(246, 247, 248); // #F6F7F8
        this.DoubleBuffered = true;

        installDir = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "spawnt");
        exeDest = Path.Combine(installDir, "spawnt.exe");

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
        lblSub.Text = "Setting up local agent telemetry and desktop tray...";
        lblSub.Font = new Font("Segoe UI", 9.5f, FontStyle.Regular);
        lblSub.ForeColor = Color.FromArgb(169, 180, 194); // #A9B4C2
        lblSub.Location = new Point(90, 72);
        lblSub.AutoSize = true;
        this.Controls.Add(lblSub);

        // Status
        lblStatus = new Label();
        lblStatus.Text = "Initializing setup...";
        lblStatus.Font = new Font("Segoe UI", 9.0f, FontStyle.Regular);
        lblStatus.ForeColor = Color.FromArgb(255, 37, 68); // #FF2544 Primary Red
        lblStatus.Location = new Point(36, 138);
        lblStatus.Size = new Size(408, 22);
        this.Controls.Add(lblStatus);

        // Progress Bar
        progressBar = new ProgressBar();
        progressBar.Location = new Point(36, 166);
        progressBar.Size = new Size(408, 14);
        progressBar.Style = ProgressBarStyle.Continuous;
        progressBar.Value = 10;
        this.Controls.Add(progressBar);

        // Version badge
        Label lblVer = new Label();
        lblVer.Text = "v1.2.1  ·  64-bit Native  ·  Local-First";
        lblVer.Font = new Font("Segoe UI", 8.0f, FontStyle.Regular);
        lblVer.ForeColor = Color.FromArgb(133, 145, 163);
        lblVer.Location = new Point(36, 225);
        lblVer.AutoSize = true;
        this.Controls.Add(lblVer);

        timer = new System.Windows.Forms.Timer();
        timer.Interval = 120;
        timer.Tick += OnTick;
        timer.Start();
    }

    protected override void OnPaint(PaintEventArgs e)
    {
        base.OnPaint(e);
        Graphics g = e.Graphics;
        g.SmoothingMode = SmoothingMode.AntiAlias;

        // Draw 1px hairline border
        using (Pen p = new Pen(Color.FromArgb(35, 44, 59), 1.0f)) {
            g.DrawRectangle(p, 0, 0, this.Width - 1, this.Height - 1);
        }

        // Draw spawnt logo at (36, 34, size 42x42)
        int x = 36, y = 34, size = 42;
        float s = (float)size / 2048.0f;
        PointF[] poly = new PointF[] {
            new PointF(x + 290.67f * s, y + 1756.53f * s),
            new PointF(x + 1757.33f * s, y + 1756.53f * s),
            new PointF(x + 1757.33f * s, y + 291.47f * s)
        };
        using (Brush bGlyph = new SolidBrush(Color.FromArgb(255, 37, 68))) {
            g.FillPolygon(bGlyph, poly);
            g.FillRectangle(bGlyph, x + 799.39f * s, y + 789.88f * s, 449.23f * s, 449.23f * s);
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
            lblStatus.Text = "Extracting spawnt native engine...";
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
            lblStatus.Text = "Launching spawnt console...";
            progressBar.Value = 100;
        } else if (step == 14) {
            timer.Stop();
            LaunchSpawnt();
            this.Close();
        }
    }

    private void ExtractEmbeddedFiles()
    {
        try {
            foreach (string pName in new string[] { "spawnt", "fleethud" }) {
                foreach (Process process in Process.GetProcessesByName(pName)) {
                    try {
                        process.Kill();
                        process.WaitForExit(3000);
                    } catch {}
                    process.Dispose();
                }
            }
            using (Process cleanup = Process.Start(new ProcessStartInfo {
                FileName = "powershell.exe",
                Arguments = "-NoProfile -Command \"Get-CimInstance Win32_Process | Where-Object { (($_.Name -like 'powershell*' -or $_.Name -like 'pwsh*') -and $_.ProcessId -ne $PID -and ($_.CommandLine -like '*tray.ps1*' -or $_.CommandLine -like '*FleetHUD*tray.ps1*')) -or ($_.CommandLine -like '*spawnt\\AppProfile*' -or $_.CommandLine -like '*FleetHUD\\AppProfile*') -or ($_.Name -like 'msedge*' -and $_.CommandLine -like '*127.0.0.1:41738*') } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }\"",
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

                // If fleethud.exe was extracted, also copy as spawnt.exe
                if (fileName.Equals("fleethud.exe", StringComparison.OrdinalIgnoreCase)) {
                    try {
                        File.Copy(dest, Path.Combine(installDir, "spawnt.exe"), true);
                    } catch {}
                }
            }

            // Copy setup binary itself as spawnt-uninstall.exe
            string currentExe = Process.GetCurrentProcess().MainModule.FileName;
            string uninstallerDest = Path.Combine(installDir, "spawnt-uninstall.exe");
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
            using (RegistryKey key = Registry.CurrentUser.CreateSubKey(@"Software\Microsoft\Windows\CurrentVersion\Uninstall\spawnt")) {
                if (key != null) {
                    string uninstallExe = Path.Combine(installDir, "spawnt-uninstall.exe");
                    string ico = Path.Combine(installDir, "spawnt.ico");
                    if (!File.Exists(ico)) ico = Path.Combine(installDir, "fleethud.ico");

                    key.SetValue("DisplayName", "spawnt");
                    key.SetValue("DisplayVersion", "1.2.1");
                    key.SetValue("Publisher", "spawnt");
                    key.SetValue("DisplayIcon", ico + ",0");
                    key.SetValue("InstallLocation", installDir);
                    key.SetValue("UninstallString", "\"" + uninstallExe + "\" --uninstall");
                    key.SetValue("QuietUninstallString", "\"" + uninstallExe + "\" --uninstall --quiet");
                    key.SetValue("EstimatedSize", 16500, RegistryValueKind.DWord);
                    key.SetValue("NoModify", 1, RegistryValueKind.DWord);
                    key.SetValue("NoRepair", 1, RegistryValueKind.DWord);
                    key.SetValue("URLInfoAbout", "https://spawnt.xyz");
                    key.SetValue("HelpLink", "https://spawnt.xyz/docs");
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
            string ico = Path.Combine(installDir, "spawnt.ico");
            if (!File.Exists(ico)) ico = Path.Combine(installDir, "fleethud.ico");

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
                    key.SetValue("spawnt", "wscript.exe \"" + vbs + "\"");
                }
            }
        } catch {}
    }

    private void LaunchSpawnt()
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
    private ProgressBar progressBar;
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
        installDir = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "spawnt");

        this.FormBorderStyle = FormBorderStyle.None;
        this.StartPosition = FormStartPosition.CenterScreen;
        this.Size = new Size(480, 240);
        this.BackColor = Color.FromArgb(12, 17, 25); // #0C1119
        this.ForeColor = Color.FromArgb(246, 247, 248); // #F6F7F8
        this.DoubleBuffered = true;

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
        lblStatus.ForeColor = Color.FromArgb(255, 37, 68);
        lblStatus.Location = new Point(36, 125);
        lblStatus.Size = new Size(408, 22);
        this.Controls.Add(lblStatus);

        progressBar = new ProgressBar();
        progressBar.Location = new Point(36, 152);
        progressBar.Size = new Size(408, 14);
        progressBar.Style = ProgressBarStyle.Continuous;
        progressBar.Value = 10;
        this.Controls.Add(progressBar);

        timer = new System.Windows.Forms.Timer();
        timer.Interval = 140;
        timer.Tick += OnTick;
        timer.Start();
    }

    protected override void OnPaint(PaintEventArgs e)
    {
        base.OnPaint(e);
        Graphics g = e.Graphics;
        g.SmoothingMode = SmoothingMode.AntiAlias;

        using (Pen p = new Pen(Color.FromArgb(35, 44, 59), 1.0f)) {
            g.DrawRectangle(p, 0, 0, this.Width - 1, this.Height - 1);
        }

        int x = 36, y = 34, size = 42;
        float s = (float)size / 2048.0f;
        PointF[] poly = new PointF[] {
            new PointF(x + 290.67f * s, y + 1756.53f * s),
            new PointF(x + 1757.33f * s, y + 1756.53f * s),
            new PointF(x + 1757.33f * s, y + 291.47f * s)
        };
        using (Brush bGlyph = new SolidBrush(Color.FromArgb(255, 37, 68))) {
            g.FillPolygon(bGlyph, poly);
            g.FillRectangle(bGlyph, x + 799.39f * s, y + 789.88f * s, 449.23f * s, 449.23f * s);
        }
    }

    private void OnTick(object sender, EventArgs e)
    {
        step++;
        if (step == 2) {
            lblStatus.Text = "Stopping running spawnt processes...";
            progressBar.Value = 25;
            KillProcesses();
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

    private void KillProcesses()
    {
        try {
            foreach (string pName in new string[] { "spawnt", "fleethud" }) {
                foreach (Process p in Process.GetProcessesByName(pName)) {
                    try { p.Kill(); p.WaitForExit(2000); } catch {}
                    p.Dispose();
                }
            }
            using (Process cleanup = Process.Start(new ProcessStartInfo {
                FileName = "powershell.exe",
                Arguments = "-NoProfile -Command \"Get-CimInstance Win32_Process | Where-Object { (($_.Name -like 'powershell*' -or $_.Name -like 'pwsh*') -and $_.ProcessId -ne $PID -and ($_.CommandLine -like '*tray.ps1*' -or $_.CommandLine -like '*FleetHUD*tray.ps1*')) -or ($_.CommandLine -like '*spawnt\\AppProfile*' -or $_.CommandLine -like '*FleetHUD\\AppProfile*') -or ($_.Name -like 'msedge*' -and $_.CommandLine -like '*127.0.0.1:41738*') } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }\"",
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
                    key.DeleteValue("spawnt", false);
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
            string desktopDir = Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory);
            foreach (string name in new string[] { "spawnt.lnk", "FleetHUD.lnk", "FleetHUD Console.lnk" }) {
                string p = Path.Combine(desktopDir, name);
                if (File.Exists(p)) File.Delete(p);
            }

            string startMenuDir = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), @"Microsoft\Windows\Start Menu\Programs");
            foreach (string name in new string[] { "spawnt.lnk", "FleetHUD.lnk", "FleetHUD Console.lnk" }) {
                string p = Path.Combine(startMenuDir, name);
                if (File.Exists(p)) File.Delete(p);
            }
        } catch {}
    }

    private void RemoveUninstallRegistration()
    {
        try {
            Registry.CurrentUser.DeleteSubKeyTree(@"Software\Microsoft\Windows\CurrentVersion\Uninstall\spawnt", false);
        } catch {}
        try {
            Registry.CurrentUser.DeleteSubKeyTree(@"Software\Microsoft\Windows\CurrentVersion\Uninstall\FleetHUD", false);
        } catch {}
    }

    private void ScheduleFileCleanup()
    {
        try {
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

        foreach (string arg in args) {
            string a = arg.ToLowerInvariant();
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
            Application.Run(new SetupForm());
        }
    }
}
