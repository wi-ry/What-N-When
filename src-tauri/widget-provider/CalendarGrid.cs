using System;

namespace WhatNWhenWidgetProvider;

internal static class CalendarGrid
{
    internal static DateTime[] CreateMonth(DateTime currentDate, DayOfWeek firstDayOfWeek)
    {
        var firstOfMonth = new DateTime(currentDate.Year, currentDate.Month, 1);
        var firstDayOffset = ((int)firstOfMonth.DayOfWeek - (int)firstDayOfWeek + 7) % 7;
        var days = new DateTime[42];

        for (var cell = 0; cell < days.Length; cell++)
        {
            days[cell] = firstOfMonth.AddDays(cell - firstDayOffset);
        }

        return days;
    }
}
