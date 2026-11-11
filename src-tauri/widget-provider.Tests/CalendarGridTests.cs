using System;
using WhatNWhenWidgetProvider;
using Xunit;

namespace WhatNWhenWidgetProvider.Tests;

public sealed class CalendarGridTests
{
    [Fact]
    public void CreatesSixWeeksStartingOnConfiguredWeekday()
    {
        var days = CalendarGrid.CreateMonth(new DateTime(2024, 2, 15), DayOfWeek.Monday);

        Assert.Equal(42, days.Length);
        Assert.Equal(new DateTime(2024, 1, 29), days[0]);
        Assert.Equal(DayOfWeek.Monday, days[0].DayOfWeek);
        Assert.Equal(new DateTime(2024, 3, 10), days[^1]);
    }

    [Fact]
    public void IncludesLeapDayInFebruaryGrid()
    {
        var days = CalendarGrid.CreateMonth(new DateTime(2024, 2, 29), DayOfWeek.Sunday);

        Assert.Contains(new DateTime(2024, 2, 29), days);
        Assert.Equal(new DateTime(2024, 2, 1), days[4]);
    }

    [Fact]
    public void SupportsMonthsBeginningOnFirstDayOfWeek()
    {
        var days = CalendarGrid.CreateMonth(new DateTime(2024, 9, 1), DayOfWeek.Sunday);

        Assert.Equal(new DateTime(2024, 9, 1), days[0]);
        Assert.Equal(new DateTime(2024, 10, 12), days[^1]);
    }
}
