#pragma once
#include "Win32Xaml.h"
#include "MainWindow.g.h"

namespace winrt::WinMount::App::implementation {
    struct MainWindow : MainWindowT<MainWindow> {
        MainWindow() = default;

        void InitializeComponent();

        void ClickHandler(Windows::Foundation::IInspectable const& sender, Windows::UI::Xaml::RoutedEventArgs const& args);

        void EnableCustomTitleBarButton_Clicked(Windows::Foundation::IInspectable const& sender, Windows::UI::Xaml::RoutedEventArgs const& args);

        // TODO: Investigate why final_release is not called (no root_implements?)
        //       (works in base Window)
        //void final_release(std::unique_ptr<MainWindow> p);
    };
}

namespace winrt::WinMount::App::factory_implementation {
    struct MainWindow : MainWindowT<MainWindow, implementation::MainWindow> {};
}
