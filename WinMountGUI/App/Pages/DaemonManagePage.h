#pragma once

#include "Pages\DaemonManagePage.g.h"

#include "util.hpp"

namespace winrt::WinMount::App::Pages::implementation {
    struct DaemonManagePage : DaemonManagePageT<DaemonManagePage> {
        DaemonManagePage();

        void OnNavigatedTo(Windows::UI::Xaml::Navigation::NavigationEventArgs const& e);

        Windows::Foundation::IAsyncOperation<Windows::Foundation::IInspectable> GetConnectionResultAsync();

    private:
        enum class ConnectionResultKind {
            Success = 0,
            Unknown = -1,
            InternetCannotConnect = -2,
        };

        void SignalConnectionFinished();
        util::winrt::task<ConnectionResultKind> TryConnect();
        util::winrt::task<> ConnectAndNotify();

        util::winrt::awaitable_event m_ae_result;
        Windows::Foundation::IInspectable m_result;
        util::winrt::async_storage m_async;
    };
}

namespace winrt::WinMount::App::Pages::factory_implementation {
    struct DaemonManagePage : DaemonManagePageT<DaemonManagePage, implementation::DaemonManagePage> {};
}
