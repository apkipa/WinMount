#include "pch.h"
#include "App.h"
#include "Win32Xaml.h"

using namespace winrt;

// The raw application entry point in DLL; logical entry point resides in
// App::App() and App::OnLaunched()
int __declspec(dllexport) app_main(HINSTANCE hInstance, LPWSTR lpCmdLine, int nShowCmd) {
    EnableMouseInPointer(true);

    InitializeWin32Xaml(hInstance);

    WinMount::App::App app;
    auto wxm = Windows::UI::Xaml::Hosting::WindowsXamlManager::InitializeForCurrentThread();

    Win32Xaml::AppService::RunLoop();

    app.Exit();
    wxm.Close();

    return 0;
}
