using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Globalization;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Threading;
using System.Threading.Tasks;
using Microsoft.Windows.Widgets;
using Microsoft.Windows.Widgets.Providers;

namespace WhatNWhenWidgetProvider;

[System.Runtime.InteropServices.ComVisible(true)]
[System.Runtime.InteropServices.ComDefaultInterface(typeof(IWidgetProvider))]
[System.Runtime.InteropServices.Guid("84BF6B95-86A4-45B9-A9E8-775D043D1F9C")]
public sealed class WidgetProvider : IWidgetProvider
{
    private const string DefinitionId = "WhatNWhen.LockScreenCalendar";
    private const string ApplicationUserModelId = "56890AusnadianProductions.NotionCalendarWidget_jamqazxzm4680!WhatNWhen";
    private const string ActivationEventName = @"Local\WhatNWhen.DesktopActivation.v1";
    private static readonly JsonSerializerOptions TemplateSerializerOptions = new()
    {
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull
    };
    private static readonly string SmallTemplate = BuildTemplate(WidgetSize.Small);
    private static readonly string MediumTemplate = BuildTemplate(WidgetSize.Medium);
    private static readonly string LargeTemplate = BuildTemplate(WidgetSize.Large);

    private static readonly ConcurrentDictionary<string, WidgetSize> ActiveWidgets = new();
    private static readonly Timer RefreshTimer = new(
        _ => RefreshActiveWidgets(),
        null,
        TimeSpan.FromMinutes(1),
        TimeSpan.FromMinutes(30));

    public void CreateWidget(WidgetContext widgetContext)
    {
        if (widgetContext.DefinitionId != DefinitionId)
        {
            throw new InvalidOperationException($"Unknown widget definition: {widgetContext.DefinitionId}");
        }

        UpdateWidget(widgetContext.Id, widgetContext.Size);
    }

    public void DeleteWidget(string widgetId, string customState)
    {
        ActiveWidgets.TryRemove(widgetId, out _);
    }

    public void OnActionInvoked(WidgetActionInvokedArgs actionInvokedArgs)
    {
        if (actionInvokedArgs.Verb != "open-app")
        {
            return;
        }

        if (TrySignalRunningApp())
        {
            return;
        }

        var explorerPath = System.IO.Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.Windows),
            "explorer.exe");
        using var launchedProcess = Process.Start(new ProcessStartInfo
        {
            FileName = explorerPath,
            Arguments = $"shell:AppsFolder\\{ApplicationUserModelId}",
            UseShellExecute = true
        });
        if (launchedProcess is null)
        {
            throw new InvalidOperationException("Windows did not start the What-N-When app.");
        }

        _ = Task.Run(() =>
        {
            try
            {
                for (var attempt = 0; attempt < 40; attempt++)
                {
                    Thread.Sleep(250);
                    if (TrySignalRunningApp())
                    {
                        return;
                    }
                }

                Console.Error.WriteLine("The What-N-When app did not become ready for widget activation.");
            }
            catch (Exception error)
            {
                Console.Error.WriteLine($"Could not activate the What-N-When app from its widget: {error}");
            }
        });
    }

    private static bool TrySignalRunningApp()
    {
        var eventHandle = OpenEventW(EventModifyState, false, ActivationEventName);
        if (eventHandle == IntPtr.Zero)
        {
            var error = Marshal.GetLastWin32Error();
            if (error != ErrorFileNotFound)
            {
                Trace.TraceWarning(
                    "Could not open the desktop activation event (Win32 error {0}); launching the app instead.",
                    error);
            }

            return false;
        }

        try
        {
            if (!SetEvent(eventHandle))
            {
                var error = Marshal.GetLastWin32Error();
                Trace.TraceWarning(
                    "Could not signal the desktop activation event (Win32 error {0}); launching the app instead.",
                    error);
                return false;
            }
        }
        finally
        {
            CloseHandle(eventHandle);
        }

        return true;
    }

    private const uint EventModifyState = 0x0002;
    private const int ErrorFileNotFound = 2;

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr OpenEventW(uint desiredAccess, bool inheritHandle, string name);

    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetEvent(IntPtr eventHandle);

    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool CloseHandle(IntPtr handle);

    public void OnWidgetContextChanged(WidgetContextChangedArgs contextChangedArgs)
    {
        var context = contextChangedArgs.WidgetContext;
        if (ActiveWidgets.ContainsKey(context.Id))
        {
            ActiveWidgets[context.Id] = context.Size;
        }
        UpdateWidget(context.Id, context.Size);
    }

    public void Activate(WidgetContext widgetContext)
    {
        ActiveWidgets[widgetContext.Id] = widgetContext.Size;
        UpdateWidget(widgetContext.Id, widgetContext.Size);
    }

    public void Deactivate(string widgetId)
    {
        ActiveWidgets.TryRemove(widgetId, out _);
    }

    private static void RefreshActiveWidgets()
    {
        foreach (var widget in ActiveWidgets)
        {
            try
            {
                UpdateWidget(widget.Key, widget.Value);
            }
            catch (Exception error)
            {
                Console.Error.WriteLine($"Could not refresh lock-screen widget {widget.Key}: {error}");
            }
        }
    }

    private static void UpdateWidget(string widgetId, WidgetSize size)
    {
        var now = DateTime.Now;
        var dateFormat = CultureInfo.CurrentCulture.DateTimeFormat;
        var firstDayOfWeek = dateFormat.FirstDayOfWeek;
        var model = new Dictionary<string, object>
        {
            ["monthName"] = now.ToString("MMMM", CultureInfo.CurrentCulture),
            ["weekday"] = now.ToString("dddd", CultureInfo.CurrentCulture),
            ["dayOfMonth"] = now.ToString("%d", CultureInfo.CurrentCulture)
        };
        for (var weekday = 0; weekday < 7; weekday++)
        {
            var dayOfWeek = (DayOfWeek)(((int)firstDayOfWeek + weekday) % 7);
            model[$"weekday{weekday}"] = dateFormat.GetAbbreviatedDayName(dayOfWeek)[..1];
        }

        var firstOfMonth = new DateTime(now.Year, now.Month, 1);
        var firstDayOffset = ((int)firstOfMonth.DayOfWeek - (int)firstDayOfWeek + 7) % 7;
        var todayMarker = CreateTodayMarkerDataUri(now.Day);
        for (var cell = 0; cell < 42; cell++)
        {
            var date = firstOfMonth.AddDays(cell - firstDayOffset);
            var isCurrentDay = date.Date == now.Date;
            model[$"day{cell}"] = date.Day.ToString(CultureInfo.CurrentCulture);
            model[$"color{cell}"] = date.Month == now.Month ? "Default" : "Light";
            model[$"todayCell{cell}"] = isCurrentDay;
            model[$"notTodayCell{cell}"] = !isCurrentDay;
            model[$"todayMarker{cell}"] = isCurrentDay ? todayMarker : string.Empty;
        }

        var data = JsonSerializer.Serialize(model);

        var options = new WidgetUpdateRequestOptions(widgetId)
        {
            Template = size switch
            {
                WidgetSize.Small => SmallTemplate,
                WidgetSize.Large => LargeTemplate,
                _ => MediumTemplate
            },
            Data = data
        };
        WidgetManager.GetDefault().UpdateWidget(options);
    }

    private static string CreateTodayMarkerDataUri(int day)
    {
        var accent = GetWindowsAccentColor();
        var luminance = 0.299 * accent.R + 0.587 * accent.G + 0.114 * accent.B;
        var textColor = luminance > 150 ? Color.Black : Color.White;

        using var bitmap = new Bitmap(64, 64);
        using (var graphics = Graphics.FromImage(bitmap))
        using (var background = new SolidBrush(accent))
        using (var foreground = new SolidBrush(textColor))
        using (var font = new Font("Segoe UI", 25, FontStyle.Bold, GraphicsUnit.Pixel))
        using (var centered = new StringFormat
        {
            Alignment = StringAlignment.Center,
            LineAlignment = StringAlignment.Center
        })
        {
            graphics.SmoothingMode = SmoothingMode.AntiAlias;
            graphics.Clear(Color.Transparent);
            graphics.FillEllipse(background, 1, 1, 62, 62);
            graphics.DrawString(day.ToString(CultureInfo.CurrentCulture), font, foreground, new RectangleF(0, 0, 64, 64), centered);
        }

        using var stream = new System.IO.MemoryStream();
        bitmap.Save(stream, System.Drawing.Imaging.ImageFormat.Png);
        return $"data:image/png;base64,{Convert.ToBase64String(stream.ToArray())}";
    }

    private static Color GetWindowsAccentColor()
    {
        Marshal.ThrowExceptionForHR(DwmGetColorizationColor(out var colorizationColor, out _));
        return Color.FromArgb(
            (byte)(colorizationColor >> 24),
            (byte)(colorizationColor >> 16),
            (byte)(colorizationColor >> 8),
            (byte)colorizationColor);
    }

    private static string BuildTemplate(WidgetSize size)
    {
        static object CreateDateColumn(int cell, WidgetSize size)
        {
            var markerSize = size switch
            {
                WidgetSize.Large => "36px",
                WidgetSize.Medium => "26px",
                _ => "28px"
            };
            var minimumCellHeight = size switch
            {
                WidgetSize.Large => "42px",
                WidgetSize.Medium => "26px",
                _ => "34px"
            };
            return new
            {
                type = "Column",
                width = "stretch",
                minHeight = minimumCellHeight,
                verticalContentAlignment = "Center",
                items = new object[]
                {
                    new
                    {
                        type = "Image",
                        url = $"${{todayMarker{cell}}}",
                        width = markerSize,
                        height = markerSize,
                        horizontalAlignment = "Center",
                        isVisible = $"${{todayCell{cell}}}"
                    },
                    new
                    {
                        type = "TextBlock",
                        text = $"${{day{cell}}}",
                        color = $"${{color{cell}}}",
                        size = "Small",
                        horizontalAlignment = "Center",
                        spacing = "None",
                        isVisible = $"${{notTodayCell{cell}}}"
                    }
                }
            };
        }

        static object CreateDateSummary() =>
            new
            {
                type = "ColumnSet",
                spacing = "Medium",
                columns = new object[]
                {
                    new
                    {
                        type = "Column",
                        width = "90px",
                        verticalContentAlignment = "Center",
                        items = new object[]
                        {
                            new { type = "TextBlock", text = "${dayOfMonth}", weight = "Bolder", size = "ExtraLarge", color = "Accent", spacing = "None" },
                            new { type = "TextBlock", text = "${monthName}", color = "Accent", size = "Small", spacing = "None" }
                        }
                    },
                    new
                    {
                        type = "Column",
                        width = "stretch",
                        separator = true,
                        verticalContentAlignment = "Center",
                        items = new object[]
                        {
                            new
                            {
                                type = "ColumnSet",
                                spacing = "None",
                                columns = new object[]
                                {
                                    new { type = "Column", width = "16px", items = Array.Empty<object>() },
                                    new
                                    {
                                        type = "Column",
                                        width = "stretch",
                                        verticalContentAlignment = "Center",
                                        items = new object[]
                                        {
                                            new { type = "TextBlock", text = "${weekday}", weight = "Bolder", size = "Medium", wrap = false, spacing = "None" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            };

        var items = new List<object> { CreateDateSummary() };
        if (size != WidgetSize.Small)
        {
            var grid = new List<object>
            {
                new
                {
                    type = "ColumnSet",
                    spacing = "None",
                    columns = Enumerable.Range(0, 7)
                        .Select(day => new
                        {
                            type = "Column",
                            width = "stretch",
                            minHeight = size == WidgetSize.Large ? "30px" : "20px",
                            verticalContentAlignment = "Center",
                            items = new[]
                            {
                                new
                                {
                                    type = "TextBlock",
                                    text = $"${{weekday{day}}}",
                                    color = "Light",
                                    size = "Small",
                                    horizontalAlignment = "Center",
                                    spacing = "None"
                                }
                            }
                        })
                        .ToArray()
                }
            };

            for (var week = 0; week < 6; week++)
            {
                var weekIndex = week;
                grid.Add(new
                {
                    type = "ColumnSet",
                    spacing = "None",
                    columns = Enumerable.Range(0, 7)
                        .Select(day => CreateDateColumn(weekIndex * 7 + day, size))
                        .ToArray()
                });
            }

            items.Add(new
            {
                type = "Container",
                style = "emphasis",
                spacing = size == WidgetSize.Medium ? "Small" : "Medium",
                items = grid.ToArray()
            });
        }

        return JsonSerializer.Serialize(new Dictionary<string, object>
        {
            ["type"] = "AdaptiveCard",
            ["$schema"] = "http://adaptivecards.io/schemas/adaptive-card.json",
            ["version"] = "1.5",
            ["selectAction"] = new
            {
                type = "Action.Execute",
                title = "Open calendar",
                verb = "open-app"
            },
            ["body"] = items
        }, TemplateSerializerOptions);
    }

    [DllImport("dwmapi.dll", ExactSpelling = true)]
    private static extern int DwmGetColorizationColor(
        out uint colorizationColor,
        [MarshalAs(UnmanagedType.Bool)] out bool opaqueBlend);
}
