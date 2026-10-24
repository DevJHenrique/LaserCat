// SPDX-License-Identifier: AGPL-3.0-only
using System;
using System.Drawing;
using System.IO;
using System.Reflection;
using System.Windows.Forms;

[assembly: AssemblyTitle("LaserCat Installer")]
[assembly: AssemblyVersion("0.2.1.0")]
internal sealed class Installer : Form {
    private readonly TextBox destination = new TextBox();
    private readonly CheckBox shortcuts = new CheckBox();
    private readonly Button install = new Button();
    [STAThread] private static void Main() {
        Application.EnableVisualStyles();
        Application.SetCompatibleTextRenderingDefault(false);
        Application.Run(new Installer());
    }
    private Installer() {
        Text = "Install LaserCat 0.2.1";
        ClientSize = new Size(590, 285);
        FormBorderStyle = FormBorderStyle.FixedDialog;
        MaximizeBox = false; StartPosition = FormStartPosition.CenterScreen;
        Font = new Font("Segoe UI", 10); AutoScaleMode = AutoScaleMode.Dpi;
        BackColor = Color.FromArgb(22,29,40); ForeColor = Color.White;
        Label title = new Label { Text="LASERCAT", Location=new Point(25,22), Size=new Size(500,38), Font=new Font("Segoe UI",21,FontStyle.Bold), ForeColor=Color.FromArgb(100,160,240) };
        Label intro = new Label { Text="Laser cutting and engraving on a visual bed.\nInstalls for the current user only; no administrator rights required.", Location=new Point(27,75), Size=new Size(530,48) };
        destination.Text = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Programs", "LaserCat");
        destination.SetBounds(28,139,440,28);
        Button browse = new Button { Text="Browse…", Location=new Point(477,136), Size=new Size(85,30) };
        browse.Click += delegate { using(FolderBrowserDialog dialog=new FolderBrowserDialog()) { if(dialog.ShowDialog(this)==DialogResult.OK) destination.Text=Path.Combine(dialog.SelectedPath,"LaserCat"); } };
        shortcuts.Text="Create Start menu shortcuts (includes the demo)";shortcuts.Checked=true;shortcuts.SetBounds(28,180,530,27);
        install.Text="Install";install.SetBounds(433,229,130,34);install.BackColor=Color.FromArgb(36,115,214);install.FlatStyle=FlatStyle.Flat;
        install.Click += Install;
        Controls.AddRange(new Control[]{title,intro,destination,browse,shortcuts,install});
    }
    private static void Extract(string resource,string path) {
        string temp=path+"."+Guid.NewGuid().ToString("N")+".tmp";
        try {
            using(Stream source=Assembly.GetExecutingAssembly().GetManifestResourceStream(resource))
            using(FileStream output=new FileStream(temp,FileMode.CreateNew,FileAccess.Write)) { source.CopyTo(output); output.Flush(true); }
            if(File.Exists(path)) File.Replace(temp,path,null); else File.Move(temp,path);
        } finally { if(File.Exists(temp)) File.Delete(temp); }
    }
    private void Install(object sender,EventArgs args) {
        install.Enabled=false;
        try {
            string dir=Path.GetFullPath(destination.Text);
            Directory.CreateDirectory(dir);
            string exe=Path.Combine(dir,"lasercat.exe");
            Extract("app",exe);Extract("icon",Path.Combine(dir,"lasercat.ico"));Extract("readme",Path.Combine(dir,"README.md"));Extract("license",Path.Combine(dir,"LICENSE"));Extract("notices",Path.Combine(dir,"THIRD_PARTY_NOTICES.txt"));
            Version v=Assembly.GetExecutingAssembly().GetName().Version;
            Extract("source",Path.Combine(dir,String.Format("LaserCat-{0}.{1}.{2}-source.zip",v.Major,v.Minor,v.Build)));
            string calibration=Path.Combine(dir,"calibration");Directory.CreateDirectory(calibration);
            Extract("engraving",Path.Combine(calibration,"engraving-calibration.lcc"));Extract("cutting",Path.Combine(calibration,"cutting-calibration.lcc"));
            foreach(string resource in Assembly.GetExecutingAssembly().GetManifestResourceNames()) {
                if(!resource.StartsWith("document:",StringComparison.Ordinal)) continue;
                string relative=resource.Substring("document:".Length).Replace('/',Path.DirectorySeparatorChar);
                string output=Path.Combine(dir,relative);Directory.CreateDirectory(Path.GetDirectoryName(output));Extract(resource,output);
            }
            if(shortcuts.Checked) {
                Type shellType=Type.GetTypeFromProgID("WScript.Shell");
                object shell=Activator.CreateInstance(shellType);
                string programs=Environment.GetFolderPath(Environment.SpecialFolder.Programs);
                Shortcut(shell,shellType,Path.Combine(programs,"LaserCat.lnk"),exe,dir,"");
                Shortcut(shell,shellType,Path.Combine(programs,"LaserCat - Demo.lnk"),exe,dir,"--demo");
                System.Runtime.InteropServices.Marshal.FinalReleaseComObject(shell);
            }
            MessageBox.Show(this,"Installed to:\n"+dir+"\n\nOpen LaserCat from the Start menu. Configure the real machine dimensions before connecting a machine.\n\nTo uninstall, delete this folder and the shortcuts; your projects and recovery copy are kept.","Installation complete",MessageBoxButtons.OK,MessageBoxIcon.Information);
            Close();
        } catch(Exception e) { MessageBox.Show(this,e.Message+"\n\nIf the application is open, close it before installing again.","Could not install",MessageBoxButtons.OK,MessageBoxIcon.Error);install.Enabled=true; }
    }
    private static void Shortcut(object shell,Type type,string path,string exe,string dir,string arguments) {
        object link=type.InvokeMember("CreateShortcut",BindingFlags.InvokeMethod,null,shell,new object[]{path});
        Type t=link.GetType();
        t.InvokeMember("TargetPath",BindingFlags.SetProperty,null,link,new object[]{exe});
        t.InvokeMember("WorkingDirectory",BindingFlags.SetProperty,null,link,new object[]{dir});
        t.InvokeMember("Arguments",BindingFlags.SetProperty,null,link,new object[]{arguments});
        t.InvokeMember("IconLocation",BindingFlags.SetProperty,null,link,new object[]{Path.Combine(dir,"lasercat.ico")});
        t.InvokeMember("Save",BindingFlags.InvokeMethod,null,link,new object[]{});
        System.Runtime.InteropServices.Marshal.FinalReleaseComObject(link);
    }
}
