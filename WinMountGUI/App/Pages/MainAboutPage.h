#pragma once

#include "Pages\MainAboutPage.g.h"

namespace winrt::WinMount::App::Pages::implementation {
    struct MainAboutPage : MainAboutPageT<MainAboutPage> {
        MainAboutPage();

        void OnNavigatedTo(Windows::UI::Xaml::Navigation::NavigationEventArgs const& e);
        void ViewLicensesButton_Click(
            Windows::Foundation::IInspectable const&,
            Windows::UI::Xaml::RoutedEventArgs const&
        );
    };
}

namespace winrt::WinMount::App::Pages::factory_implementation {
    struct MainAboutPage : MainAboutPageT<MainAboutPage, implementation::MainAboutPage> {};
}
