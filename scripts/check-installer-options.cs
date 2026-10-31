using System;
using System.IO;
using System.Reflection;
using System.Windows.Forms;
using Microsoft.Win32;

public static class CheckInstallerOptions {
    private static void Assert(bool value, string message) { if (!value) throw new Exception(message); }
    [STAThread]
    public static int Main(string[] args) {
        Application.EnableVisualStyles();
        Application.SetCompatibleTextRenderingDefault(false);
        Assembly setup = Assembly.LoadFile(Path.GetFullPath(args[0]));
        using (Form form = (Form)Activator.CreateInstance(setup.GetType("SetupForm"), new object[] { false })) {
            var timer = (Timer)form.GetType().GetField("timer", BindingFlags.Instance | BindingFlags.NonPublic).GetValue(form);
            Assert(!timer.Enabled, "Setup must wait for Install before modifying the computer");
            foreach (string name in new[] { "installPath", "desktopShortcut", "startMenuShortcut", "startAtLogin", "explorerMenu", "installButton" })
                Assert(form.Controls.Find(name, true).Length == 1, "Missing setup option: " + name);
            if (args.Length > 1) {
                form.Show();
                Application.DoEvents();
                using (var bitmap = new System.Drawing.Bitmap(form.Width, form.Height)) {
                    form.DrawToBitmap(bitmap, new System.Drawing.Rectangle(0, 0, form.Width, form.Height));
                    bitmap.Save(args[1]);
                }
                form.Hide();
            }
        }
        Type integration = setup.GetType("InstallerIntegration");
        Assert(integration != null, "Missing Explorer integration");
        string files = Path.Combine(Path.GetTempPath(), "spawnt-installer-files-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(files);
        try {
            File.WriteAllText(Path.Combine(files, "spawnt.exe"), "shipped payload");
            File.WriteAllText(Path.Combine(files, "user-notes.txt"), "keep");
            Directory.CreateDirectory(Path.Combine(files, "workspace"));
            File.WriteAllText(Path.Combine(files, "workspace", "code.txt"), "keep");
            MethodInfo remove = integration.GetMethod("RemovePayloadFiles");
            Assert(remove != null, "Uninstall needs non-recursive payload cleanup");
            remove.Invoke(null, new object[] { setup, files });
            Assert(!File.Exists(Path.Combine(files, "spawnt.exe")), "Uninstall must remove shipped files");
            Assert(File.ReadAllText(Path.Combine(files, "user-notes.txt")) == "keep", "Uninstall must preserve unrelated files");
            Assert(File.Exists(Path.Combine(files, "workspace", "code.txt")), "Uninstall must preserve subdirectories");
        } finally { Directory.Delete(files, true); }
        string testKey = @"Software\SpawntInstallerTests\" + Guid.NewGuid().ToString("N");
        try {
            using (RegistryKey root = Registry.CurrentUser.CreateSubKey(testKey)) {
                string dir = @"C:\Test folder & space\Spawnt";
                var configure = integration.GetMethod("ConfigureExplorer");
                configure.Invoke(null, new object[] { root, dir, true });
                foreach (string verb in new[] { @"Software\Classes\Directory\shell\Spawnt", @"Software\Classes\Directory\Background\shell\Spawnt" }) {
                    using (RegistryKey key = root.OpenSubKey(verb + @"\command")) {
                        Assert(key != null, "Missing folder/background command");
                        string command = (string)key.GetValue("");
                        Assert(command.Contains("-File \"C:\\Test folder & space\\Spawnt\\open-folder.ps1\""), "Launcher path must be quoted");
                        Assert(command.Contains(verb.Contains("Background") ? "\"%V\\.\"" : "\"%1\\.\""), "Explorer must pass the selected folder, including drive roots");
                    }
                }
                configure.Invoke(null, new object[] { root, dir, false });
                Assert(root.OpenSubKey(@"Software\Classes\Directory\shell\Spawnt") == null, "Opt-out/uninstall must remove Explorer entry");
            }
        } finally { Registry.CurrentUser.DeleteSubKeyTree(testKey, false); }
        Console.WriteLine("PASS: setup waits for consent, exposes options, quotes/removes Explorer integration and preserves unrelated files on uninstall");
        return 0;
    }
}
