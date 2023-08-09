#pragma once

#include "Pages\MainSettingsPage.g.h"

namespace winrt::WinMount::App::Pages::implementation {
    struct MainSettingsPage : MainSettingsPageT<MainSettingsPage> {
        MainSettingsPage();
    };
}

namespace winrt::WinMount::App::Pages::factory_implementation {
    struct MainSettingsPage : MainSettingsPageT<MainSettingsPage, implementation::MainSettingsPage> {};
}
