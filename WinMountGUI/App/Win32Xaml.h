#pragma once
#include "Win32Xaml.AppService.g.h"
#include "Win32Xaml.WindowTitleBar.g.h"
#include "Win32Xaml.Window.g.h"
#include "Win32Xaml.ShellIcon.g.h"
#include <windows.ui.xaml.hosting.desktopwindowxamlsource.h>
#include <dcomp.h>
#include <ShellScalingApi.h>

// Win32Xaml feature gates - BEGIN

// TODO: Add feature gate for `UIElement as Title Bar`
#define WIN32XAML_ENABLE_LAYOUT_SYNCHRONIZATION 1
#define WIN32XAML_LAYOUT_SYNCHRONIZATION_USE_ALTERNATIVE 0
#define WIN32XAML_ENABLE_SAFE_TEARDOWN 0

// Win32Xaml feature gates - END

void InitializeWin32Xaml(HINSTANCE hInstance);

namespace winrt::Win32Xaml::implementation {
    struct GdiIconSet;
    struct Window;

    enum class CaptionButtonKind {
        None = 0,
        Minimize = 1,
        Maximize = 2,
        Restore = 3,
        Close = 4,
    };
    enum CaptionButtonState {
        None = 0,
        PointerOver,
        Pressed,
        Inactive,
        CaptionButtonStateLastIndex,
    };

    struct AppService {
        AppService() = default;

        static bool AutoQuit();
        static void AutoQuit(bool value);
        static void Exit();
        static void RunLoop();
    };

    struct WindowTitleBar : WindowTitleBarT<WindowTitleBar> {
        Windows::Foundation::IReference<Windows::UI::Color> ButtonBackgroundColor() { return m_btn_bkg_clr; }
        void ButtonBackgroundColor(Windows::Foundation::IReference<Windows::UI::Color> const& value) {
            m_btn_bkg_clr = value; NotifyWindowUpdate();
        }
        Windows::Foundation::IReference<Windows::UI::Color> ButtonForegroundColor() { return m_btn_fore_clr; }
        void ButtonForegroundColor(Windows::Foundation::IReference<Windows::UI::Color> const& value) {
            m_btn_fore_clr = value; NotifyWindowUpdate();
        }
        Windows::Foundation::IReference<Windows::UI::Color> ButtonHoverBackgroundColor() { return m_btn_hover_bkg_clr; }
        void ButtonHoverBackgroundColor(Windows::Foundation::IReference<Windows::UI::Color> const& value) {
            m_btn_hover_bkg_clr = value; NotifyWindowUpdate();
        }
        Windows::Foundation::IReference<Windows::UI::Color> ButtonHoverForegroundColor() { return m_btn_hover_fore_clr; }
        void ButtonHoverForegroundColor(Windows::Foundation::IReference<Windows::UI::Color> const& value) {
            m_btn_hover_fore_clr = value; NotifyWindowUpdate();
        }
        Windows::Foundation::IReference<Windows::UI::Color> ButtonPressedBackgroundColor() { return m_btn_pressed_bkg_clr; }
        void ButtonPressedBackgroundColor(Windows::Foundation::IReference<Windows::UI::Color> const& value) {
            m_btn_pressed_bkg_clr = value; NotifyWindowUpdate();
        }
        Windows::Foundation::IReference<Windows::UI::Color> ButtonPressedForegroundColor() { return m_btn_pressed_fore_clr; }
        void ButtonPressedForegroundColor(Windows::Foundation::IReference<Windows::UI::Color> const& value) {
            m_btn_pressed_fore_clr = value; NotifyWindowUpdate();
        }
        Windows::Foundation::IReference<Windows::UI::Color> ButtonInactiveBackgroundColor() { return m_btn_inactive_bkg_clr; }
        void ButtonInactiveBackgroundColor(Windows::Foundation::IReference<Windows::UI::Color> const& value) {
            m_btn_inactive_bkg_clr = value; NotifyWindowUpdate();
        }
        Windows::Foundation::IReference<Windows::UI::Color> ButtonInactiveForegroundColor() { return m_btn_inactive_fore_clr; }
        void ButtonInactiveForegroundColor(Windows::Foundation::IReference<Windows::UI::Color> const& value) {
            m_btn_inactive_fore_clr = value; NotifyWindowUpdate();
        }

    private:
        friend struct Window;

        using NullableColor = Windows::Foundation::IReference<Windows::UI::Color>;

        void NotifyWindowUpdate() {
            // Sends WM_PAINT to root window
            InvalidateRect(m_root_hwnd, nullptr, false);
        }

        NullableColor m_btn_bkg_clr, m_btn_fore_clr;
        NullableColor m_btn_hover_bkg_clr, m_btn_hover_fore_clr;
        NullableColor m_btn_pressed_bkg_clr, m_btn_pressed_fore_clr;
        NullableColor m_btn_inactive_bkg_clr, m_btn_inactive_fore_clr;

        HWND m_root_hwnd;
    };

    struct Window : WindowT<Window> {
        Window();
        ~Window();

        void Activate();
        void SetTitleBar(Windows::UI::Xaml::UIElement const& element);
        void Close();

        bool IsMain() { return m_is_main; }
        hstring Title();
        void Title(hstring const& value);
        Windows::UI::Xaml::UIElement Content();
        void Content(Windows::UI::Xaml::UIElement const& value);
        bool ExtendsContentIntoTitleBar();
        void ExtendsContentIntoTitleBar(bool value);
        bool UseTransparentBackground();
        void UseTransparentBackground(bool value);
        Win32Xaml::WindowTitleBar TitleBar() { return *m_title_bar; }

        event_token Closed(Windows::Foundation::TypedEventHandler<Win32Xaml::Window, Windows::Foundation::IInspectable> const& handler);
        void Closed(event_token const& token) noexcept;

        // Non-midl methods
        HWND GetRootHwnd();

    private:
        friend void ::InitializeWin32Xaml(HINSTANCE hInstance);
        friend struct AppService;

        unsigned GetClientTopPadding(void) const;
        LRESULT WindowProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam);
        LRESULT InputSinkWindowProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam);
        void EnsureInputSinkWindow(void);
        void EnterFramelessMode(void);
        void LeaveFramelessMode(void);
        void InitializeDComp(void);
        void CommitDComp(void);
        void UpdateCaptionVisibility(bool visible);
        bool UpdateCaptionLayout(void);
        void RedrawCaptionButton(CaptionButtonKind kind);
        void RedrawCaption(void);
        bool UpdateAndRedrawCaption(
            CaptionButtonState bs_min, CaptionButtonState bs_max, CaptionButtonState bs_close
        );
        void EnsureCaptionResource(void);
        void ResetCaptionResource(void);

        event<Windows::Foundation::TypedEventHandler<Win32Xaml::Window, Windows::Foundation::IInspectable>> m_ev_closed;

        Windows::UI::Xaml::Hosting::DesktopWindowXamlSource m_dwxs;
        com_ptr<IDesktopWindowXamlSourceNative2> m_dwxs_n2;
        HWND m_root_hwnd;
        HWND m_xaml_hwnd;
        HWND m_corewnd_hwnd;
        bool m_is_main{ false };
        bool m_is_frameless{ false };
        bool m_should_remove_title{ false };
        Windows::UI::Xaml::Controls::ContentPresenter m_root_cp;
        event_token m_et_root_cp_actual_theme_changed;
        bool m_last_is_maximized{};
        bool m_is_active{};
        bool m_is_nc_pointer_rpressed{};

        DEVICE_SCALE_FACTOR m_scale_factor;
        unsigned m_dpi;

        com_ptr<IDCompositionDesktopDevice> m_dcomp_dev;
        //com_ptr<IDCompositionTarget> m_dcomp_target_bottom;
        com_ptr<IDCompositionTarget> m_dcomp_target_top;
        com_ptr<IDCompositionVisual2> m_v_top;
        com_ptr<IDCompositionVisual3> m_v_caption_buttons;
        com_ptr<IDCompositionVisual2> m_v_caption_button_minimize;
        com_ptr<IDCompositionVisual2> m_v_caption_button_maximize_restore;
        com_ptr<IDCompositionVisual2> m_v_caption_button_close;
        com_ptr<IDCompositionSurface> m_sf_caption_button_minimize;
        com_ptr<IDCompositionSurface> m_sf_caption_button_maximize_restore;
        com_ptr<IDCompositionSurface> m_sf_caption_button_close;
        CaptionButtonState m_bs_minimize{}, m_bs_maximize_restore{}, m_bs_close{};
        RECT m_rt_caption_button;
        RECT m_rt_btn_minimize, m_rt_btn_maximize_restore, m_rt_btn_close;
        CaptionButtonKind m_cur_pressed_caption_btn{};

        com_ptr<WindowTitleBar> m_title_bar;
        // Cached color values
        // NOTE: cbtn: Caption button
        Windows::UI::Color m_clr_cbtn_none_bkg{ 0xff, 0xff, 0xff, 0xff };
        Windows::UI::Color m_clr_cbtn_none_fore{ 0xff, 0x0, 0x0, 0x0 };
        Windows::UI::Color m_clr_cbtn_hover_bkg{ 0xff, 0xff - 0x19, 0xff - 0x19, 0xff - 0x19 };
        Windows::UI::Color m_clr_cbtn_hover_fore{ 0xff, 0x0, 0x0, 0x0 };
        Windows::UI::Color m_clr_cbtn_pressed_bkg{ 0xff, 0xff - 0x33, 0xff - 0x33, 0xff - 0x33 };
        Windows::UI::Color m_clr_cbtn_pressed_fore{ 0xff, 0x0, 0x0, 0x0 };
        Windows::UI::Color m_clr_cbtn_inactive_bkg{ 0xff, 0xff, 0xff, 0xff };
        Windows::UI::Color m_clr_cbtn_inactive_fore{ 0xff, 0x99, 0x99, 0x99 };

        std::shared_ptr<GdiIconSet[CaptionButtonStateLastIndex]> m_gdi_icon_sets;

        HWND m_input_sink_hwnd{};
        POINT m_input_sink_last_point{};
        ULONGLONG m_input_sink_last_tick{};

#if WIN32XAML_ENABLE_LAYOUT_SYNCHRONIZATION
        std::function<void()> m_fn_commit_dcomp;
#endif
    };

    struct ShellIcon : ShellIconT<ShellIcon> {
        ShellIcon() = default;

        bool IsActive();
        void IsActive(bool value);
        winrt::Windows::Graphics::Imaging::SoftwareBitmap IconImage();
        void IconImage(winrt::Windows::Graphics::Imaging::SoftwareBitmap const& value);
        winrt::Windows::UI::Xaml::Controls::MenuFlyout ContextFlyout();
        void ContextFlyout(winrt::Windows::UI::Xaml::Controls::MenuFlyout const& value);
    };
}

namespace winrt::Win32Xaml::factory_implementation {
    struct AppService : AppServiceT<AppService, implementation::AppService> {};
    struct Window : WindowT<Window, implementation::Window> {};
    struct ShellIcon : ShellIconT<ShellIcon, implementation::ShellIcon> {};
}
