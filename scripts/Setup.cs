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

[assembly: AssemblyTitle("FleetHUD Mission Control Setup")]
[assembly: AssemblyProduct("FleetHUD")]
[assembly: AssemblyCompany("FleetHUD")]
[assembly: AssemblyVersion("1.1.2.0")]
[assembly: AssemblyFileVersion("1.1.2.0")]

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
        this.BackColor = Color.FromArgb(13, 17, 23); // #0d1117
        this.ForeColor = Color.FromArgb(201, 209, 219); // #c9d1d9
        this.DoubleBuffered = true;

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
        lblTitle.Text = "FleetHUD Mission Control";
        lblTitle.Font = new Font("Segoe UI", 15, FontStyle.Bold);
        lblTitle.ForeColor = Color.White;
        lblTitle.Location = new Point(88, 38);
        lblTitle.AutoSize = true;
        this.Controls.Add(lblTitle);

        Label lblSub = new Label();
        lblSub.Text = "Setting up native engine and system tray...";
        lblSub.Font = new Font("Segoe UI", 9.5f, FontStyle.Regular);
        lblSub.ForeColor = Color.FromArgb(139, 148, 158); // #8b949e
        lblSub.Location = new Point(90, 72);
        lblSub.AutoSize = true;
        this.Controls.Add(lblSub);

        // Status
        lblStatus = new Label();
        lblStatus.Text = "Initializing setup...";
        lblStatus.Font = new Font("Segoe UI", 9.0f, FontStyle.Regular);
        lblStatus.ForeColor = Color.FromArgb(56, 189, 248); // #38bdf8
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
        lblVer.Text = "v1.1.2  ·  64-bit Native";
        lblVer.Font = new Font("Segoe UI", 8.0f, FontStyle.Regular);
        lblVer.ForeColor = Color.FromArgb(110, 118, 129);
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

        // Draw border
        using (Pen p = new Pen(Color.FromArgb(48, 54, 61), 1.5f)) {
            g.DrawRectangle(p, 0, 0, this.Width - 1, this.Height - 1);
        }

        // Draw Squircle Logo (at 36, 32, size 42x42)
        int x = 36, y = 34, size = 42;
        using (GraphicsPath path = RoundedRect(new Rectangle(x, y, size, size), 10)) {
            using (Brush b = new SolidBrush(Color.FromArgb(13, 17, 23))) {
                g.FillPath(b, path);
            }
        }

        // Draw the original geometric glyph scaled into 42x42.
        float s = (float)size / 2048.0f;
        PointF[] poly = new PointF[] {
            new PointF(x + 290.67f * s, y + 1756.53f * s),
            new PointF(x + 290.67f * s, y + 291.47f * s),
            new PointF(x + 1755.73f * s, y + 291.47f * s)
        };
        using (Brush bGlyph = new SolidBrush(Color.White)) {
            g.FillPolygon(bGlyph, poly);
            g.FillRectangle(bGlyph, x + 799.39f * s, y + 789.88f * s, 957.95f * s, 957.95f * s);
        }
    }

    private GraphicsPath RoundedRect(Rectangle bounds, int radius)
    {
        GraphicsPath path = new GraphicsPath();
        int d = radius * 2;
        path.AddArc(bounds.X, bounds.Y, d, d, 180, 90);
        path.AddArc(bounds.Right - d, bounds.Y, d, d, 270, 90);
        path.AddArc(bounds.Right - d, bounds.Bottom - d, d, d, 0, 90);
        path.AddArc(bounds.X, bounds.Bottom - d, d, d, 90, 90);
        path.CloseFigure();
        return path;
    }

    private void OnTick(object sender, EventArgs e)
    {
        step++;
        if (step == 2) {
            lblStatus.Text = "Creating installation directory...";
            progressBar.Value = 25;
            Directory.CreateDirectory(installDir);
        } else if (step == 4) {
            lblStatus.Text = "Extracting FleetHUD native engine...";
            progressBar.Value = 50;
            ExtractEmbeddedFiles();
        } else if (step == 7) {
            lblStatus.Text = "Creating desktop and Start menu shortcuts...";
            progressBar.Value = 75;
            CreateShortcuts();
        } else if (step == 9) {
            lblStatus.Text = "Registering autostart and system tray...";
            progressBar.Value = 90;
            ConfigureAutostart();
        } else if (step == 11) {
            lblStatus.Text = "Launching FleetHUD Mission Control...";
            progressBar.Value = 100;
        } else if (step == 13) {
            timer.Stop();
            LaunchFleetHUD();
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
                Arguments = "-NoProfile -Command \"Get-CimInstance Win32_Process | Where-Object { $_.Name -eq 'powershell.exe' -and $_.ProcessId -ne $PID -and $_.CommandLine -like '*FleetHUD*tray.ps1*' } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }\"",
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
        } catch (Exception ex) {
            timer.Stop();
            lblStatus.Text = "Installation failed";
            MessageBox.Show(ex.Message, "FleetHUD Setup", MessageBoxButtons.OK, MessageBoxIcon.Error);
        }
    }

    private void CreateShortcuts()
    {
        try {
            Type shellType = Type.GetTypeFromProgID("WScript.Shell");
            dynamic shell = Activator.CreateInstance(shellType);
            string vbs = Path.Combine(installDir, "open-console.vbs");
            string ico = Path.Combine(installDir, "fleethud.ico");

            // Desktop
            string desktopDir = Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory);
            string desktopLnk = Path.Combine(desktopDir, "FleetHUD.lnk");
            dynamic scDesktop = shell.CreateShortcut(desktopLnk);
            scDesktop.TargetPath = "wscript.exe";
            scDesktop.Arguments = "\"" + vbs + "\"";
            scDesktop.IconLocation = ico + ",0";
            scDesktop.Description = "FleetHUD Mission Control";
            scDesktop.WorkingDirectory = installDir;
            scDesktop.Save();

            // Start Menu
            string startMenuDir = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), @"Microsoft\Windows\Start Menu\Programs");
            string startLnk = Path.Combine(startMenuDir, "FleetHUD.lnk");
            dynamic scStart = shell.CreateShortcut(startLnk);
            scStart.TargetPath = "wscript.exe";
            scStart.Arguments = "\"" + vbs + "\"";
            scStart.IconLocation = ico + ",0";
            scStart.Description = "FleetHUD Mission Control";
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

    [STAThread]
    static void Main()
    {
        Application.EnableVisualStyles();
        Application.SetCompatibleTextRenderingDefault(false);
        Application.Run(new SetupForm());
    }
}
