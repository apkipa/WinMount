#pragma once

#include "Pages\MainPage.g.h"
#include "WinMountClient.hpp"
#include "Items\Items.h"

#include "util.hpp"

namespace winrt::WinMount::App::Pages::implementation {
    struct MainFsPage;
    struct MainFsrvPage;
    struct MainSettingsPage;
    struct MainAboutPage;

    struct MainPage : MainPageT<MainPage> {
        MainPage();

        void OnNavigatedTo(Windows::UI::Xaml::Navigation::NavigationEventArgs const& e);
        void MainNavView_ItemInvoked(
            Windows::Foundation::IInspectable const&,
            Microsoft::UI::Xaml::Controls::NavigationViewItemInvokedEventArgs const& e
        );

        WinMount::App::Items::MainViewModel ViewModel() { return *m_vm.get(); }

    private:
        friend MainFsPage;
        friend MainFsrvPage;
        friend MainSettingsPage;
        friend MainAboutPage;

        void UpdateNavigationFrame(Microsoft::UI::Xaml::Controls::NavigationViewItemBase const& nvi);

        ::WinMount::WinMountClient m_client{ nullptr };

        com_ptr<Items::implementation::MainViewModel> m_vm{ nullptr };
    };
}

namespace winrt::WinMount::App::Pages::factory_implementation {
    struct MainPage : MainPageT<MainPage, implementation::MainPage> {};
}
