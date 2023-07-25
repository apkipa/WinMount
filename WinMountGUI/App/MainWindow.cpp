#include "pch.h"
#include "MainWindow.h"
#if __has_include("MainWindow.g.cpp")
#include "MainWindow.g.cpp"
#endif
#include "MainPage.h"

using namespace winrt;
using namespace Windows::UI::Xaml;

namespace winrt::WinMount::App::implementation {
    void MainWindow::InitializeComponent() {
        MainWindowT::InitializeComponent();

        this->EnableCustomTitleBarButton_Clicked(nullptr, nullptr);
    }
    void MainWindow::ClickHandler(IInspectable const&, RoutedEventArgs const&) {
        Win32Xaml::Window wnd;
        Windows::UI::Xaml::Controls::Frame frame;
        frame.Navigate(xaml_typename<WinMount::App::MainPage>());
        wnd.Content(frame);
        wnd.Activate();
        //UseTransparentBackground(!UseTransparentBackground());
    }
    void MainWindow::EnableCustomTitleBarButton_Clicked(IInspectable const&, RoutedEventArgs const&) {
        bool should_enable = !this->ExtendsContentIntoTitleBar();
        if (should_enable) {
            this->ExtendsContentIntoTitleBar(true);
            this->SetTitleBar(TopRectangle());
        }
        else {
            this->ExtendsContentIntoTitleBar(false);
            this->SetTitleBar(nullptr);
        }
    }
}
