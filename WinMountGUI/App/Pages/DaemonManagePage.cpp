#include "pch.h"
#include "Pages\DaemonManagePage.h"
#if __has_include("Pages\DaemonManagePage.g.cpp")
#include "Pages\DaemonManagePage.g.cpp"
#endif
#include "WinMountClient.hpp"

using namespace winrt;
using namespace Windows::Foundation;
using namespace Windows::UI::Xaml;
using namespace Windows::UI::Xaml::Controls;
using namespace Windows::UI::Xaml::Navigation;

void start_external_daemon() {
    // TODO: Remove these comments
    /*static constexpr std::wstring_view DAEMON_EXECUTABLE_NAME = L"WinMountCore.exe";
    auto buf_len = GetCurrentDirectoryW(0, nullptr);
    check_bool(buf_len != 0);
    std::unique_ptr<wchar_t[]> buf{ new wchar_t[buf_len + 1 + DAEMON_EXECUTABLE_NAME.size()] };
    check_bool(GetCurrentDirectoryW(buf_len, buf.get()));
    buf[buf_len - 1] = L'\\';
    wmemcpy(buf.get() + buf_len, DAEMON_EXECUTABLE_NAME.data(), DAEMON_EXECUTABLE_NAME.size());
    buf[buf_len + DAEMON_EXECUTABLE_NAME.size()] = L'\0';

    wchar_t cmd_line[] = L"WinMountCore.exe daemon";
    STARTUPINFOW si{ .cb = sizeof si };
    si.dwFlags = STARTF_FORCEOFFFEEDBACK;
    PROCESS_INFORMATION pi;
    check_bool(CreateProcessW(
        buf.get(),
        cmd_line,
        nullptr,
        nullptr,
        false,
        CREATE_NO_WINDOW,
        nullptr,
        nullptr,
        &si,
        &pi
    ));
    CloseHandle(pi.hThread);
    CloseHandle(pi.hProcess);*/

    wchar_t cmd_line[] = L"WinMountCore.exe daemon";
    STARTUPINFOW si{ .cb = sizeof si };
    si.dwFlags = STARTF_FORCEOFFFEEDBACK;
    PROCESS_INFORMATION pi;
    check_bool(CreateProcessW(
        nullptr, cmd_line,
        nullptr, nullptr,
        false,
        CREATE_NO_WINDOW,
        nullptr,
        nullptr,
        &si, &pi
    ));
    CloseHandle(pi.hThread);
    CloseHandle(pi.hProcess);
}

// TODO: Finish DaemonManagePage

namespace winrt::WinMount::App::Pages::implementation {
    DaemonManagePage::DaemonManagePage(): m_result(nullptr) {}
    void DaemonManagePage::OnNavigatedTo(NavigationEventArgs const& e) {
        auto params = unbox_value<DaemonManagePageNavParams>(e.Parameter());

        if (params.ScenarioMode == DaemonManagePageScenarioMode::FirstLoad) {
            m_async.cancel_and_run([](DaemonManagePage* that) -> IAsyncAction {
                auto cancellation_token = co_await get_cancellation_token();
                cancellation_token.enable_propagation();

                switch (co_await that->TryConnect()) {
                case ConnectionResultKind::Success:
                    that->SignalConnectionFinished();
                    break;
                case ConnectionResultKind::InternetCannotConnect:
                    // Start the daemon, then try again
                    start_external_daemon();
                    co_await that->ConnectAndNotify();
                    break;
                default:
                    // TODO: Show configuration UI
                    break;
                }
            }, this);
        }
        else {
            // TODO...
            throw hresult_not_implemented();
        }
    }
    IAsyncOperation<IInspectable> DaemonManagePage::GetConnectionResultAsync() {
        auto cancellation_token = co_await get_cancellation_token();
        cancellation_token.enable_propagation();

        auto strong_this = this->get_strong();

        co_await m_ae_result;

        co_return m_result;
    }
    void DaemonManagePage::SignalConnectionFinished() {
        m_ae_result.set();
    }
    auto DaemonManagePage::TryConnect() -> util::winrt::task<ConnectionResultKind> {
        auto cancellation_token = co_await get_cancellation_token();
        cancellation_token.enable_propagation();

        try {
            auto client = co_await ::WinMount::connect_winmount_client(L"ws://127.0.0.1:19423/ws");
            m_result = util::winrt::box_any(std::move(client));
            co_return ConnectionResultKind::Success;
        }
        catch (hresult_error const& e) {
            if (e.code() == WININET_E_CANNOT_CONNECT) {
                co_return ConnectionResultKind::InternetCannotConnect;
            }
            util::winrt::log_current_exception();
        }
        catch (...) {
            util::winrt::log_current_exception();
        }
        co_return ConnectionResultKind::Unknown;
    }
    util::winrt::task<> DaemonManagePage::ConnectAndNotify() {
        auto cancellation_token = co_await get_cancellation_token();
        cancellation_token.enable_propagation();

        m_result = util::winrt::box_any(co_await ::WinMount::connect_winmount_client(L"ws://127.0.0.1:19423/ws"));
        this->SignalConnectionFinished();
    }
}
