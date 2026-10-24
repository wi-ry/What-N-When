using System;
using System.Runtime.InteropServices;
using System.Threading;
using Microsoft.Windows.Widgets.Providers;
using WinRT;

namespace WhatNWhenWidgetProvider;

internal static class Program
{
    private static readonly Guid ProviderClassId = new("84BF6B95-86A4-45B9-A9E8-775D043D1F9C");

    [MTAThread]
    private static void Main(string[] args)
    {
        if (args.Length == 0 || args[0] != "-RegisterProcessAsComServer")
        {
            Console.Error.WriteLine("The widget provider must be started by the Windows widget host.");
            return;
        }

        ComWrappersSupport.InitializeComWrappers();
        var factory = new WidgetProviderClassFactory();
        var result = CoRegisterClassObject(
            ProviderClassId,
            factory,
            ClsctxLocalServer,
            RegclsMultipleUse,
            out var registration);
        Marshal.ThrowExceptionForHR(result);

        using var shutdown = new ManualResetEvent(false);
        AppDomain.CurrentDomain.ProcessExit += (_, _) => shutdown.Set();
        shutdown.WaitOne();
        Marshal.ThrowExceptionForHR(CoRevokeClassObject(registration));
    }

    private const uint ClsctxLocalServer = 0x4;
    private const uint RegclsMultipleUse = 0x1;

    [DllImport("ole32.dll", ExactSpelling = true)]
    private static extern int CoRegisterClassObject(
        [MarshalAs(UnmanagedType.LPStruct)] Guid classId,
        [MarshalAs(UnmanagedType.IUnknown)] object classFactory,
        uint context,
        uint flags,
        out uint registration);

    [DllImport("ole32.dll", ExactSpelling = true)]
    private static extern int CoRevokeClassObject(uint registration);
}

[ComImport]
[InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
[Guid("00000001-0000-0000-C000-000000000046")]
internal interface IClassFactory
{
    [PreserveSig]
    int CreateInstance(IntPtr outer, ref Guid interfaceId, out IntPtr instance);

    [PreserveSig]
    int LockServer(bool lockServer);
}

internal sealed class WidgetProviderClassFactory : IClassFactory
{
    private static readonly Guid UnknownInterfaceId = new("00000000-0000-0000-C000-000000000046");

    public int CreateInstance(IntPtr outer, ref Guid interfaceId, out IntPtr instance)
    {
        instance = IntPtr.Zero;
        if (outer != IntPtr.Zero)
        {
            return unchecked((int)0x80040110);
        }

        var providerInterfaceId = typeof(IWidgetProvider).GUID;
        if (interfaceId != providerInterfaceId && interfaceId != UnknownInterfaceId)
        {
            return unchecked((int)0x80004002);
        }

        instance = MarshalInspectable<IWidgetProvider>.FromManaged(new WidgetProvider());
        return 0;
    }

    public int LockServer(bool lockServer) => 0;
}
