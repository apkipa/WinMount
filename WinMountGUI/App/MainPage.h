#pragma once

#include "MainPage.g.h"

namespace winrt::WinMount::App::implementation {
    struct MainPage : MainPageT<MainPage> {
        MainPage() {
            // Xaml objects should not call InitializeComponent during construction.
            // See https://github.com/microsoft/cppwinrt/tree/master/nuget#initializecomponent
        }

        void ClickHandler(Windows::Foundation::IInspectable const& sender, Windows::UI::Xaml::RoutedEventArgs const& args);
        void ShowDlgHandler(Windows::Foundation::IInspectable const& sender, Windows::UI::Xaml::RoutedEventArgs const& args);
    };
}

namespace winrt::WinMount::App::factory_implementation {
    struct MainPage : MainPageT<MainPage, implementation::MainPage> {};
}
