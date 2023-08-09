#include "pch.h"
#include "Pages\MainAboutPage.h"
#if __has_include("Pages\MainAboutPage.g.cpp")
#include "Pages\MainAboutPage.g.cpp"
#endif

#include "Pages\MainPage.h"
#include "WinMountClient.hpp"

using namespace winrt;
using namespace Windows::UI::Xaml;
using namespace Windows::UI::Xaml::Navigation;
using namespace Windows::UI::Xaml::Controls;

namespace winrt::WinMount::App::Pages::implementation {
    MainAboutPage::MainAboutPage() {}
    void MainAboutPage::OnNavigatedTo(NavigationEventArgs const& e) {
        auto parent_page = e.Parameter().as<MainPage>();

        auto const& client = parent_page->m_client;

        this->AppHeaderTextBlock().Text(std::format(L"WinMount GUI v{}", ::WinMount::CLIENT_VERSION));
        this->DaemonVersionTextBlock().Text(std::format(L"Daemon version: {}", client.get_daemon_version()));
    }
    void MainAboutPage::ViewLicensesButton_Click(IInspectable const&, RoutedEventArgs const&) {
        ContentDialog cd;
        cd.XamlRoot(this->XamlRoot());
        cd.Title(box_value(L"Open Source Licenses (GUI)"));
        cd.Content(box_value(L""
#include "AppDepsLicense.rawstr.txt"
        ));
        cd.CloseButtonText(L"Close");
        cd.ShowAsync();
        util::winrt::run_when_loaded([](ContentDialog const& cd) {
            auto sv = cd.GetTemplateChild(L"ContentScrollViewer").as<ScrollViewer>();
            sv.VerticalScrollBarVisibility(ScrollBarVisibility::Auto);
        }, cd);
    }
}
