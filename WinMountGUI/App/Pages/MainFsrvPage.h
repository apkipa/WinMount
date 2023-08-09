#pragma once

#include "Pages\MainFsrvPage.g.h"

#include "Pages\Items.h"

#include "util.hpp"

namespace winrt::WinMount::App::Pages::implementation {
    struct MainFsrvPage : MainFsrvPageT<MainFsrvPage> {
        MainFsrvPage();
        ~MainFsrvPage();

        static hstring IsRunningToStartStopButtonText(bool isRunning);

        void OnNavigatedTo(Windows::UI::Xaml::Navigation::NavigationEventArgs const& e);
        void AddNewFsrvButton_Click(
            Windows::Foundation::IInspectable const&,
            Windows::UI::Xaml::RoutedEventArgs const&
        );
        void ReloadFsrvListButton_Click(
            Windows::Foundation::IInspectable const&,
            Windows::UI::Xaml::RoutedEventArgs const&
        );
        void FsrvItem_StartStopButton_Click(
            Windows::Foundation::IInspectable const& sender,
            Windows::UI::Xaml::RoutedEventArgs const&
        );
        void FsrvListView_SelectionChanged(
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
        Windows::Foundation::IAsyncAction ReloadFsrvListAsync();
        bool SelectFsrvItemById(guid const& id);
        Windows::Foundation::IAsyncOperation<bool> StartFsrvAsync(guid id);
        Windows::Foundation::IAsyncOperation<bool> StopFsrvAsync(guid id);

        com_ptr<MainViewModel> m_main_vm{ nullptr };

        util::winrt::async_storage m_async;
    };
}

namespace winrt::WinMount::App::Pages::factory_implementation {
    struct MainFsrvPage : MainFsrvPageT<MainFsrvPage, implementation::MainFsrvPage> {};
}
