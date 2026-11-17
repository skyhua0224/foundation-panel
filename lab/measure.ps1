param([string]$Exe, [int]$Port, [string]$Profile, [string]$Mode = "direct", [int]$ObserveMs = 7000)
Add-Type -TypeDefinition @"
using System; using System.Text; using System.Collections.Generic; using System.Diagnostics; using System.Runtime.InteropServices; using System.Threading;
public static class Probe {
  delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc p, IntPtr l);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] static extern IntPtr GetDC(IntPtr h);
  [DllImport("user32.dll")] static extern int ReleaseDC(IntPtr h, IntPtr dc);
  [DllImport("gdi32.dll")] static extern IntPtr CreateCompatibleDC(IntPtr dc);
  [DllImport("gdi32.dll")] static extern IntPtr CreateDIBSection(IntPtr dc, ref BITMAPINFO bi, uint usage, out IntPtr bits, IntPtr sec, uint off);
  [DllImport("gdi32.dll")] static extern IntPtr SelectObject(IntPtr dc, IntPtr o);
  [DllImport("gdi32.dll")] static extern bool BitBlt(IntPtr d, int x, int y, int w, int h, IntPtr s, int sx, int sy, uint rop);
  [DllImport("gdi32.dll")] static extern bool DeleteObject(IntPtr o);
  [DllImport("gdi32.dll")] static extern bool DeleteDC(IntPtr dc);
  [StructLayout(LayoutKind.Sequential)] struct RECT { public int L, T, R, B; }
  [StructLayout(LayoutKind.Sequential)] struct BITMAPINFO { public int Size, Width, Height; public short Planes, BitCount; public int Compression, SizeImage, XPel, YPel, ClrUsed, ClrImportant; }
  static IntPtr Find(uint pid, string title, out RECT rect) {
    IntPtr found = IntPtr.Zero; RECT rr = new RECT();
    EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p); if (p == pid && IsWindowVisible(h)) { var t = new StringBuilder(256); GetWindowText(h, t, 256); if (t.ToString() == title) { found = h; GetWindowRect(h, out rr); return false; } } return true; }, IntPtr.Zero);
    rect = rr; return found;
  }
  static int[] Grab(RECT r, int stride) {
    int w = r.R - r.L, h = r.B - r.T;
    IntPtr screen = GetDC(IntPtr.Zero); IntPtr mem = CreateCompatibleDC(screen);
    var bi = new BITMAPINFO(); bi.Size = 40; bi.Width = w; bi.Height = -h; bi.Planes = 1; bi.BitCount = 32;
    IntPtr bits; IntPtr dib = CreateDIBSection(screen, ref bi, 0, out bits, IntPtr.Zero, 0);
    IntPtr old = SelectObject(mem, dib);
    BitBlt(mem, 0, 0, w, h, screen, r.L, r.T, 0x00CC0020);
    int sw = w / stride, sh = h / stride; var o = new int[sw * sh];
    for (int y = 0; y < sh; y++) for (int x = 0; x < sw; x++) o[y * sw + x] = Marshal.ReadInt32(bits, ((y * stride) * w + x * stride) * 4);
    SelectObject(mem, old); DeleteObject(dib); DeleteDC(mem); ReleaseDC(IntPtr.Zero, screen);
    return o;
  }
  static double Diff(int[] a, int[] b) { if (a.Length != b.Length) return 1; int n = 0; for (int i = 0; i < a.Length; i++) { if (a[i] != b[i]) n++; } return (double)n / a.Length; }
  public static string Run(string exe, string title, int observeMs, string profile) {
    var psi = new ProcessStartInfo(exe); psi.UseShellExecute = false; psi.WorkingDirectory = System.IO.Path.GetDirectoryName(exe);
    psi.Environment["SUNSHINE_GUI_TEST_OFFSCREEN"] = "1"; psi.Environment["SUNSHINE_GUI_ALLOW_PARALLEL_INSTANCE"] = "1"; psi.Environment["WEBVIEW2_USER_DATA_FOLDER"] = profile;
    var sw = Stopwatch.StartNew(); var p = Process.Start(psi);
    double tVisible = -1, tFirst = -1, tSettled = -1, tContent = -1; int frames = 0; int[] first = null, prev = null; var series = new List<string>();
    while (sw.ElapsedMilliseconds < observeMs) {
      if (p.HasExited) break;
      RECT r; var h = Find((uint)p.Id, title, out r);
      if (h == IntPtr.Zero) { Thread.Sleep(2); continue; }
      if (tVisible < 0) tVisible = sw.Elapsed.TotalMilliseconds;
      if (r.R - r.L < 100 || r.B - r.T < 100) continue;
      var s = Grab(r, 6); double t = sw.Elapsed.TotalMilliseconds; frames++;
      if (first == null) first = s;
      else {
        double df = Diff(first, s); if (tFirst < 0 && df > 0.01) tFirst = t; if (tContent < 0 && df > 0.5) tContent = t;
        if (prev != null && Diff(prev, s) > 0.0005) tSettled = t;
      }
      if (frames % 5 == 0) series.Add(((int)t) + ":" + Diff(first, s).ToString("F3"));
      prev = s;
    }
    int pid = p.Id; try { p.Kill(); } catch {}
    return tVisible.ToString("F1") + "," + tFirst.ToString("F1") + "," + tSettled.ToString("F1") + "," + tContent.ToString("F1") + "," + frames + "," + pid + "|" + string.Join(" ", series);
  }
}
"@
$t0 = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
$res = [Probe]::Run($Exe, "Sunshine Control Panel", $ObserveMs, $Profile)
@{ t0 = $t0; result = $res } | ConvertTo-Json -Compress
