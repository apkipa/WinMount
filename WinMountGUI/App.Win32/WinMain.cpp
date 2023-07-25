#include "pch.h"

struct dll_handle_traits {
    using type = HMODULE;
    static void close(type value) noexcept {
        WINRT_VERIFY_(1, FreeLibrary(value));
    }
    static type invalid() noexcept {
        return nullptr;
    }
};
using dll_handle = winrt::handle_type<dll_handle_traits>;

auto load_dll(const wchar_t* lib) {
    return dll_handle{ LoadLibrary(lib) };
}

int __declspec(dllimport) app_main(HINSTANCE hInstance, LPWSTR lpCmdLine, int nShowCmd);

// The executable loader in charge of bootstraping the application DLL
int APIENTRY wWinMain(
    _In_ HINSTANCE hInstance,
    _In_opt_ HINSTANCE /*hPrevInstance*/,
    _In_ LPWSTR lpCmdLine,
    _In_ int nShowCmd
) {
    winrt::init_apartment(winrt::apartment_type::single_threaded);

    // TODO: Function-level try-catch

    // See https://github.com/microsoft/microsoft-ui-xaml/issues/7260#issuecomment-1231314776
    // & https://github.com/CommunityToolkit/Microsoft.Toolkit.Win32/blob/master/Microsoft.Toolkit.Win32.UI.XamlApplication/XamlApplication.cpp#L142
    dll_handle preload_libs[] = {
        load_dll(L"twinapi.appcore.dll"),
        load_dll(L"threadpoolwinrt.dll"),
    };

    int ret = app_main(hInstance, lpCmdLine, nShowCmd);

    // Clean up leftover messages
    MSG msg;
    while (PeekMessageW(&msg, nullptr, 0, 0, PM_REMOVE)) {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }

    return ret;
}
