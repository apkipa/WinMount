#pragma once
#include <winrt/Tenkai.UI.Xaml.h>
#include "MainWindow.g.h"

namespace winrt::WinMount::App::implementation {
    struct MainWindow : MainWindowT<MainWindow> {
        MainWindow() = default;

        void InitializeComponent();

        // TODO: Investigate why final_release is not called (no root_implements?)
        //       (works in base Window)
        //void final_release(std::unique_ptr<MainWindow> p);
    };
}

namespace winrt::WinMount::App::factory_implementation {
    struct MainWindow : MainWindowT<MainWindow, implementation::MainWindow> {};
}
