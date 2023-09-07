#include "pch.h"
#include "Pages\MainPage.h"
#include "Pages\MainPage.g.cpp"
#include "util.hpp"

using namespace winrt;
using namespace Windows::UI::Xaml;
using namespace Windows::UI::Xaml::Navigation;

namespace winrt {
    namespace muxc = Microsoft::UI::Xaml::Controls;
}

// TODO: AllocateLoadingUsage to automatically manage top loading animation

namespace winrt::WinMount::App::Pages::implementation {
    MainPage::MainPage() {}
    void MainPage::OnNavigatedTo(NavigationEventArgs const& e) {
        m_client = util::winrt::unbox_any<::WinMount::WinMountClient>(e.Parameter());
        m_vm = make_self<Items::implementation::MainViewModel>(m_client);

        // Remove focus visual on startup
        util::winrt::run_when_loaded([this](auto&&) {
            util::winrt::force_focus_element(*this, FocusState::Programmatic);

            // Link NavigationViewItem's with their corresponding pages
            this->Nvi_Filesystems().Tag(box_value(xaml_typename<Pages::MainFsPage>()));
            this->Nvi_FilesystemServers().Tag(box_value(xaml_typename<Pages::MainFsrvPage>()));
            this->Nvi_About().Tag(box_value(xaml_typename<Pages::MainAboutPage>()));

            // Automatically load one item into ContentFrame
            auto main_nav_view = this->MainNavView();
            auto default_nvi = this->Nvi_Filesystems();
            main_nav_view.SelectedItem(default_nvi);
            this->UpdateNavigationFrame(default_nvi);
        }, this);
    }
    void MainPage::MainNavView_ItemInvoked(
        IInspectable const&,
        muxc::NavigationViewItemInvokedEventArgs const& e
    ) {
        auto nvi = e.InvokedItemContainer();
        if (nvi.IsSelected()) { return; }
        this->UpdateNavigationFrame(nvi);
    }
    void MainPage::UpdateNavigationFrame(muxc::NavigationViewItemBase const& nvi) {
        auto cf = ContentFrame();
        if (auto tn = nvi.Tag().try_as<Windows::UI::Xaml::Interop::TypeName>()) {
            //this->MainNavViewHeaderTextBlock().Text(nvi.Content().as<hstring>());
            cf.Navigate(*tn, *this);
        }
        else {
            // Assuming invoked settings
            //this->MainNavViewHeaderTextBlock().Text(nvi.Content().as<hstring>());
            cf.Navigate(xaml_typename<Pages::MainSettingsPage>(), *this);
        }
        cf.BackStack().Clear();
    }
}
