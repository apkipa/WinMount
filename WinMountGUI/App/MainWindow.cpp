#include "pch.h"
#include "MainWindow.h"
#if __has_include("MainWindow.g.cpp")
#include "MainWindow.g.cpp"
#endif
#include "Pages\MainPage.h"
#include "Pages\DaemonManagePage.h"
#include "util.hpp"

using namespace winrt;
using namespace Windows::UI::Xaml;

namespace winrt::WinMount::App::implementation {
    void MainWindow::InitializeComponent() {
        MainWindowT::InitializeComponent();

        this->ExtendsContentIntoTitleBar(true);
        this->SetTitleBar(BackgroundDragArea());
        {   // Update title bar button colors
            using Windows::UI::Color;
            using Windows::UI::Colors;
            auto tb = this->TitleBar();
            Color bg_normal_clr, bg_hover_clr, bg_pressed_clr;
            bg_normal_clr = Colors::Transparent();
            bg_hover_clr = Windows::UI::ColorHelper::FromArgb(0x19, 0, 0, 0);
            bg_pressed_clr = Windows::UI::ColorHelper::FromArgb(0x33, 0, 0, 0);
            tb.ButtonBackgroundColor(bg_normal_clr);
            tb.ButtonInactiveBackgroundColor(bg_normal_clr);
            tb.ButtonHoverBackgroundColor(bg_hover_clr);
            tb.ButtonPressedBackgroundColor(bg_pressed_clr);
        }

        Pages::DaemonManagePageNavParams params = {
            .ScenarioMode = Pages::DaemonManagePageScenarioMode::FirstLoad
        };
        this->MainFrame().Navigate(xaml_typename<Pages::DaemonManagePage>(), box_value(params));

        [](MainWindow* that) -> fire_forget_except {
            auto main_frame = that->MainFrame();
            // TODO: Remember to close connection
            auto result = co_await main_frame.Content().as<Pages::DaemonManagePage>().GetConnectionResultAsync();
            main_frame.Navigate(xaml_typename<Pages::MainPage>(), result);
            main_frame.BackStack().Clear();
        }(this);
    }
}
