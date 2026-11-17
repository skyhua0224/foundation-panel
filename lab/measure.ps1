param([string]$Exe, [int]$Port, [string]$Profile, [string]$Mode = "direct", [int]$ObserveMs = 7000)
Add-Type -ReferencedAssemblies System.Drawing.Common, System.Runtime.InteropServices, System.Collections, System.Diagnostics.Process, System.ComponentModel.Primitives, System.Drawing.Primitives -TypeDefinition @"
using System; using System.Text; using System.Collections.Generic; using System.Diagnostics; using System.Drawing; using System.Drawing.Imaging; using System.Runtime.InteropServices; using System.Threading;
public static class Probe {
  delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc p, IntPtr l);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr h, out RECT r);
  [StructLayout(LayoutKind.Sequential)] struct RECT { public int L, T, R, B; }
  static IntPtr Find(uint pid, string title, out RECT rect) {
    IntPtr found = IntPtr.Zero; RECT rr = new RECT();
    EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p); if (p == pid && IsWindowVisible(h)) { var t = new StringBuilder(256); GetWindowText(h, t, 256); if (t.ToString() == title) { found = h; GetWindowRect(h, out rr); return false; } } return true; }, IntPtr.Zero);
    rect = rr; return found;
  }
  static int[] Sample(Bitmap b) {
    var d = b.LockBits(new Rectangle(0, 0, b.Width, b.Height), ImageLockMode.ReadOnly, PixelFormat.Format32bppArgb);
    int stride = 6; int w = b.Width / stride, h = b.Height / stride; var o = new int[w * h];
    unsafe0(d, o, w, h, stride); b.UnlockBits(d); return o;
  }
  static void unsafe0(BitmapData d, int[] o, int w, int h, int stride) {
    byte[] row = new byte[d.Stride];
    for (int y = 0; y < h; y++) { Marshal.Copy(IntPtr.Add(d.Scan0, y * stride * d.Stride), row, 0, d.Stride); for (int x = 0; x < w; x++) o[y * w + x] = BitConverter.ToInt32(row, x * stride * 4); }
  }
  static double Diff(int[] a, int[] b) { int n = 0; for (int i = 0; i < a.Length; i++) { if (a[i] != b[i]) n++; } return (double)n / a.Length; }
  // returns "tVisible,tFirstChange,tSettled,frames,pid" in ms from process start
  public static string Run(string exe, string title, int observeMs, string profile, int port) {
    var psi = new ProcessStartInfo(exe); psi.UseShellExecute = false; psi.WorkingDirectory = System.IO.Path.GetDirectoryName(exe);
    psi.EnvironmentVariables["SUNSHINE_GUI_TEST_OFFSCREEN"] = "1"; psi.EnvironmentVariables["SUNSHINE_GUI_ALLOW_PARALLEL_INSTANCE"] = "1";
    psi.EnvironmentVariables["WEBVIEW2_USER_DATA_FOLDER"] = profile;
    var sw = Stopwatch.StartNew(); var p = Process.Start(psi);
    double tVisible = -1, tFirst = -1, tSettled = -1; int frames = 0; int[] first = null, prev = null; var series = new List<string>();
    while (sw.ElapsedMilliseconds < observeMs) {
      if (p.HasExited) break;
      RECT r; var h = Find((uint)p.Id, title, out r);
      if (h != IntPtr.Zero) {
        if (tVisible < 0) tVisible = sw.Elapsed.TotalMilliseconds;
        int w = r.R - r.L, ht = r.B - r.T;
        if (w > 100 && ht > 100) {
          using (var bmp = new Bitmap(w, ht, PixelFormat.Format32bppArgb)) {
            using (var g = Graphics.FromImage(bmp)) { g.CopyFromScreen(r.L, r.T, 0, 0, new Size(w, ht)); }
            var s = Sample(bmp); double t = sw.Elapsed.TotalMilliseconds; frames++;
            if (first == null) { first = s; }
            else {
              if (tFirst < 0 && Diff(first, s) > 0.01) tFirst = t;
              if (prev != null && Diff(prev, s) > 0.0005) tSettled = t;
            }
            if (frames % 5 == 0) series.Add(((int)t) + ":" + Diff(first, s).ToString("F3"));
            prev = s;
          }
        }
      } else Thread.Sleep(2);
    }
    int pid = p.Id; try { p.Kill(); } catch {}
    return tVisible.ToString("F1") + "," + tFirst.ToString("F1") + "," + tSettled.ToString("F1") + "," + frames + "," + pid + "|" + string.Join(" ", series);
  }
}
"@
$t0 = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
$res = [Probe]::Run($Exe, "Sunshine Control Panel", $ObserveMs, $Profile, $Port)
@{ t0 = $t0; result = $res } | ConvertTo-Json -Compress
