using System;
using System.IO;
using System.Reflection;
using System.Security.Cryptography;

public static class CheckInstallerPayload {
    private static string Hash(Stream stream) {
        using (SHA256 hash = SHA256.Create()) { return BitConverter.ToString(hash.ComputeHash(stream)); }
    }
    private static void Verify(Assembly installer, string resource, string file) {
        using (Stream embedded = installer.GetManifestResourceStream(resource))
        using (Stream expected = File.OpenRead(file)) {
            if (embedded == null || Hash(embedded) != Hash(expected)) throw new Exception("Embedded payload mismatch: " + resource);
        }
    }
    public static int Main(string[] args) {
        if (args.Length != 4) throw new ArgumentException("Usage: checker setup.exe engine.exe notices.html version");
        Assembly installer = Assembly.LoadFile(Path.GetFullPath(args[0]));
        if (installer.GetName().Version.ToString() != args[3]) throw new Exception("Installer assembly version mismatch");
        if (System.Diagnostics.FileVersionInfo.GetVersionInfo(args[0]).FileVersion != args[3]) throw new Exception("Installer file version mismatch");
        Verify(installer, "gnomcode.exe", args[1]);
        Verify(installer, "THIRD_PARTY_NOTICES.html", args[2]);
        foreach (string name in new [] { "LICENSE", "tray.ps1", "run-tray.vbs", "open-console.vbs", "open-dashboard.vbs", "open-folder.ps1", "gnomcode.ico", "gnomcode-tray.ico", "gnomcode-mark.png" }) {
            using (Stream resource = installer.GetManifestResourceStream(name)) {
                if (resource == null || resource.Length == 0) throw new Exception("Missing installer resource: " + name);
            }
        }
        Console.WriteLine("PASS: installer version, embedded executable, notices and launcher resources");
        return 0;
    }
}
