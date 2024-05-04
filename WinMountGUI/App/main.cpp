#include "pch.h"
#include "App.h"
#include <winrt/Tenkai.h>

using namespace winrt;

// The raw application entry point in DLL; logical entry point resides in
// App::App() and App::OnLaunched()
int __declspec(dllexport) app_main(HINSTANCE hInstance, LPWSTR lpCmdLine, int nShowCmd) {
    init_apartment(apartment_type::single_threaded);
    Tenkai::AppService::InitializeForApplication([&](auto&&) {
        detach_abi(WinMount::App::App{});
    });
    Tenkai::AppService::RunLoop();
    Tenkai::AppService::UninitializeForApplication();

    return 0;
}
