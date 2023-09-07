#pragma once

#include "Controls\ItemConfigEditControl.h"

#include "Pages\MainFsPage.g.h"

#include "MainPage.h"
#include "util.hpp"

namespace winrt::WinMount::App::Pages::implementation {
    struct MainFsPage : MainFsPageT<MainFsPage> {
        MainFsPage();
        ~MainFsPage();

        void OnNavigatedTo(Windows::UI::Xaml::Navigation::NavigationEventArgs const& e);
        void AddNewFsButton_Click(
            Windows::Foundation::IInspectable const&,
            Windows::UI::Xaml::RoutedEventArgs const&
        );
        void ReloadFsListButton_Click(
            Windows::Foundation::IInspectable const&,
            Windows::UI::Xaml::RoutedEventArgs const&
        );
        void FsItem_StartStopButton_Click(
            Windows::Foundation::IInspectable const& sender,
            Windows::UI::Xaml::RoutedEventArgs const&
        );
        void FsListView_SelectionChanged(
            Windows::Foundation::IInspectable const&,
            Windows::UI::Xaml::Controls::SelectionChangedEventArgs const& e
        );
        void DetailsAddNew_FsTypeComboBox_SelectionChanged(
            Windows::Foundation::IInspectable const&,
            Windows::UI::Xaml::Controls::SelectionChangedEventArgs const& e
        );
        void DetailsAddNew_CreateButton_Click(
            Windows::Foundation::IInspectable const&,
            Windows::UI::Xaml::RoutedEventArgs const&
        );
        void DetailsEditCurrent_DeleteButton_Click(
            Windows::Foundation::IInspectable const&,
            Windows::UI::Xaml::RoutedEventArgs const&
        );
        void DetailsEditCurrent_CommitButton_Click(
            Windows::Foundation::IInspectable const&,
            Windows::UI::Xaml::RoutedEventArgs const&
        );

    private:
        Windows::Foundation::IAsyncAction ReloadFsListAsync();
        bool SelectFsItemById(guid const& id);
        Windows::Foundation::IAsyncOperation<bool> StartFsAsync(guid id);
        Windows::Foundation::IAsyncOperation<bool> StopFsAsync(guid id);
        void StartFs(guid const& id);
        void StopFs(guid const& id);

        MainPage* m_parent;

        util::winrt::async_storage m_async;
    };
}

namespace winrt::WinMount::App::Pages::factory_implementation {
    struct MainFsPage : MainFsPageT<MainFsPage, implementation::MainFsPage> {};
}
