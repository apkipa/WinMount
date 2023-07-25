#include "pch.h"
#include "MainPage.h"
#include "MainPage.g.cpp"

using namespace winrt;
using namespace Windows::UI::Xaml;

namespace winrt::WinMount::App::implementation {
    void MainPage::ClickHandler(IInspectable const&, RoutedEventArgs const&) {
        myButton().Content(box_value(L"Clicked"));
    }
    void MainPage::ShowDlgHandler(IInspectable const&, RoutedEventArgs const&) {
        using namespace Windows::UI::Xaml::Controls;
        ContentDialog cd;
        cd.XamlRoot(XamlRoot());
        cd.Title(box_value(L"Title"));
        cd.Content(box_value(L"Content"));
        cd.CloseButtonText(L"Close");
        cd.ShowAsync();
    }
}
