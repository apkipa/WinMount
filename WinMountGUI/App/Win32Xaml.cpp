#include "pch.h"
#include "Win32Xaml.h"
#include "Win32Xaml\AppService.g.cpp"
#include "Win32Xaml\Window.g.cpp"
#include "Win32Xaml\WindowTitleBar.g.cpp"
#include "Win32Xaml\ShellIcon.g.cpp"

#include <Uxtheme.h>
#include <dwmapi.h>
#include <CoreWindow.h>
#include <d3d11.h>
#include <wincodec.h>
#include <UIAutomation.h>
#include <propvarutil.h>
// NOTE: ntdef.h is kernel-mode only, so we must manually pick out stuff we needed
//#include <ntdef.h>

#define NT_SUCCESS(Status) (((NTSTATUS)(Status)) >= 0)
#ifdef _PREFAST_
#define NT_INFORMATION(Status) (((NTSTATUS)(Status)) >= (long)0x40000000)
#else
#define NT_INFORMATION(Status) ((((ULONG)(Status)) >> 30) == 1)
#endif
#ifdef _PREFAST_
#define NT_WARNING(Status) (((NTSTATUS)(Status) < (long)0xc0000000))
#else
#define NT_WARNING(Status) ((((ULONG)(Status)) >> 30) == 2)
#endif
#ifdef _PREFAST_
#define NT_ERROR(Status) (((NTSTATUS)(Status)) >= (unsigned long)0xc0000000)
#else
#define NT_ERROR(Status) ((((ULONG)(Status)) >> 30) == 3)
#endif

#pragma comment(lib, "dwmapi")
#pragma comment(lib, "uxtheme")
#pragma comment(lib, "shcore")
#pragma comment(lib, "dcomp")
#pragma comment(lib, "propsys")

// TODO: Use Win11 style caption buttions for Win11

/* NOTE:
*  Integrating with shell (in fullscreen / compat overlay mode) is almost impossible,
*  because 1) shell hardcodes "ApplicationFrameWindow" class name; 2) shell and UWP
*  host are tightly coupled via bidirectional RPC, and shell holds / regenerates
*  IWinRTApplicationView. The only way to get UWP-like gestures & experiences is to:
*  1) somehow trick shell into creating a stub UWP frame window, then construct our own
*  Windows.UI.Core.CoreWindow and set it via CApplicationFrame::SetPresentedWindow, or
*  2) pretend as the shell to interact with UWP host, then trigger a "crash" in shell to
*  let it load our orphan UWP frame window.
*/

namespace util {
    namespace misc {
        // Similar to std::experimental::scope_exit
        template<typename T>
        struct ScopeExit final {
            explicit ScopeExit(T&& func) : m_func(std::forward<T>(func)) {}
            ~ScopeExit() { if (m_active) { std::invoke(m_func); } }
            void release(void) { m_active = false; }
        private:
            bool m_active{ true };
            T m_func;
        };
        template<typename T>
        inline auto scope_exit(T&& func) {
            return ScopeExit{ std::forward<T>(func) };
        }
    }
}

// Source: https://github.com/ADeltaX/IWindowPrivate
MIDL_INTERFACE("06636C29-5A17-458D-8EA2-2422D997A922")
IWindowPrivate : public IInspectable {
public:
    virtual HRESULT STDMETHODCALLTYPE get_TransparentBackground(_Out_ boolean* value) = 0;
    virtual HRESULT STDMETHODCALLTYPE put_TransparentBackground(_In_ boolean value) = 0;
};

struct STUB_DO_NOT_USE {};

namespace Win32Xaml {
    namespace sys_interface {
        namespace v10_0_19045 {
            // IDCompositionDesktopDevicePartner6 (Win10 22H2 19045.3086)
            // NOTE: QI from Windows.UI.Composition.Compositor or Windows.UI.Composition.IInteropCompositorPartner
            DECLARE_INTERFACE_IID_(IDCompositionDesktopDevicePartner6, ::IUnknown, "CB139649-6D80-48E7-B54D-09737D84DB47") {
                STDMETHOD(Commit)(THIS) PURE;
                STDMETHOD(WaitForCommitCompletion)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(GetFrameStatistics)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateVisual)(THIS_
                    IDCompositionVisual **visual) PURE;
                STDMETHOD(CreateSurfaceFactory)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateSurface)(THIS_
                    UINT width,
                    UINT height,
                    DXGI_FORMAT pixelFormat,
                    DXGI_ALPHA_MODE alphaMode,
                    IDCompositionSurface **surface) PURE;
                STDMETHOD(CreateVirtualSurface)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateTranslateTransform)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateScaleTransform)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateRotateTransform)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateSkewTransform)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateMatrixTransform)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateTransformGroup)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateTranslateTransform3D)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateScaleTransform3D)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateRotateTransform3D)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateMatrixTransform3D)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateTransform3DGroup)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateEffectGroup)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateRectangleClip)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateAnimation)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateTargetForHwnd)(THIS_
                    HWND hwnd,
                    BOOL topmost,
                    IDCompositionTarget **target) PURE;
                STDMETHOD(CreateSurfaceFromHandle)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateSurfaceFromHwnd)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateSharedResource)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(OpenSharedResourceHandle)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(OpenSharedResource)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateAnimationTrigger)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateGradientSurface)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(EnableWhitePixelOptimization)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(OfferSurfaceResources)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(ReclaimSurfaceResources)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(DisableD2DStatePreservation)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(Flush)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateSurfaceFromVisualSnapshot)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateInteraction)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateTouchInteractionConfiguration)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateTouchpadInteractionConfiguration)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreatePenInteractionConfiguration)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateMouseInteractionConfiguration)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(OpenSharedResource)(THIS_
                    STUB_DO_NOT_USE, STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateDesktopTree)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(GetCurrentBatchID)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(GetLastConfirmedBatchId)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateGaussianBlurEffect)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateManipulationTransform)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateBrightnessEffect)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateColorMatrixEffect)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateShadowEffect)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateHueRotationEffect)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateSaturationEffect)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateTurbulenceEffect)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateLinearTransferEffect)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateTableTransferEffect)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(RegisterCallbackThread)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateCompositeEffect)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(SnapAnimationReadBackTime)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(GetMaxTextureSize)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateBlendEffect)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateArithmeticCompositeEffect)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateAffineTransform2DEffect)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(SetCompositionPrimitiveGroupRendererEnabled)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateRenderTarget)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateDDisplayRenderTarget)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(SynchronizedCommit)(THIS_
                    HANDLE hObject) PURE;
                STDMETHOD(CreateMousewheelInteractionConfiguration)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateVisualReferenceController)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateSynchronousSuperWetInkProxy)(THIS_
                    STUB_DO_NOT_USE) PURE;
            };
        }

        namespace v10_0_22621 {
            // IDCompositionDesktopDevicePartner6 (Win11 22H2 22621.1848)
            // NOTE: QI from Windows.UI.Composition.Compositor or Windows.UI.Composition.IInteropCompositorPartner
            DECLARE_INTERFACE_IID_(IDCompositionDesktopDevicePartner6, ::IUnknown, "E01EB649-787E-4560-B398-0DE7A2065D8B") {
                STDMETHOD(Commit)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(WaitForCommitCompletion)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(GetFrameStatistics)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateVisual)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateSurfaceFactory)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateSurface)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateVirtualSurface)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateTranslateTransform)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateScaleTransform)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateRotateTransform)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateSkewTransform)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateMatrixTransform)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateTransformGroup)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateTranslateTransform3D)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateScaleTransform3D)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateRotateTransform3D)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateMatrixTransform3D)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateTransform3DGroup)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateEffectGroup)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateRectangleClip)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateAnimation)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateTargetForHwnd)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateSurfaceFromHandle)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateSurfaceFromHwnd)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateSharedResource)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(OpenSharedResourceHandle)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(OpenSharedResource)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateAnimationTrigger)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateGradientSurface)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(EnableWhitePixelOptimization)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(OfferSurfaceResources)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(ReclaimSurfaceResources)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(DisableD2DStatePreservation)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(Flush)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateSurfaceFromVisualSnapshot)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateInteraction)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateTouchInteractionConfiguration)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateTouchpadInteractionConfiguration)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreatePenInteractionConfiguration)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateMouseInteractionConfiguration)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(OpenSharedResource)(THIS_
                    STUB_DO_NOT_USE, STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateDesktopTree)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(GetCurrentBatchID)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(GetLastConfirmedBatchId)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateGaussianBlurEffect)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateManipulationTransform)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateBrightnessEffect)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateColorMatrixEffect)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateShadowEffect)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateHueRotationEffect)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateSaturationEffect)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateTurbulenceEffect)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateLinearTransferEffect)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateTableTransferEffect)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(RegisterCallbackThread)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateCompositeEffect)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(SnapAnimationReadBackTime)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(GetMaxTextureSize)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateBlendEffect)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateArithmeticCompositeEffect)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateAffineTransform2DEffect)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(SetCompositionPrimitiveGroupRendererEnabled)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateRenderTarget)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateDDisplayRenderTarget)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateCursorVisual)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateMousewheelInteractionConfiguration)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(SynchronizedCommit)(THIS_
                    HANDLE hObject) PURE;
                STDMETHOD(CreateVisualReferenceController)(THIS_
                    STUB_DO_NOT_USE) PURE;
                STDMETHOD(CreateWindowTarget)(THIS_
                    STUB_DO_NOT_USE) PURE;
            };
        }
    };
}

// Windows.UI.Xaml.Hosting.IXamlIsland
DECLARE_INTERFACE_IID_(IXamlIsland, ::IInspectable, "412B49D7-B8B7-416A-B49B-57F9EDBEF991") {
    STDMETHOD(Stub1)(THIS_
        STUB_DO_NOT_USE) PURE;
};

// Windows.UI.Xaml.IFrameworkApplicationPrivate (Win10 22H2 19045.3086)
// NOTE: QI from Windows.UI.Xaml.Application
DECLARE_INTERFACE_IID_(IFrameworkApplicationPrivate, ::IInspectable, "B3AB45D8-6A4E-4E76-A00D-32D4643A9F1A") {
    STDMETHOD(StartOnCurrentThread)(THIS_
        ABI::Windows::UI::Xaml::IApplicationInitializationCallback *pCallback) PURE;
    STDMETHOD(CreateIsland)(THIS_
        IXamlIsland **ppResult) PURE;
    STDMETHOD(CreateIslandWithAppWindow)(THIS_
        ::IInspectable *pAppWindow,
        IXamlIsland **ppResult) PURE;
    STDMETHOD(CreateIslandWithContentBridge)(THIS_
        ::IInspectable *pOwner,
        ::IInspectable *pContentBridge,
        IXamlIsland **ppResult) PURE;
    STDMETHOD(RemoveIsland)(THIS_
        IXamlIsland *pIsland) PURE;
    STDMETHOD(SetSynchronizationWindow)(THIS_
        HWND commitResizeWindow) PURE;
};

// IInteropCompositor* source: https://blog.adeltax.com/interopcompositor-and-coredispatcher/
// Windows.UI.Composition.IInteropCompositorPartner
DECLARE_INTERFACE_IID_(IInteropCompositorPartner, IUnknown, "e7894c70-af56-4f52-b382-4b3cd263dc6f") {
    STDMETHOD(MarkDirty)(THIS) PURE;
    STDMETHOD(ClearCallback)(THIS) PURE;
    STDMETHOD(CreateManipulationTransform)(THIS_
        IN struct IDCompositionTransform* transform,
        IN REFIID iid,
        OUT VOID **result) PURE;
    STDMETHOD(RealClose)(THIS) PURE;
};
// Windows.UI.Composition.IInteropCompositorPartnerCallback
DECLARE_INTERFACE_IID_(IInteropCompositorPartnerCallback, IUnknown, "9bb59fc9-3326-4c32-bf06-d6b415ac2bc5") {
    STDMETHOD(NotifyDirty)(THIS) PURE;
    STDMETHOD(NotifyDeferralState)(THIS_
        bool deferRequested) PURE;
};
// Windows.UI.Composition.IInteropCompositorFactoryPartner
DECLARE_INTERFACE_IID_(IInteropCompositorFactoryPartner, IInspectable, "22118adf-23f1-4801-bcfa-66cbf48cc51b") {
    STDMETHOD(CreateInteropCompositor)(THIS_
        IN IUnknown *renderingDevice,
        IN IInteropCompositorPartnerCallback *callback,
        IN REFIID iid,
        OUT VOID **instance) PURE;
    STDMETHOD(CheckEnabled)(THIS_
        OUT bool* enableInteropCompositor,
        OUT bool* enableExposeVisual) PURE;
};

// IDCompositionVisualPartner
// NOTE: QI from Windows.UI.Composition.Visual
DECLARE_INTERFACE_IID_(IDCompositionVisualPartner, ::IUnknown, "2C4EEF28-1BC0-4736-B7DD-B62692F9BD67") {
    STDMETHOD(Stub_DoNotUse1)(THIS_
        STUB_DO_NOT_USE) PURE;
};

// Windows.ApplicationModel.Core(.???).ICoreApplicationViewTitleBarInternal
// NOTE: QI from Windows.ApplicationModel.Core.CoreApplicationViewTitleBar
DECLARE_INTERFACE_IID_(ICoreApplicationViewTitleBarInternal, ::IInspectable, "6E470D39-E79F-453C-92EF-6310EDEFD440") {
    STDMETHOD(OnVisibilityChanged)(THIS_
        STUB_DO_NOT_USE) PURE;
    STDMETHOD(OnLayoutMetricsChanged)(THIS_
        STUB_DO_NOT_USE) PURE;
    STDMETHOD(SetInputSinkWindow)(THIS_
        HWND hWnd) PURE;
    // TODO: SetTitleBarVisual accepts IDCompositionVisualPartner in reality
    STDMETHOD(SetTitleBarVisual)(THIS_
        IN IUnknown *ptr) PURE;
    STDMETHOD(HasTitleBarVisual)(THIS_
        OUT BOOL *value) PURE;
    STDMETHOD(NotifyPersistedValues)(THIS_
        STUB_DO_NOT_USE) PURE;
    STDMETHOD(get_DesiredTitlebarOverlayState)(THIS_
        STUB_DO_NOT_USE) PURE;
    STDMETHOD(put_DesiredTitlebarOverlayState)(THIS_
        STUB_DO_NOT_USE) PURE;
};

/*
// NOTE: Use QueryWindowService to get an instance
DECLARE_INTERFACE_IID_(IWindowServiceProxy, ::IUnknown, "CDAE3790-E890-4803-B2CF-0BAEEEDBFAF5") {
    STDMETHOD(GetTargetWindow)(THIS_ HWND*) PURE;
};
*/

struct bitmap_handle_traits {
    using type = HBITMAP;
    static void close(type value) noexcept {
        WINRT_VERIFY_(1, DeleteObject(value));
    }
    static type invalid() noexcept {
        return nullptr;
    }
};
using bitmap_handle = winrt::handle_type<bitmap_handle_traits>;

auto brush_from_element_theme(winrt::Windows::UI::Xaml::ElementTheme et) {
    using winrt::Windows::UI::Xaml::ElementTheme;
    using winrt::Windows::UI::Colors;
    using namespace winrt::Windows::UI::Xaml::Media;
    if (et == ElementTheme::Dark) {
        return SolidColorBrush(Colors::Black());
    }
    else {
        return SolidColorBrush(Colors::White());
    }
}

HBITMAP create_dib_from_32bpp_wic_bitmap(IWICBitmapSource* bmp, void** out_pixels) {
    WICPixelFormatGUID pixel_fmt;
    winrt::check_hresult(bmp->GetPixelFormat(&pixel_fmt));
    if (pixel_fmt != GUID_WICPixelFormat32bppBGRA) {
        throw winrt::hresult_error(E_FAIL, L"Source bitmap pixel format is not 32bppBGRA");
    }
    UINT width, height;
    winrt::check_hresult(bmp->GetSize(&width, &height));

    void* image_bits;
    if (!out_pixels) { out_pixels = &image_bits; }
    BITMAPINFO bminfo{};
    bminfo.bmiHeader.biSize = sizeof(BITMAPINFOHEADER);
    bminfo.bmiHeader.biWidth = width;
    bminfo.bmiHeader.biHeight = -static_cast<LONG>(height);
    bminfo.bmiHeader.biPlanes = 1;
    bminfo.bmiHeader.biBitCount = 32;
    bminfo.bmiHeader.biCompression = BI_RGB;
    auto hbmp = CreateDIBSection(nullptr, &bminfo, DIB_RGB_COLORS, out_pixels, NULL, 0);
    winrt::check_pointer(hbmp);
    auto se_hbmp = util::misc::scope_exit([&] {
        DeleteObject(hbmp);
    });

    UINT stride, total_size;
    winrt::check_hresult(UIntMult(4, width, &stride));
    winrt::check_hresult(UIntMult(stride, height, &total_size));

    winrt::check_hresult(bmp->CopyPixels(nullptr, stride, total_size,
        reinterpret_cast<BYTE*>(*out_pixels)));

    se_hbmp.release();
    return hbmp;
}

constexpr POINT points_to_point(POINTS const& pts) {
    return { pts.x, pts.y };
}

constexpr POINT lparam_to_point(LPARAM lParam) {
    return points_to_point(std::bit_cast<POINTS>(static_cast<uint32_t>(lParam)));
}

auto get_resize_frame_vertical_for_dpi(unsigned dpi) {
    return GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi) +
        GetSystemMetricsForDpi(SM_CYSIZEFRAME, dpi);
}
auto get_resize_frame_horizontal_for_dpi(unsigned dpi) {
    return GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi) +
        GetSystemMetricsForDpi(SM_CXSIZEFRAME, dpi);
}

auto MakeDCompSurfBDCompatShim(IDCompositionSurface* dcompSurface) {
    return [=](RECT const* updateRect, POINT* updateOffset, IID const& iid, void** updateObject) {
        return dcompSurface->BeginDraw(updateRect, iid, updateObject, updateOffset);
    };
}

void populate_1x1_bgra_premul_dcomp_surface(IDCompositionSurface* dcomp_surface, winrt::Windows::UI::Color color) {
    POINT update_offset;
    winrt::com_ptr<ID3D11Texture2D> d3d11_texture2d;
    d3d11_texture2d.capture(MakeDCompSurfBDCompatShim(dcomp_surface), nullptr, &update_offset);
    D3D11_BOX dest_box;
    dest_box.front = 0;
    dest_box.back = 1;
    dest_box.left = update_offset.x;
    dest_box.top = update_offset.y;
    dest_box.right = dest_box.left + 1;
    dest_box.bottom = dest_box.top + 1;
    winrt::com_ptr<ID3D11Device> d3d11_dev;
    d3d11_texture2d->GetDevice(d3d11_dev.put());
    winrt::com_ptr<ID3D11DeviceContext> d3d11_dev_ctx;
    d3d11_dev->GetImmediateContext(d3d11_dev_ctx.put());
    uint8_t pixel[4]{
        static_cast<uint8_t>(MulDiv(color.B, color.A, 255)),
        static_cast<uint8_t>(MulDiv(color.G, color.A, 255)),
        static_cast<uint8_t>(MulDiv(color.R, color.A, 255)),
        color.A
    };
    d3d11_dev_ctx->UpdateSubresource(d3d11_texture2d.get(), 0, &dest_box, &pixel, 4, 4);
    winrt::check_hresult(dcomp_surface->EndDraw());
}

void fill_rect_with_color_premul(HDC hdc, RECT const& rt, winrt::Windows::UI::Color color) {
    uint8_t pixel[4]{
        static_cast<uint8_t>(MulDiv(color.B, color.A, 255)),
        static_cast<uint8_t>(MulDiv(color.G, color.A, 255)),
        static_cast<uint8_t>(MulDiv(color.R, color.A, 255)),
        color.A
    };
    BITMAPINFO bi{};
    bi.bmiHeader.biSize = sizeof(BITMAPINFOHEADER);
    bi.bmiHeader.biWidth = 1;
    bi.bmiHeader.biHeight = 1;
    bi.bmiHeader.biPlanes = 1;
    bi.bmiHeader.biBitCount = 32;
    bi.bmiHeader.biCompression = BI_RGB;
    StretchDIBits(hdc, rt.left, rt.top, rt.right - rt.left, rt.bottom - rt.top,
        0, 0, 1, 1, pixel, &bi, DIB_RGB_COLORS, SRCCOPY);
}

void track_and_exec_sys_menu_for_window(HWND hwnd, POINT pt_screen, bool is_bidi_locale) {
    const bool is_maximized = IsZoomed(hwnd);
    auto sys_menu = GetSystemMenu(hwnd, false);
    constexpr auto DISABLED_MENU_STYLE = MF_DISABLED | MF_GRAYED;
    EnableMenuItem(sys_menu, SC_RESTORE, is_maximized ? 0 : DISABLED_MENU_STYLE);
    EnableMenuItem(sys_menu, SC_MAXIMIZE, is_maximized ? DISABLED_MENU_STYLE : 0);
    EnableMenuItem(sys_menu, SC_MOVE, is_maximized ? DISABLED_MENU_STYLE : 0);
    EnableMenuItem(sys_menu, SC_SIZE, is_maximized ? DISABLED_MENU_STYLE : 0);
    SetMenuDefaultItem(sys_menu, SC_CLOSE, false);
    auto ret = TrackPopupMenuEx(
        sys_menu,
        TPM_RETURNCMD | TPM_NONOTIFY | (is_bidi_locale ? TPM_LAYOUTRTL | TPM_RIGHTALIGN : 0),
        pt_screen.x, pt_screen.y,
        hwnd,
        nullptr
    );
    if (ret) { SendMessageW(hwnd, WM_SYSCOMMAND, ret, 0); }
}

template<typename T>
T value_or(winrt::Windows::Foundation::IReference<T> const& v, T or_default) {
    return v ? v.Value() : std::move(or_default);
}

static constexpr wchar_t g_win32xaml_class_name[] = L"XamlHostWindowClass";
static constexpr wchar_t g_win32xaml_input_sink_class_name[] = L"XamlHostInputSinkWindowClass";
static HINSTANCE g_hinst;

namespace Win32Xaml {
    namespace dyn_proc {
        enum PreferredAppMode {
            Default,
            AllowDark,
            ForceDark,
            ForceLight,
            Max
        };

#define Win32Xaml_dyn_proc_DefineEntry(name, type)          \
    using t ## name = type;                                 \
    namespace details { t ## name name; }                   \
    auto const& name = details::name
#define Win32Xaml_dyn_proc_AssignEntry(name, addr)                                                  \
    ::Win32Xaml::dyn_proc::details::name = reinterpret_cast<::Win32Xaml::dyn_proc::t ## name>(      \
        addr)
#define Win32Xaml_dyn_proc_AssignEntry_GPA(name, module, entry)             \
    Win32Xaml_dyn_proc_AssignEntry(name, ::GetProcAddress(module, entry))

#if WIN32XAML_ENABLE_LAYOUT_SYNCHRONIZATION
        // TODO: Does EnableResizeLayoutSynchronization really return void?
        Win32Xaml_dyn_proc_DefineEntry(EnableResizeLayoutSynchronization,
            void(WINAPI*)(HWND hwnd, bool enable));
        Win32Xaml_dyn_proc_DefineEntry(GetResizeDCompositionSynchronizationObject,
            void(WINAPI*)(HWND hwnd, LPHANDLE pHandle));
#endif
        Win32Xaml_dyn_proc_DefineEntry(SetPreferredAppMode,
            PreferredAppMode(WINAPI*)(PreferredAppMode appMode));
        Win32Xaml_dyn_proc_DefineEntry(SHCreateStreamOnModuleResourceW,
            HRESULT(WINAPI*)(HMODULE hModule, LPCWSTR pwszName, LPCWSTR pwszType, IStream** ppStream));
        /*
        Win32Xaml_dyn_proc_DefineEntry(RegisterWindowService,
            HRESULT(WINAPI*)(HWND hwnd, REFIID iid, void* ptr));
        // TODO: Should also take care of HresultFromKnownLastError
        Win32Xaml_dyn_proc_DefineEntry(SetModernAppWindow,
            BOOL(WINAPI*)(HWND hwnd_frame, HWND hwnd_app));
        Win32Xaml_dyn_proc_DefineEntry(GetModernAppWindow,
            HWND(WINAPI*)(HWND hwnd));
        */
    }

    winrt::com_ptr<IWICImagingFactory> g_wic_factory;
}

struct OSVersion {
    uint32_t major, minor, patch;

    auto operator<=>(OSVersion const& rhs) const noexcept = default;

    static OSVersion get_win11_21h2() noexcept { return { 10, 0, 22000 }; }

    bool is_win11_or_newer() const noexcept { return *this >= get_win11_21h2(); }
};
OSVersion get_os_version(void) {
    static auto version = []() -> OSVersion {
        auto ntdll = GetModuleHandleW(L"ntdll.dll");
        NTSTATUS(WINAPI* RtlGetVersion)(PRTL_OSVERSIONINFOEXW ptr);
        RtlGetVersion = reinterpret_cast<decltype(RtlGetVersion)>(
            GetProcAddress(ntdll, "RtlGetVersion"));
        if (!RtlGetVersion) { return {}; }
        OSVERSIONINFOEXW os_ver;
        os_ver.dwOSVersionInfoSize = sizeof os_ver;
        if (!NT_SUCCESS(RtlGetVersion(&os_ver))) { return {}; }
        return { os_ver.dwMajorVersion, os_ver.dwMinorVersion, os_ver.dwBuildNumber };
    }();
    return version;
}

using namespace Win32Xaml::dyn_proc;
using Win32Xaml::g_wic_factory;

void InitializeWin32Xaml(HINSTANCE hInstance) {
    if (g_hinst) { return; }
    if (!hInstance) { throw winrt::hresult_invalid_argument(L"Invalid hInstance"); }
    g_hinst = hInstance;

    auto load_dyn_procs_fn = [] {
        // WARN: If dll is not loaded, it will load and **leak** the dll
        auto get_dll_fn = [](const wchar_t* name) {
            auto dll = GetModuleHandleW(name);
            if (!dll) { dll = LoadLibraryW(name); }
            return dll;
        };
        auto mod_uxtheme = get_dll_fn(L"uxtheme.dll");
        auto mod_user32 = get_dll_fn(L"user32.dll");
        auto mod_shcore = get_dll_fn(L"shcore.dll");
        //auto mod_twinapi = get_dll_fn(L"twinapi.dll");
#if WIN32XAML_ENABLE_LAYOUT_SYNCHRONIZATION
        Win32Xaml_dyn_proc_AssignEntry_GPA(EnableResizeLayoutSynchronization,
            mod_user32, MAKEINTRESOURCEA(2615));
        Win32Xaml_dyn_proc_AssignEntry_GPA(GetResizeDCompositionSynchronizationObject,
            mod_user32, MAKEINTRESOURCEA(2614));
#endif
        Win32Xaml_dyn_proc_AssignEntry_GPA(SetPreferredAppMode,
            mod_uxtheme, MAKEINTRESOURCEA(135));
        Win32Xaml_dyn_proc_AssignEntry_GPA(SHCreateStreamOnModuleResourceW,
            mod_shcore, MAKEINTRESOURCEA(109));
        /*
        Win32Xaml_dyn_proc_AssignEntry_GPA(RegisterWindowService,
            mod_twinapi, MAKEINTRESOURCEA(10));
        Win32Xaml_dyn_proc_AssignEntry_GPA(SetModernAppWindow,
            mod_user32, MAKEINTRESOURCEA(2568));
        Win32Xaml_dyn_proc_AssignEntry_GPA(GetModernAppWindow,
            mod_user32, MAKEINTRESOURCEA(2569));
        */

        bool ok = true;
#if WIN32XAML_ENABLE_LAYOUT_SYNCHRONIZATION
        ok = ok && EnableResizeLayoutSynchronization;
        ok = ok && GetResizeDCompositionSynchronizationObject;
#endif
        ok = ok && SetPreferredAppMode;
        ok = ok && SHCreateStreamOnModuleResourceW;
        /*
        ok = ok && RegisterWindowService;
        ok = ok && SetModernAppWindow;
        ok = ok && GetModernAppWindow;
        */
        if (!ok) {
            throw winrt::hresult_error(E_FAIL, L"Could not resolve all required dynamic procedures");
        }
    };
    load_dyn_procs_fn();

    g_wic_factory = winrt::create_instance<IWICImagingFactory>(CLSID_WICImagingFactory);

    WNDCLASSW wc{};
    // Main window
    wc.style = 0;
    wc.lpfnWndProc = [](HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam) -> LRESULT {
        using ::winrt::Win32Xaml::implementation::Window;

        // TODO: Capture exceptions

        // TODO: Dark mode

        if (msg == WM_CREATE) {
            void* copied_this = reinterpret_cast<LPCREATESTRUCTW>(lParam)->lpCreateParams;
            SetWindowLongPtrW(hwnd, 0, reinterpret_cast<LONG_PTR>(copied_this));
            return 0;
        }
        auto* copied_this = reinterpret_cast<Window*>(GetWindowLongPtrW(hwnd, 0));
        if (copied_this == nullptr) {
            return DefWindowProcW(hwnd, msg, wParam, lParam);
        }
        if (msg == WM_CLOSE) {
            copied_this->Close();
            return 0;
        }

        /*if (copied_this->m_imp->window_theme == WindowTheme::FollowSystem) {
            if (util::win32::dark_mode::is_color_scheme_change_message(msg, lParam)) {
                util::win32::dark_mode::update_title_bar_theme_color(hwnd);
                return 0;
            }
        }*/

        return copied_this->WindowProc(hwnd, msg, wParam, lParam);
    };
    wc.cbClsExtra = 0;
    wc.cbWndExtra = sizeof(void*);
    wc.hInstance = g_hinst;
    wc.hIcon = nullptr;
    wc.hCursor = LoadCursor(nullptr, IDC_ARROW);
    wc.lpszMenuName = nullptr;
    wc.lpszClassName = g_win32xaml_class_name;
    if (RegisterClassW(&wc) == 0) { winrt::throw_last_error(); }

    // Input sink window
    wc.lpfnWndProc = [](HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam) -> LRESULT {
        using ::winrt::Win32Xaml::implementation::Window;

        // TODO: Capture exceptions

        if (msg == WM_CREATE) {
            void* copied_this = reinterpret_cast<LPCREATESTRUCTW>(lParam)->lpCreateParams;
            SetWindowLongPtrW(hwnd, 0, reinterpret_cast<LONG_PTR>(copied_this));
            return 0;
        }
        auto* copied_this = reinterpret_cast<Window*>(GetWindowLongPtrW(hwnd, 0));
        if (copied_this == nullptr) {
            return DefWindowProcW(hwnd, msg, wParam, lParam);
        }

        return copied_this->InputSinkWindowProc(hwnd, msg, wParam, lParam);
    };
    wc.lpszClassName = g_win32xaml_input_sink_class_name;
    if (RegisterClassW(&wc) == 0) { winrt::throw_last_error(); }
}

namespace winrt::Win32Xaml::implementation {
    using namespace Windows::Foundation;

    // A set of HBITMAPs for a specific DPI
    struct GdiIconSet {
        DEVICE_SCALE_FACTOR scale_factor{ DEVICE_SCALE_FACTOR_INVALID };
        DEVICE_SCALE_FACTOR real_scale_factor{ DEVICE_SCALE_FACTOR_INVALID };
        bitmap_handle bmp_minimize;
        bitmap_handle bmp_close;
        bitmap_handle bmp_maximize;
        bitmap_handle bmp_restore;
        // Additional icons omitted

        // Loads images from system UWP component
        static GdiIconSet load_colored(DEVICE_SCALE_FACTOR scale_factor,
            Windows::UI::Color fore_color, Windows::UI::Color close_fore_color
        ) {
            static constexpr DEVICE_SCALE_FACTOR scale_factors_list[]{
                SCALE_100_PERCENT, SCALE_125_PERCENT, SCALE_150_PERCENT, SCALE_200_PERCENT,
                SCALE_250_PERCENT, SCALE_300_PERCENT, SCALE_400_PERCENT,
            };

            GdiIconSet icons;
            size_t sf_index = std::size(scale_factors_list);
            // Find last one satisfying x >= elem
            while (--sf_index > 0 && scale_factor < scale_factors_list[sf_index]);
            icons.scale_factor = scale_factor;
            icons.real_scale_factor = scale_factors_list[sf_index];

            icons.bmp_minimize = load_one_colored_by_name(MAKEINTRESOURCEW(9635 + sf_index), fore_color);
            icons.bmp_close = load_one_colored_by_name(MAKEINTRESOURCEW(9645 + sf_index), close_fore_color);
            icons.bmp_maximize = load_one_colored_by_name(MAKEINTRESOURCEW(9655 + sf_index), fore_color);
            icons.bmp_restore = load_one_colored_by_name(MAKEINTRESOURCEW(9665 + sf_index), fore_color);

            return icons;
        }
        // Loads a image, then applies colors and premultiplies
        static bitmap_handle load_one_colored_by_name(WCHAR const* name, Windows::UI::Color fore_color) {
            static auto mod_appframe = LoadLibraryExW(L"ApplicationFrame.dll", nullptr, LOAD_LIBRARY_AS_DATAFILE);
            if (!mod_appframe) {
                throw hresult_error(E_FAIL, L"Could not find ApplicationFrame.dll, "
                    "which is required for loading icons");
            }
            com_ptr<IStream> stm;
            check_hresult(SHCreateStreamOnModuleResourceW(mod_appframe, name, L"IMAGE", stm.put()));
            com_ptr<IWICBitmapDecoder> decoder;
            com_ptr<IWICBitmapFrameDecode> frame_src;
            com_ptr<IWICFormatConverter> converter;
            check_hresult(g_wic_factory->CreateDecoderFromStream(stm.get(), &GUID_VendorMicrosoftBuiltIn,
                WICDecodeMetadataCacheOnLoad, decoder.put()));
            check_hresult(decoder->GetFrame(0, frame_src.put()));
            check_hresult(g_wic_factory->CreateFormatConverter(converter.put()));
            check_hresult(converter->Initialize(
                frame_src.get(),
                GUID_WICPixelFormat32bppBGRA,
                WICBitmapDitherTypeNone,
                nullptr,
                0,
                WICBitmapPaletteTypeMedianCut
            ));
            UINT width, height;
            void* pixels_void;
            bitmap_handle bmp{ create_dib_from_32bpp_wic_bitmap(converter.get(), &pixels_void) };
            check_hresult(converter->GetSize(&width, &height));
            // NOTE: Foreground color always replaces pixels with RGB = 0xffffff
            size_t total_size = 4ull * width * height;
            auto pixels = reinterpret_cast<uint8_t*>(pixels_void);
            for (size_t i = 0; i < total_size; i += 4) {
                auto cur_pix = &pixels[i];
                auto pix_a = cur_pix[3];
                // Replace
                if (cur_pix[0] == 0xff && cur_pix[1] == 0xff && cur_pix[2] == 0xff) {
                    cur_pix[0] = fore_color.B;
                    cur_pix[1] = fore_color.G;
                    cur_pix[2] = fore_color.R;
                }
                // Premultiply
                cur_pix[0] = static_cast<uint8_t>(MulDiv(cur_pix[0], pix_a, 255));
                cur_pix[1] = static_cast<uint8_t>(MulDiv(cur_pix[1], pix_a, 255));
                cur_pix[2] = static_cast<uint8_t>(MulDiv(cur_pix[2], pix_a, 255));
            }
            return bmp;
        }
    };

    constexpr auto CAPTION_BUTTON_WIDTH = 46;
    constexpr auto CAPTION_BUTTON_HEIGHT = 32;

    thread_local bool t_should_quit = false;
    thread_local com_ptr<Window> t_main_window;
    thread_local std::vector<com_ptr<Window>> t_windows;
    bool AppService::AutoQuit() {
        throw hresult_not_implemented();
    }
    void AppService::AutoQuit(bool value) {
        throw hresult_not_implemented();
    }
    void AppService::Exit() {
        t_should_quit = true;
    }
    void AppService::RunLoop() {
        MSG msg;
        t_should_quit = t_windows.empty();
        while (!t_should_quit) {
            auto ret = GetMessageW(&msg, nullptr, 0, 0);
            if (ret == -1) { throw_last_error(); }
            // Workaround XAML Islands Alt+F4 bug, see
            // https://github.com/microsoft/microsoft-ui-xaml/issues/2408
            if (msg.message == WM_SYSKEYDOWN && msg.wParam == VK_F4) [[unlikely]] {
                SendMessage(GetAncestor(msg.hwnd, GA_ROOT), msg.message, msg.wParam, msg.lParam);
                continue;
            }
            BOOL handled{};
            for (auto const& i : t_windows) {
                check_hresult(i->m_dwxs_n2->PreTranslateMessage(&msg, &handled));
                if (handled) { break; }
            }
            if (!handled) {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            t_should_quit |= t_windows.empty();
        }
        while (!t_windows.empty()) {
            t_windows.front()->Close();
        }
#if WIN32XAML_ENABLE_SAFE_TEARDOWN
        // NOTE: System XAML framework uses DispatchTimer (?) to dispose some XAML resources, which
        //       means some (user-defined) destructors are not guaranteed to run on application
        //       exit. If the application (unfortunately) relies on destructors being called in the
        //       correct order, enable safe teardown below.
        // WARN: Enabling safe teardown may add seconds of delay to the exit process
        {
            using namespace Windows::System;
            auto dq = DispatcherQueue::GetForCurrentThread();
            bool real_quit{ false };
            dq.TryEnqueue(DispatcherQueuePriority::Low, [&] { real_quit = true; });
            do {
                auto ret = GetMessageW(&msg, nullptr, 0, 0);
                if (ret == -1) { throw_last_error(); }
                DispatchMessageW(&msg);
            } while (!real_quit);
        }
#endif
    }

    Window::Window() : m_gdi_icon_sets(std::make_shared<GdiIconSet[CaptionButtonStateLastIndex]>()),
        m_title_bar(make_self<WindowTitleBar>())
    {
        using namespace Windows::UI::Xaml::Hosting;
        // TODO: What will happen if window is destroyed before it is activated?
        m_is_main = t_windows.empty();
        t_windows.push_back(get_strong());
        auto scope_windows = util::misc::scope_exit([&] {
            t_windows.pop_back();
        });
        m_root_hwnd = CreateWindowExW(
            WS_EX_NOREDIRECTIONBITMAP,
            g_win32xaml_class_name,
            L"Xaml Window",
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT, CW_USEDEFAULT,
            CW_USEDEFAULT, CW_USEDEFAULT,
            nullptr,
            nullptr,
            g_hinst,
            this
        );
        check_pointer(m_root_hwnd);
        auto scope_root_hwnd = util::misc::scope_exit([&] {
            DestroyWindow(m_root_hwnd);
        });
#if WIN32XAML_ENABLE_LAYOUT_SYNCHRONIZATION
        // TODO: Not working with secondary windows; figure out why
        // Tell OS that we want to participate in synchronization with a custom layout system
        // (XAML Islands in this case)
        EnableResizeLayoutSynchronization(m_root_hwnd, true);

        //Windows::UI::Core::CoreWindowResizeManager::GetForCurrentView().ShouldWaitForLayoutCompletion(true);
#endif
        this->InitializeDComp();
        this->UpdateCaptionVisibility(false);
        this->CommitDComp();

        // NOTE: Hide CoreWindow beforehand to avoid focus issues
        auto core_wnd = Windows::UI::Core::CoreWindow::GetForCurrentThread();
        check_hresult(core_wnd.as<ICoreWindowInterop>()->get_WindowHandle(&m_corewnd_hwnd));
        if (m_is_main) {
            t_main_window = get_strong();
            // Prevent DesktopWindowXamlSource from appearing in taskbar on Windows 10
            ShowWindow(m_corewnd_hwnd, SW_HIDE);
            // Remove XAML emergency background
            auto wp = Windows::UI::Xaml::Window::Current().as<IWindowPrivate>();
            check_hresult(wp->put_TransparentBackground(true));
        }
        // Use our own emergency background
        this->UseTransparentBackground(false);

#if WIN32XAML_FIX_ACRYLIC_FIRST_ACTIVATION
        // Fix acrylic brush not working on first activation
        // NOTE: We need to ensure window is in activated state when system
        //       samples window focus state during XAML Islands creation
        ShowWindow(m_root_hwnd, SW_SHOW);
#endif

        m_dwxs_n2 = m_dwxs.as<IDesktopWindowXamlSourceNative2>();
        check_hresult(m_dwxs_n2->AttachToWindow(m_root_hwnd));
        check_hresult(m_dwxs_n2->get_WindowHandle(&m_xaml_hwnd));
#if 0
        m_dwxs.TakeFocusRequested(
            // TODO: TakeFocusRequested misfires in NavigationView; figure out the solution
            [](DesktopWindowXamlSource const& sender, DesktopWindowXamlSourceTakeFocusRequestedEventArgs const& e) {
                auto reason = e.Request().Reason();
                if (reason == XamlSourceFocusNavigationReason::First ||
                    reason == XamlSourceFocusNavigationReason::Last)
                {
                    sender.NavigateFocus(e.Request());
                }
            }
        );
#else
        m_root_cp.TabFocusNavigation(Windows::UI::Xaml::Input::KeyboardNavigationMode::Cycle);
#endif
        /*
        {
            struct WindowServiceProxy : implements<WindowServiceProxy, IWindowServiceProxy> {
                WindowServiceProxy(HWND hwnd) : m_hwnd(hwnd) {}
                HRESULT GetTargetWindow(HWND* out_hwnd) noexcept {
                    *out_hwnd = m_hwnd;
                    return S_OK;
                }
            private:
                HWND m_hwnd;
            };
            auto wsp = make<WindowServiceProxy>(m_corewnd_hwnd);
            check_hresult(RegisterWindowService(m_root_hwnd, guid_of<IWindowServiceProxy>(), wsp.get()));
            //check_bool(SetModernAppWindow(m_root_hwnd, m_corewnd_hwnd) || GetModernAppWindow(m_root_hwnd) == m_corewnd_hwnd);
        }
        */
        m_dwxs.Content(m_root_cp);
        {
            auto monitor = MonitorFromWindow(m_root_hwnd, MONITOR_DEFAULTTONEAREST);
            check_hresult(GetScaleFactorForMonitor(monitor, &m_scale_factor));
            m_dpi = GetDpiForWindow(m_root_hwnd);
        }

        // Let DComp pass messages to root window
        // TODO: Why WS_EX_LAYERED works? Will it cause XAML Islands to misbehave or experience
        //       performance degradation?
        SetWindowLongPtr(m_xaml_hwnd, GWL_EXSTYLE,
            GetWindowLongPtr(m_xaml_hwnd, GWL_EXSTYLE) | WS_EX_LAYERED);
#if WIN32XAML_ENABLE_LAYOUT_SYNCHRONIZATION
#if WIN32XAML_LAYOUT_SYNCHRONIZATION_USE_ALTERNATIVE
        // Failsafe method
        // Tell OS that layout has completed, and it is time to redraw Win32 frame
        // TODO: The client area will still flicker when there are too many elements
        //       (especially when ContentDialog is present), maybe system hasn't committed
        //       visuals to DComp device? Figure out the solution
        m_root_cp.SizeChanged([](auto&&, auto&&) {
            Windows::UI::Core::CoreWindowResizeManager::GetForCurrentView().NotifyLayoutCompleted();
        });
#endif
#endif
        m_title_bar->m_root_hwnd = m_root_hwnd;

        // TODO: UI Automation fix
        /*SetPropW(m_root_hwnd, L"UIA_WindowPatternEnabled", reinterpret_cast<HANDLE>(1));
        SetPropW(m_root_hwnd, L"UIA_HasOwnNonClientUIATree", reinterpret_cast<HANDLE>(1));*/

        scope_root_hwnd.release();
        scope_windows.release();
    }
    Window::~Window() { Close(); }
    void Window::Close() {
        if (!m_xaml_hwnd) { return; }

        ShowWindow(m_root_hwnd, SW_HIDE);
        // TODO: Investigate memory leak
        using Windows::UI::Xaml::Media::VisualTreeHelper;
        // Close ContentDialog if there is one
        for (auto&& i : VisualTreeHelper::GetOpenPopupsForXamlRoot(m_root_cp.XamlRoot())) {
            if (auto cd = i.Child().try_as<Windows::UI::Xaml::Controls::ContentDialog>()) {
                cd.Hide();
            }
            i.IsOpen(false);
        }
        if (m_input_sink_hwnd) {
            // TODO: Is it required to unassociate title bar element and input sink window?
            DestroyWindow(m_input_sink_hwnd);
        }
        if (m_is_main) {
            // Postpone main window cleanup (keeps CoreWindow alive)
            m_dwxs.Content(nullptr);
            //m_ev_closed.clear();
        }
        else {
            // System APIs are buggy and don't dispose resources automatically, so we are
            // forced to clean up the mess by setting nullptr and calling Close manually :(
            m_dwxs.Content(nullptr);
            m_dwxs.Close();
            DestroyWindow(m_root_hwnd);
            //m_ev_closed.clear();
        }
        m_xaml_hwnd = nullptr;
        m_ev_closed(*this, nullptr);
        std::erase_if(t_windows, [&](com_ptr<Window> const& v) { return v.get() == this; });
        if (t_windows.empty()) {
            // Finish postponed cleanup
            t_main_window->m_dwxs.Close();
            DestroyWindow(t_main_window->m_root_hwnd);
            t_main_window = nullptr;
        }
    }
    void Window::Activate() {
#if WIN32XAML_FIX_ACRYLIC_FIRST_ACTIVATION
        ShowWindow(m_xaml_hwnd, SW_SHOW);
        // Fix focus not applying due to window activation order changes
        SetFocus(m_xaml_hwnd);
        {   // Fix mysterious issue where XAML window doesn't resize automatically
            RECT rt_root;
            GetClientRect(m_root_hwnd, &rt_root);
            SetWindowPos(
                m_xaml_hwnd,
                nullptr,
                0, 0, rt_root.right, rt_root.bottom,
                SWP_NOMOVE | SWP_NOZORDER
            );
        }
#else
        ShowWindow(m_root_hwnd, SW_SHOW);
        ShowWindow(m_xaml_hwnd, SW_SHOW);
#endif

        // Force XAML window & content to load with correct size
        //SetWindowPos(m_root_hwnd, nullptr, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_FRAMECHANGED);

        [](auto that) -> fire_and_forget {
            using namespace std::literals;
            apartment_context ui_ctx;

            auto update_xaml_island_fn = [&] {
                RECT /*rt_root, */rt_xaml;
                //GetClientRect(that->m_root_hwnd, &rt_root);
                GetClientRect(that->m_xaml_hwnd, &rt_xaml);
                // Fix XAML layout issue
                SetWindowPos(
                    that->m_xaml_hwnd,
                    nullptr,
                    0, 0, rt_xaml.right, rt_xaml.bottom + 1,
                    SWP_NOMOVE | SWP_NOZORDER
                );
                SetWindowPos(
                    that->m_xaml_hwnd,
                    nullptr,
                    0, 0, rt_xaml.right, rt_xaml.bottom,
                    SWP_NOMOVE | SWP_NOZORDER
                );
            };

            Windows::Foundation::TimeSpan delays[] = { 50ms, 50ms, 400ms };
            for (auto i : delays) {
                co_await i;
                co_await ui_ctx;
                if (!that->m_xaml_hwnd) { co_return; }
                update_xaml_island_fn();
            }
        }(get_strong());
    }
    void Window::SetTitleBar(Windows::UI::Xaml::UIElement const& element) {
        using Windows::UI::Xaml::Hosting::ElementCompositionPreview;

        if (!m_is_main) {
            throw hresult_invalid_argument(L"Cannot set title bar for non-main window");
        }
        this->EnsureInputSinkWindow();

        auto cav_tb = Windows::ApplicationModel::Core::CoreApplication::GetCurrentView().TitleBar();
        if (!element) {
            cav_tb.as<ICoreApplicationViewTitleBarInternal>()->SetTitleBarVisual(nullptr);
            return;
        }

        auto elem_handoff_visual = ElementCompositionPreview::GetElementVisual(element);
        auto visual_partner = elem_handoff_visual.as<IDCompositionVisualPartner>();
        cav_tb.as<ICoreApplicationViewTitleBarInternal>()->SetTitleBarVisual(visual_partner.get());
    }
    hstring Window::Title() {
        auto str_size = GetWindowTextLengthW(m_root_hwnd);
        handle_type<winrt::impl::hstring_traits> precreated_hstr{
            winrt::impl::precreate_hstring_on_heap(str_size) };
        // SAFETY: It is safe to "overrun" the buffer since the extra space for
        //         the null terminator has been taken into account
        precreated_hstr.get()->length = GetWindowTextW(m_root_hwnd,
            const_cast<wchar_t*>(precreated_hstr.get()->ptr), str_size + 1);
        return { precreated_hstr.detach(), take_ownership_from_abi };
    }
    void Window::Title(hstring const& value) {
        SetWindowTextW(m_root_hwnd, value.c_str());
    }
    Windows::UI::Xaml::UIElement Window::Content() {
        return m_root_cp.Content().as<Windows::UI::Xaml::UIElement>();
    }
    void Window::Content(Windows::UI::Xaml::UIElement const& value) {
        m_root_cp.Content(value);
    }
    bool Window::ExtendsContentIntoTitleBar() {
        return m_is_frameless;
    }
    void Window::ExtendsContentIntoTitleBar(bool value) {
        if (m_is_frameless == value) { return; }
        if (value && !m_is_main) {
            throw hresult_invalid_argument(L"Cannot extend content into title bar for non-main window");
        }

        if (value) { this->EnterFramelessMode(); }
        else { this->LeaveFramelessMode(); }
        m_is_frameless = value;

        // TODO: Is flushing window frame required?

        if (value) {
            this->UpdateCaptionLayout();
            this->RedrawCaption();
        }
        this->UpdateCaptionVisibility(value);
        this->CommitDComp();
    }
    bool Window::UseTransparentBackground() {
        return !m_et_root_cp_actual_theme_changed;
    }
    void Window::UseTransparentBackground(bool value) {
        if (this->UseTransparentBackground() == value) { return; }
        if (value) {
            m_root_cp.ActualThemeChanged(m_et_root_cp_actual_theme_changed);
            m_et_root_cp_actual_theme_changed = {};
            m_root_cp.Background(nullptr);
        }
        else {
            m_et_root_cp_actual_theme_changed = m_root_cp.ActualThemeChanged(
                [](Windows::UI::Xaml::FrameworkElement const& sender, auto&&) {
                    auto cp = sender.as<Windows::UI::Xaml::Controls::ContentPresenter>();
                    cp.Background(brush_from_element_theme(cp.ActualTheme()));
                }
            );
            m_root_cp.Background(brush_from_element_theme(m_root_cp.ActualTheme()));
        }
    }
    event_token Window::Closed(TypedEventHandler<Win32Xaml::Window, IInspectable> const& handler) {
        return m_ev_closed.add(handler);
    }
    void Window::Closed(event_token const& token) noexcept {
        m_ev_closed.remove(token);
    }
    HWND Window::GetRootHwnd() {
        return m_root_hwnd;
    }
    unsigned Window::GetClientTopPadding(void) const {
        if (!m_should_remove_title) { return 0; }
        // TODO: Win11 already draws the top frame for us, take this into consideration
        return IsZoomed(m_root_hwnd) ? get_resize_frame_vertical_for_dpi(m_dpi) : 1;
    }
    LRESULT Window::WindowProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam) {
        auto redraw_caption_buttons_with_hittest_fn = [&](LRESULT ht_result, bool force_redraw) {
            bool handle_default{};

            auto bs_none_style = m_is_active ? CaptionButtonState::None : CaptionButtonState::Inactive;

            auto bs_min = bs_none_style;
            auto bs_max = bs_none_style;
            auto bs_close = bs_none_style;
            if (m_cur_pressed_caption_btn == CaptionButtonKind::None) {
                switch (ht_result) {
                case HTMINBUTTON:   bs_min = CaptionButtonState::PointerOver;       break;
                case HTMAXBUTTON:   bs_max = CaptionButtonState::PointerOver;       break;
                case HTCLOSE:       bs_close = CaptionButtonState::PointerOver;     break;
                default:            handle_default = true;                          break;
                }
            }
            else {
                if (ht_result == HTMINBUTTON && m_cur_pressed_caption_btn == CaptionButtonKind::Minimize) {
                    bs_min = CaptionButtonState::Pressed;
                }
                else if (ht_result == HTMAXBUTTON && (m_cur_pressed_caption_btn == CaptionButtonKind::Maximize ||
                    m_cur_pressed_caption_btn == CaptionButtonKind::Restore))
                {
                    bs_max = CaptionButtonState::Pressed;
                }
                else if (ht_result == HTCLOSE && m_cur_pressed_caption_btn == CaptionButtonKind::Close) {
                    bs_close = CaptionButtonState::Pressed;
                }
            }

            bool should_flush{};
            if (force_redraw) {
                m_bs_minimize = bs_min;
                m_bs_maximize_restore = bs_max;
                m_bs_close = bs_close;
                this->RedrawCaption();
                should_flush = true;
            }
            else {
                should_flush = this->UpdateAndRedrawCaption(bs_min, bs_max, bs_close);
            }
            if (should_flush) {
                this->CommitDComp();
            }

            return handle_default;
        };

        if (msg == WM_SIZE) {
            bool cur_is_maximized = wParam == SIZE_MAXIMIZED;

#if WIN32XAML_ENABLE_LAYOUT_SYNCHRONIZATION
#if !WIN32XAML_LAYOUT_SYNCHRONIZATION_USE_ALTERNATIVE
            // TODO: The performance is bad (UWP only calls this one time at initialization) and
            //       the way this works is likely a bug. Find a better way out.
            // Let the system XAML framework call CoreWindowResizeManager::NotifyLayoutCompleted
            // at the right time
            auto priv = Windows::UI::Xaml::Application::Current().as<IFrameworkApplicationPrivate>();
            check_hresult(priv->SetSynchronizationWindow(hwnd));
#endif
#endif

            RECT rt;
            GetClientRect(hwnd, &rt);
            rt.top = this->GetClientTopPadding();
            SetWindowPos(
                m_xaml_hwnd,
                nullptr,
                rt.left, rt.top,
                rt.right - rt.left, rt.bottom - rt.top,
                SWP_NOZORDER
            );
            // Workaround ContentDialog resizing bug, see
            // https://github.com/microsoft/microsoft-ui-xaml/issues/3577
            PostMessageW(m_corewnd_hwnd, msg, wParam, lParam);

            // Update title bar if necessary
            if (m_should_remove_title) {
                if (this->UpdateCaptionLayout()) {
                    this->RedrawCaption();
                }
                else if (m_last_is_maximized != cur_is_maximized) {
                    POINT pt;
                    GetCursorPos(&pt);
                    redraw_caption_buttons_with_hittest_fn(
                        this->WindowProc(hwnd, WM_NCHITTEST, 0, MAKELONG(pt.x, pt.y)),
                        true
                    );
                }
                this->CommitDComp();
            }

            m_last_is_maximized = cur_is_maximized;

            return 0;
        }
        else if (msg == WM_PAINT) {
            ValidateRect(hwnd, nullptr);

            if (m_should_remove_title) {
                // TODO: Optimize performance
                this->ResetCaptionResource();

                // Update title bar colors
                m_clr_cbtn_none_bkg = value_or(m_title_bar->m_btn_bkg_clr, { 0xff, 0xff, 0xff, 0xff });
                m_clr_cbtn_none_fore = value_or(m_title_bar->m_btn_fore_clr, { 0xff, 0x0, 0x0, 0x0 });
                m_clr_cbtn_hover_bkg = value_or(m_title_bar->m_btn_hover_bkg_clr, { 0xff, 0xff - 0x19, 0xff - 0x19, 0xff - 0x19 });
                m_clr_cbtn_hover_fore = value_or(m_title_bar->m_btn_hover_fore_clr, { 0xff, 0x0, 0x0, 0x0 });
                m_clr_cbtn_pressed_bkg = value_or(m_title_bar->m_btn_pressed_bkg_clr, { 0xff, 0xff - 0x33, 0xff - 0x33, 0xff - 0x33 });
                m_clr_cbtn_pressed_fore = value_or(m_title_bar->m_btn_pressed_fore_clr, { 0xff, 0x0, 0x0, 0x0 });
                m_clr_cbtn_inactive_bkg = value_or(m_title_bar->m_btn_inactive_bkg_clr, { 0xff, 0xff, 0xff, 0xff });
                m_clr_cbtn_inactive_fore = value_or(m_title_bar->m_btn_inactive_fore_clr, { 0xff, 0x99, 0x99, 0x99 });

                POINT pt;
                GetCursorPos(&pt);
                redraw_caption_buttons_with_hittest_fn(
                    this->WindowProc(hwnd, WM_NCHITTEST, 0, MAKELONG(pt.x, pt.y)),
                    true
                );
            }

            return 0;
        }
        else if (msg == WM_NCCALCSIZE) {
            if (!m_should_remove_title) { return DefWindowProcW(hwnd, msg, wParam, lParam); }

            // Removes title & upper border
            if (!wParam) { return 0; }
            auto params = reinterpret_cast<NCCALCSIZE_PARAMS*>(lParam);
            auto& client_rect = params->rgrc[0];

            auto original_top = client_rect.top;

            auto ret = DefWindowProcW(hwnd, WM_NCCALCSIZE, wParam, lParam);
            if (ret != 0) { return ret; }

            client_rect.top = original_top;

            // TODO: Auto-hide task bar?

            return 0;
        }
        else if (msg == WM_GETMINMAXINFO) {
            auto pmmi = reinterpret_cast<LPMINMAXINFO>(lParam);
            pmmi->ptMinTrackSize = { 400, 300 };

            return 0;
        }
        else if (msg == WM_DPICHANGED) {
            m_dpi = LOWORD(wParam);
            auto monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
            check_hresult(GetScaleFactorForMonitor(monitor, &m_scale_factor));

            RECT rt = *reinterpret_cast<RECT*>(lParam);
            SetWindowPos(hwnd, nullptr,
                rt.left, rt.top,
                rt.right - rt.left, rt.bottom - rt.top,
                SWP_NOZORDER | SWP_NOACTIVATE
            );
        }
        else if (msg == WM_SETFOCUS) {
            SetFocus(m_xaml_hwnd);
            return 0;
        }
        else if (msg == WM_ACTIVATE) {
            m_is_active = wParam != WA_INACTIVE;

            if (m_should_remove_title) {
                POINT pt;
                GetCursorPos(&pt);
                redraw_caption_buttons_with_hittest_fn(
                    this->WindowProc(hwnd, WM_NCHITTEST, 0, MAKELONG(pt.x, pt.y)),
                    false
                );
            }

            // TODO: Broadcast event Activated
        }
        else if (msg == WM_NCHITTEST) {
            if (!m_should_remove_title) { return DefWindowProcW(hwnd, msg, wParam, lParam); }

            RECT rt;
            GetWindowRect(hwnd, &rt);
            POINT pt = lparam_to_point(lParam);
            LRESULT ht_result;

            // Handle the top 1px border (UWP didn't properly handle this)
            if (rt.top == pt.y) {
                LPARAM new_lparam = MAKELONG(pt.x, rt.bottom - 1);
                ht_result = DefWindowProcW(hwnd, msg, wParam, new_lparam);
                if (ht_result == HTBOTTOMLEFT) { ht_result = HTTOPLEFT; }
                else if (ht_result == HTBOTTOMRIGHT) { ht_result = HTTOPRIGHT; }
                else { ht_result = HTTOP; }
            }
            else {
                ht_result = DefWindowProcW(hwnd, msg, wParam, lParam);
                if (ht_result == HTCLIENT) {
                    // Caption buttons hit test
                    ScreenToClient(hwnd, &pt);
                    if (PtInRect(&m_rt_btn_minimize, pt)) {
                        ht_result = HTMINBUTTON;
                    }
                    else if (PtInRect(&m_rt_btn_maximize_restore, pt)) {
                        ht_result = HTMAXBUTTON;
                    }
                    else if (PtInRect(&m_rt_btn_close, pt)) {
                        ht_result = HTCLOSE;
                    }
                }
            }

            return ht_result;
        }
        else if (msg == WM_NCPOINTERDOWN || msg == WM_POINTERDOWN) {
            if (!m_should_remove_title) { return DefWindowProcW(hwnd, msg, wParam, lParam); }
            bool handle_default{};

            m_is_nc_pointer_rpressed = IS_POINTER_SECONDBUTTON_WPARAM(wParam);

            auto bs_min = CaptionButtonState::None;
            auto bs_max = CaptionButtonState::None;
            auto bs_close = CaptionButtonState::None;
            auto ht_result = this->WindowProc(hwnd, WM_NCHITTEST, 0, lParam);
            switch (ht_result) {
            case HTMINBUTTON:
                bs_min = CaptionButtonState::Pressed;
                m_cur_pressed_caption_btn = CaptionButtonKind::Minimize;
                break;
            case HTMAXBUTTON:
                bs_max = CaptionButtonState::Pressed;
                m_cur_pressed_caption_btn = m_last_is_maximized ?
                    CaptionButtonKind::Restore : CaptionButtonKind::Maximize;
                break;
            case HTCLOSE:
                bs_close = CaptionButtonState::Pressed;
                m_cur_pressed_caption_btn = CaptionButtonKind::Close;
                break;
            default:            handle_default = true;                          break;
            }
            if (this->UpdateAndRedrawCaption(bs_min, bs_max, bs_close)) {
                this->CommitDComp();
            }

            if (!handle_default) {
                SetActiveWindow(hwnd);
                return 0;
            }
        }
        else if (msg == WM_NCPOINTERUP || msg == WM_POINTERUP) {
            if (!m_should_remove_title) { return DefWindowProcW(hwnd, msg, wParam, lParam); }
            bool handle_default{};

            auto ht_result = this->WindowProc(hwnd, WM_NCHITTEST, 0, lParam);
            WPARAM invoke_menu_id{};
            switch (m_cur_pressed_caption_btn) {
            case CaptionButtonKind::Minimize:
                if (ht_result == HTMINBUTTON) { invoke_menu_id = SC_MINIMIZE; }
                break;
            case CaptionButtonKind::Maximize:
                if (ht_result == HTMAXBUTTON) { invoke_menu_id = SC_MAXIMIZE; }
                break;
            case CaptionButtonKind::Restore:
                if (ht_result == HTMAXBUTTON) { invoke_menu_id = SC_RESTORE; }
                break;
            case CaptionButtonKind::Close:
                if (ht_result == HTCLOSE) { invoke_menu_id = SC_CLOSE; }
                break;
            }

            bool is_rclick = m_is_nc_pointer_rpressed;

            m_is_nc_pointer_rpressed = false;
            m_cur_pressed_caption_btn = {};

            if (invoke_menu_id != 0) {
                if (is_rclick) {
                    redraw_caption_buttons_with_hittest_fn(HTCAPTION, false);
                    handle_default = false;

                    const bool is_bidi_locale = false;
                    track_and_exec_sys_menu_for_window(hwnd, lparam_to_point(lParam), is_bidi_locale);
                }
                else {
                    handle_default = redraw_caption_buttons_with_hittest_fn(ht_result, false);

                    PostMessageW(m_root_hwnd, WM_SYSCOMMAND, invoke_menu_id, 0);
                }
            }
            else {
                handle_default = redraw_caption_buttons_with_hittest_fn(ht_result, false);
            }

            if (!handle_default) { return 0; }
        }
        else if (msg == WM_NCPOINTERUPDATE || msg == WM_POINTERUPDATE) {
            if (!m_should_remove_title) { return DefWindowProcW(hwnd, msg, wParam, lParam); }
            bool handle_default{};

            auto ht_result = this->WindowProc(hwnd, WM_NCHITTEST, 0, lParam);
            if (!(m_is_nc_pointer_rpressed ?
                IS_POINTER_SECONDBUTTON_WPARAM(wParam) : IS_POINTER_FIRSTBUTTON_WPARAM(wParam)))
            {
                m_is_nc_pointer_rpressed = false;
                m_cur_pressed_caption_btn = CaptionButtonKind::None;
            }
            if (m_cur_pressed_caption_btn == CaptionButtonKind::None &&
                IS_POINTER_FIRSTBUTTON_WPARAM(wParam))
            {
                // Resizing / moving may be taking place, ignore events
                handle_default = true;
            }
            else {
                handle_default = redraw_caption_buttons_with_hittest_fn(ht_result, false);
            }

            if (ht_result == HTMAXBUTTON) {
                // HACK: For WM_GETOBJECT
                auto ex_style = GetWindowLongPtr(m_xaml_hwnd, GWL_EXSTYLE);
                SetWindowLongPtr(m_xaml_hwnd, GWL_EXSTYLE, ex_style | WS_EX_TRANSPARENT);
            }
            else {
                // HACK: For WM_GETOBJECT
                auto ex_style = GetWindowLongPtr(m_xaml_hwnd, GWL_EXSTYLE);
                SetWindowLongPtr(m_xaml_hwnd, GWL_EXSTYLE, ex_style & ~WS_EX_TRANSPARENT);
            }

            if (!handle_default) { return 0; }
        }
        else if (msg == WM_POINTERLEAVE) {
            if (m_should_remove_title) {
                auto bs_none_style = m_is_active ? CaptionButtonState::None : CaptionButtonState::Inactive;
                if (this->UpdateAndRedrawCaption(bs_none_style, bs_none_style, bs_none_style)) {
                    this->CommitDComp();
                }

                // HACK: For WM_GETOBJECT
                auto ex_style = GetWindowLongPtr(m_xaml_hwnd, GWL_EXSTYLE);
                SetWindowLongPtr(m_xaml_hwnd, GWL_EXSTYLE, ex_style & ~WS_EX_TRANSPARENT);
            }
        }
        else if (msg == WM_NCMOUSEMOVE) {
            // Windows will still send WM_NCMOUSEMOVE when we already handled WM_NCPOINTER*,
            // so we must handle this ancient message as well
            // NOTE: We forward the requests to WM_NCPOINTERUPDATE
            const bool is_lbutton_down = GetKeyState(VK_LBUTTON) & 0x8000;
            const bool is_rbutton_down = GetKeyState(VK_RBUTTON) & 0x8000;
            WPARAM wParam{};
            wParam += is_lbutton_down ? POINTER_MESSAGE_FLAG_FIRSTBUTTON : 0;
            wParam += is_rbutton_down ? POINTER_MESSAGE_FLAG_SECONDBUTTON : 0;
            wParam <<= 16;
            return this->WindowProc(hwnd, WM_NCPOINTERUPDATE, wParam, lParam);
        }
        else if (msg == WM_GETOBJECT) {
            if (!m_should_remove_title) { return DefWindowProcW(hwnd, msg, wParam, lParam); }

            // Provide UI Automation objects to properly display Win11 snap layouts flyout
            // TODO: Properly implement UI Automation for title bar, otherwise those
            //       who need real accessibility will be REALLY unhappy

            // TODO: Exception handling (ExceptionBoundary)
            struct AppTitleBarAcc : implements<AppTitleBarAcc,
                IRawElementProviderSimple, IRawElementProviderFragmentRoot, IRawElementProviderFragment, IInvokeProvider>
            {
            private:
                enum class Role {
                    Window,
                    TitleBar,
                    MinButton,
                    MaxButton,
                    CloseButton,
                };

            public:
                AppTitleBarAcc(Window* wnd) : AppTitleBarAcc(wnd, Role::Window) {}
                AppTitleBarAcc(Window* wnd, Role role) : m_root_hwnd(wnd->m_root_hwnd),
                    m_root_wnd_title(wnd->Title()), m_role(role)
                {
                    m_rt_title_bar = {
                        wnd->m_rt_btn_minimize.left, wnd->m_rt_btn_minimize.top,
                        wnd->m_rt_btn_close.right, wnd->m_rt_btn_close.bottom,
                    };
                    m_rt_min_btn = wnd->m_rt_btn_minimize;
                    m_rt_max_btn = wnd->m_rt_btn_maximize_restore;
                    m_rt_close_btn = wnd->m_rt_btn_close;
                }
                AppTitleBarAcc(AppTitleBarAcc* other, Role role) : m_root_hwnd(other->m_root_hwnd),
                    m_root_wnd_title(other->m_root_wnd_title), m_rt_title_bar(other->m_rt_title_bar),
                    m_rt_min_btn(other->m_rt_min_btn), m_rt_max_btn(other->m_rt_max_btn),
                    m_rt_close_btn(other->m_rt_close_btn), m_role(role) {}

                // IRawElementProviderSimple
                HRESULT get_HostRawElementProvider(IRawElementProviderSimple** pRetVal) {
                    if (m_role == Role::Window) {
                        return UiaHostProviderFromHwnd(m_root_hwnd, pRetVal);
                    }
                    *pRetVal = nullptr;
                    return S_OK;
                }
                HRESULT get_ProviderOptions(ProviderOptions* pRetVal) {
                    *pRetVal = ProviderOptions_ServerSideProvider;
                    return S_OK;
                }
                HRESULT GetPatternProvider(PATTERNID patternId, IUnknown** pRetVal) {
                    *pRetVal = nullptr;
                    // TODO...
                    if (patternId == UIA_ValuePatternId) {
                        OutputDebugStringW(L"GetPatternProvider_UIA_ValuePatternId\n");
                    }
                    else if (patternId == UIA_InvokePatternId) {
                        OutputDebugStringW(L"GetPatternProvider_UIA_InvokePatternId\n");
                        *pRetVal = static_cast<IInvokeProvider*>(this); (*pRetVal)->AddRef();
                    }
                    else {
                        OutputDebugStringW(std::format(L"GetPatternProvider_OTHER, id = {}\n", patternId).c_str());
                    }
                    return S_OK;
                }
                HRESULT GetPropertyValue(PROPERTYID propertyId, VARIANT* pRetVal) {
                    HRESULT hr = S_OK;
                    OutputDebugStringW(std::format(L"GetPropertyValue, id = {}\n", propertyId).c_str());
                    if (propertyId == UIA_IsKeyboardFocusablePropertyId) {
                        hr = InitVariantFromBoolean(m_role == Role::Window, pRetVal);
                    }
                    else if (propertyId == UIA_HasKeyboardFocusPropertyId) {
                        hr = InitVariantFromBoolean(
                            m_role == Role::Window ? GetActiveWindow() == m_root_hwnd : false,
                            pRetVal
                        );
                    }
                    else if (propertyId == UIA_AutomationIdPropertyId) {
                        wchar_t const* prop_id;
                        switch (m_role) {
                        case Role::TitleBar:
                            prop_id = L"title bar";
                            break;
                        case Role::MinButton:
                            prop_id = L"minimize button";
                            break;
                        case Role::MaxButton:
                            prop_id = L"maximize button";
                            break;
                        case Role::CloseButton:
                            prop_id = L"close button";
                            break;
                        case Role::Window:
                            prop_id = L"window";
                            break;
                        default:
                            prop_id = L"";
                            break;
                        }
                        hr = InitVariantFromString(prop_id, pRetVal);
                    }
                    else if (propertyId == UIA_BoundingRectanglePropertyId) {
                        UiaRect rt;
                        hr = this->get_BoundingRectangle(&rt);
                        DOUBLE arr[4]{ rt.left, rt.top, rt.width, rt.height };
                        hr = InitVariantFromDoubleArray(arr, 4, pRetVal);
                    }
                    else if (propertyId == UIA_ControlTypePropertyId) {
                        long type_id;
                        switch (m_role) {
                        case Role::TitleBar:
                            type_id = UIA_TitleBarControlTypeId;
                            break;
                        case Role::MinButton:
                            type_id = UIA_ButtonControlTypeId;
                            break;
                        case Role::MaxButton:
                            type_id = UIA_ButtonControlTypeId;
                            break;
                        case Role::CloseButton:
                            type_id = UIA_ButtonControlTypeId;
                            break;
                        case Role::Window:
                        default:
                            type_id = UIA_WindowControlTypeId;
                            break;
                        }
                        hr = InitVariantFromInt32(type_id, pRetVal);
                    }
                    else if (propertyId == UIA_LocalizedControlTypePropertyId) {
                        hr = InitVariantFromString(L"", pRetVal);
                    }
                    else if (propertyId == UIA_NamePropertyId) {
                        wchar_t const* str = L"";
                        switch (m_role) {
                        case Role::Window:
                            str = m_root_wnd_title.c_str();
                            break;
                        case Role::TitleBar:
                            str = m_root_wnd_title.c_str();
                            break;
                        case Role::MinButton:
                            str = L"Minimize";
                            break;
                        case Role::MaxButton:
                            str = IsZoomed(m_root_hwnd) ? L"Restore" : L"Maximize";
                            break;
                        case Role::CloseButton:
                            str = L"Close";
                            break;
                        }
                        hr = InitVariantFromString(str, pRetVal);
                    }
                    else if (propertyId == UIA_IsControlElementPropertyId) {
                        hr = InitVariantFromBoolean(true, pRetVal);
                    }
                    else {
                        VariantInit(pRetVal);
                    }
                    return hr;
                }

                // IRawElementProviderFragmentRoot
                HRESULT ElementProviderFromPoint(double x, double y, IRawElementProviderFragment** pRetVal) {
                    POINT pt{ static_cast<LONG>(x), static_cast<LONG>(y) };
                    Role role = Role::Window;
                    switch (SendMessageW(m_root_hwnd, WM_NCHITTEST, 0, MAKELONG(pt.x, pt.y))) {
                    case HTMINBUTTON:       role = Role::MinButton;         break;
                    case HTMAXBUTTON:       role = Role::MaxButton;         break;
                    case HTCLOSE:           role = Role::CloseButton;       break;
                    case HTCAPTION:         role = Role::TitleBar;          break;
                    default:                break;
                    }
                    *pRetVal = make_self<AppTitleBarAcc>(this, role).detach();
                    return S_OK;
                }
                HRESULT GetFocus(IRawElementProviderFragment** pRetVal) {
                    OutputDebugStringW(L"GetFocus\n");
                    *pRetVal = nullptr;
                    return S_OK;
                }

                // IRawElementProviderFragment
                HRESULT get_BoundingRectangle(UiaRect* pRetVal) {
                    bool should_convert{ true };
                    RECT rt;
                    switch (m_role) {
                    case Role::TitleBar:        rt = m_rt_title_bar;        break;
                    case Role::MinButton:       rt = m_rt_min_btn;          break;
                    case Role::MaxButton:       rt = m_rt_max_btn;          break;
                    case Role::CloseButton:     rt = m_rt_close_btn;        break;
                    case Role::Window:
                    default:
                        GetWindowRect(m_root_hwnd, &rt);
                        should_convert = false;
                        break;
                    }
                    if (should_convert) {
                        MapWindowPoints(m_root_hwnd, nullptr, reinterpret_cast<POINT*>(&rt), 2);
                    }
                    *pRetVal = {
                        static_cast<double>(rt.left),
                        static_cast<double>(rt.top),
                        static_cast<double>(rt.right - rt.left),
                        static_cast<double>(rt.bottom - rt.top),
                    };
                    return S_OK;
                }
                HRESULT get_FragmentRoot(IRawElementProviderFragmentRoot** pRetVal) {
                    OutputDebugStringW(L"get_FragmentRoot\n");
                    *pRetVal = make_self<AppTitleBarAcc>(this, Role::Window).detach();
                    return S_OK;
                }
                HRESULT GetEmbeddedFragmentRoots(SAFEARRAY** pRetVal) {
                    OutputDebugStringW(L"GetEmbeddedFragmentRoots\n");
                    *pRetVal = nullptr;
                    return S_OK;
                }
                HRESULT GetRuntimeId(SAFEARRAY** pRetVal) {
                    OutputDebugStringW(L"GetRuntimeId\n");
                    *pRetVal = nullptr;
                    if (pRetVal == NULL)
                    {
                        return E_INVALIDARG;
                    }

                    int rId[] = { UiaAppendRuntimeId, static_cast<int>(m_role) };
                    SAFEARRAY* psa = SafeArrayCreateVector(VT_I4, 0, 2);
                    if (psa == NULL)
                    {
                        return E_OUTOFMEMORY;
                    }

                    for (LONG i = 0; i < 2; i++)
                    {
                        SafeArrayPutElement(psa, &i, (void*)&(rId[i]));
                    }

                    *pRetVal = psa;
                    return S_OK;
                }
                HRESULT Navigate(NavigateDirection direction, IRawElementProviderFragment** pRetVal) {
                    *pRetVal = nullptr;
                    if (direction == NavigateDirection_Parent) {
                        OutputDebugStringW(L"NavigateDirection_Parent\n");
                        switch (m_role) {
                        case Role::TitleBar:
                            *pRetVal = make_self<AppTitleBarAcc>(this, Role::Window).detach();
                            break;
                        case Role::MinButton:
                            *pRetVal = make_self<AppTitleBarAcc>(this, Role::TitleBar).detach();
                            break;
                        case Role::MaxButton:
                            *pRetVal = make_self<AppTitleBarAcc>(this, Role::TitleBar).detach();
                            break;
                        case Role::CloseButton:
                            *pRetVal = make_self<AppTitleBarAcc>(this, Role::TitleBar).detach();
                            break;
                        case Role::Window:
                        default:
                            // No parents
                            return S_OK;
                        }
                    }
                    else if (direction == NavigateDirection_NextSibling) {
                        OutputDebugStringW(L"NavigateDirection_NextSibling\n");
                        switch (m_role) {
                        case Role::MinButton:
                            *pRetVal = make_self<AppTitleBarAcc>(this, Role::MaxButton).detach();
                            break;
                        case Role::MaxButton:
                            *pRetVal = make_self<AppTitleBarAcc>(this, Role::CloseButton).detach();
                            break;
                        case Role::CloseButton:
                        default:
                            // No more siblings
                            return S_OK;
                        }
                    }
                    else if (direction == NavigateDirection_PreviousSibling) {
                        OutputDebugStringW(L"NavigateDirection_PreviousSibling\n");
                        switch (m_role) {
                        case Role::MaxButton:
                            *pRetVal = make_self<AppTitleBarAcc>(this, Role::MinButton).detach();
                            break;
                        case Role::CloseButton:
                            *pRetVal = make_self<AppTitleBarAcc>(this, Role::MaxButton).detach();
                            break;
                        case Role::MinButton:
                        default:
                            // No more siblings
                            return S_OK;
                        }
                    }
                    else if (direction == NavigateDirection_FirstChild) {
                        OutputDebugStringW(L"NavigateDirection_FirstChild\n");
                        switch (m_role) {
                        case Role::Window:
                            *pRetVal = make_self<AppTitleBarAcc>(this, Role::TitleBar).detach();
                            break;
                        case Role::TitleBar:
                            *pRetVal = make_self<AppTitleBarAcc>(this, Role::MinButton).detach();
                            break;
                        default:
                            // No children
                            return S_OK;
                        }
                    }
                    else if (direction == NavigateDirection_LastChild) {
                        OutputDebugStringW(L"NavigateDirection_LastChild\n");
                        switch (m_role) {
                        case Role::Window:
                            *pRetVal = make_self<AppTitleBarAcc>(this, Role::TitleBar).detach();
                            break;
                        case Role::TitleBar:
                            *pRetVal = make_self<AppTitleBarAcc>(this, Role::CloseButton).detach();
                            break;
                        default:
                            // No children
                            return S_OK;
                        }
                    }
                    else {
                        return E_INVALIDARG;
                    }
                    return S_OK;
                }
                HRESULT SetFocus() {
                    OutputDebugStringW(L"SetFocus\n");
                    return E_NOTIMPL;
                }

                // IInvokeProvider
                HRESULT Invoke() {
                    WPARAM wParam;
                    switch (m_role) {
                    case Role::MinButton:
                        wParam = SC_MINIMIZE;
                        break;
                    case Role::MaxButton:
                        wParam = IsZoomed(m_root_hwnd) ? SC_RESTORE : SC_MAXIMIZE;
                        break;
                    case Role::CloseButton:
                        wParam = SC_CLOSE;
                        break;
                    default:
                        return E_NOTIMPL;
                    }
                    PostMessageW(m_root_hwnd, WM_SYSCOMMAND, wParam, 0);
                    return S_OK;
                }

            private:
                HWND m_root_hwnd;
                hstring m_root_wnd_title;
                RECT m_rt_title_bar;
                RECT m_rt_min_btn;
                RECT m_rt_max_btn;
                RECT m_rt_close_btn;
                Role m_role;
            };

            if (static_cast<DWORD>(lParam) == static_cast<DWORD>(UiaRootObjectId)) {
                // Trick OS into believing it got the maximize button accessibility object
                auto elem_provider = make<AppTitleBarAcc>(this);
                return UiaReturnRawElementProvider(hwnd, wParam, lParam, elem_provider.get());
            }
        }
        return DefWindowProcW(hwnd, msg, wParam, lParam);
    }
    LRESULT Window::InputSinkWindowProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam) {
        // We should perform hit testing ourselves
        if (msg == WM_SETCURSOR) {
            /*if (HIWORD(lParam) != WM_LBUTTONDOWN) {
                return DefWindowProcW(hwnd, msg, wParam, lParam);
            }*/
            return true;
        }
        // We don't handle other messages such as WM_POINTER*, simply forward them
        if (!(WM_MOUSEFIRST <= msg && msg <= WM_MOUSELAST)) {
            return DefWindowProcW(hwnd, msg, wParam, lParam);
        }

        // We only need to take care of top border and title bar, the rest is handled by system
        int ht_result = HTCAPTION;
        auto vtopframe_size = get_resize_frame_vertical_for_dpi(m_dpi);
        auto pt = lparam_to_point(lParam);
        auto tick = GetTickCount64();
        if (pt.y < vtopframe_size) {
            // Let the outer window procedure do the heavy lifting
            RECT rt;
            POINT pt_screen{ pt };
            ClientToScreen(hwnd, &pt_screen);
            GetWindowRect(m_root_hwnd, &rt);
            pt_screen.y = rt.top;
            auto new_lparam = MAKELONG(pt_screen.x, pt_screen.y);
            ht_result = static_cast<int>(this->WindowProc(m_root_hwnd, WM_NCHITTEST, 0, new_lparam));
        }

        auto update_last_record_fn = [&] {
            m_input_sink_last_point = pt;
            m_input_sink_last_tick = tick;
        };
        auto is_double_click_fn = [&] {
            if (tick - m_input_sink_last_tick > GetDoubleClickTime()) { return false; }
            auto cxrt = GetSystemMetricsForDpi(SM_CXDOUBLECLK, m_dpi);
            auto cyrt = GetSystemMetricsForDpi(SM_CYDOUBLECLK, m_dpi);
            if (std::abs(pt.x - m_input_sink_last_point.x) > cxrt) { return false; }
            if (std::abs(pt.y - m_input_sink_last_point.y) > cyrt) { return false; }
            return true;
        };

        if (msg == WM_MOUSEMOVE) {
            HCURSOR final_cursor;
            switch (ht_result) {
            case HTTOP:         final_cursor = LoadCursorW(nullptr, IDC_SIZENS);        break;
            case HTTOPLEFT:     final_cursor = LoadCursorW(nullptr, IDC_SIZENWSE);      break;
            case HTTOPRIGHT:    final_cursor = LoadCursorW(nullptr, IDC_SIZENESW);      break;
            default:            final_cursor = LoadCursorW(nullptr, IDC_ARROW);         break;
            }
            SetCursor(final_cursor);
        }
        else if (msg == WM_RBUTTONUP) {
            // TODO: Fix title bar context menu logic
            POINT pt_screen{ pt };
            ClientToScreen(hwnd, &pt_screen);
            const bool is_bidi_locale = false;
            track_and_exec_sys_menu_for_window(m_root_hwnd, pt_screen, is_bidi_locale);
        }
        else if (msg == WM_LBUTTONDOWN) {
            if (is_double_click_fn()) {
                this->WindowProc(m_root_hwnd, WM_NCLBUTTONDBLCLK, ht_result, 0);
            }
            else {
                WPARAM final_wparam;
                switch (ht_result) {
                case HTLEFT:
                case HTRIGHT:
                case HTTOP:
                case HTTOPLEFT:
                case HTTOPRIGHT:
                case HTBOTTOM:
                case HTBOTTOMLEFT:
                case HTBOTTOMRIGHT:
                    final_wparam = SC_SIZE | (ht_result - HTMAXBUTTON);
                    break;
                default:
                    final_wparam = SC_MOVE | HTCAPTION;
                    break;
                }
                PostMessageW(m_root_hwnd, WM_SYSCOMMAND, final_wparam, lParam);
            }
            update_last_record_fn();
        }

        return 0;
    }
    void Window::EnsureInputSinkWindow(void) {
        if (m_input_sink_hwnd) { return; }
        m_input_sink_hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_NOREDIRECTIONBITMAP,
            g_win32xaml_input_sink_class_name,
            nullptr,
            WS_VISIBLE | WS_CHILD,
            0, 0, 0, 0,
            m_root_hwnd,
            nullptr,
            g_hinst,
            this
        );
        check_pointer(m_input_sink_hwnd);
        auto cav_tb = Windows::ApplicationModel::Core::CoreApplication::GetCurrentView().TitleBar();
        // NOTE: It is unlikely that we can simply invoke the syscall used by SetInputSinkWindow
        //       (which can help us get rid of single window limitation) due to compatibility (?)
        //       reasons, so keep this for the time being
        cav_tb.as<ICoreApplicationViewTitleBarInternal>()->SetInputSinkWindow(m_input_sink_hwnd);
    }
    void Window::EnterFramelessMode() {
        m_should_remove_title = true;
        {
            // Show dark border to cancel for the outcome of DwmExtendFrameIntoClientArea
            // NOTE: UWP uses CreateWindowInBandEx with dwTypeFlags = 3 to achieve dark borders
            //       while preserving default frame colors (white). Fortunately, the frame (title bar)
            //       is invisible, so we can simply instruct the entire window to use dark mode to
            //       mimic UWP behavior (although there is a subtle difference in colors).
            constexpr auto DWMWA_USE_IMMERSIVE_DARK_MODE = 20;
            BOOL b{ true };
            DwmSetWindowAttribute(m_root_hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, &b, sizeof b);
        }
        {
            // Prevent DWM from drawing caption buttons and title text
            // dwFlags, dwMask
            DWORD dws[2]{ 0x10007, 0x1000F };
            SetWindowThemeAttribute(m_root_hwnd, WTA_NONCLIENT, dws, sizeof dws);

            // Let DWM draw the top border (UWP behavior)
            auto wnd_style = static_cast<DWORD>(GetWindowLongPtrW(m_root_hwnd, GWL_STYLE));
            auto wnd_exstyle = static_cast<DWORD>(GetWindowLongPtrW(m_root_hwnd, GWL_EXSTYLE));
            RECT rt{};
            AdjustWindowRectEx(&rt, wnd_style, false, wnd_exstyle);
            MARGINS margins{};
            margins.cyTopHeight = -rt.top;
            DwmExtendFrameIntoClientArea(m_root_hwnd, &margins);
        }
    }
    void Window::LeaveFramelessMode() {
        m_should_remove_title = false;
        {
            constexpr auto DWMWA_USE_IMMERSIVE_DARK_MODE = 20;
            BOOL b{ false };
            DwmSetWindowAttribute(m_root_hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, &b, sizeof b);
        }
        {
            DWORD dws[2]{ 0x0, 0x1000F };
            SetWindowThemeAttribute(m_root_hwnd, WTA_NONCLIENT, dws, sizeof dws);

            MARGINS margins{};
            DwmExtendFrameIntoClientArea(m_root_hwnd, &margins);
        }
    }
    void Window::InitializeDComp(void) {
        // NOTE: DComp softens the DWM thumbnail (XAML Islands otherwise looks pixelated)
        //       for some reason, which aligns with UWP & traditional Win32 ones and is our
        //       desired outcome
        using Windows::UI::Composition::Compositor;
        //auto compositor = Windows::UI::Xaml::Window::Current().Compositor();
        com_ptr<ID3D11Device> d3d11_dev;
        com_ptr<IDXGIDevice1> dxgi_dev;
        // WARP device never resets (?), which is the choice of UWP and WinUI 3
        check_hresult(D3D11CreateDevice(nullptr, D3D_DRIVER_TYPE_WARP, nullptr,
            D3D11_CREATE_DEVICE_BGRA_SUPPORT, nullptr, 0, D3D11_SDK_VERSION,
            d3d11_dev.put(), nullptr, nullptr));
        d3d11_dev.as(dxgi_dev);

        /*auto interop_compositor_factory = get_activation_factory<Compositor, IInteropCompositorFactoryPartner>();
        com_ptr<IInteropCompositorPartner> interop_compositor;
        interop_compositor.capture(interop_compositor_factory, &IInteropCompositorFactoryPartner::CreateInteropCompositor,
            dxgi_dev.get(), nullptr);
        auto compositor = interop_compositor.as<Compositor>();
        m_dcomp_dev = compositor.as<IDCompositionDesktopDevice>();*/
        m_dcomp_dev.capture(DCompositionCreateDevice3, dxgi_dev.get());

        com_ptr<IDCompositionVisual2> dcomp_v2;

        check_hresult(m_dcomp_dev->CreateTargetForHwnd(m_root_hwnd, true, m_dcomp_target_top.put()));
        check_hresult(m_dcomp_dev->CreateVisual(m_v_top.put()));
        check_hresult(m_dcomp_target_top->SetRoot(m_v_top.get()));

        check_hresult(m_dcomp_dev->CreateVisual(dcomp_v2.put()));
        dcomp_v2.as(m_v_caption_buttons); dcomp_v2 = nullptr;
        check_hresult(m_v_top->AddVisual(m_v_caption_buttons.get(), true, nullptr));
        check_hresult(m_dcomp_dev->CreateVisual(m_v_caption_button_minimize.put()));
        check_hresult(m_dcomp_dev->CreateVisual(m_v_caption_button_maximize_restore.put()));
        check_hresult(m_dcomp_dev->CreateVisual(m_v_caption_button_close.put()));
        check_hresult(m_v_caption_buttons->AddVisual(m_v_caption_button_minimize.get(), true, nullptr));
        check_hresult(m_v_caption_buttons->AddVisual(m_v_caption_button_maximize_restore.get(), true, nullptr));
        check_hresult(m_v_caption_buttons->AddVisual(m_v_caption_button_close.get(), true, nullptr));

        // TODO: Background for all buttons (?)
        com_ptr<IDCompositionSurface> dcomp_surface;
        check_hresult(m_dcomp_dev->CreateSurface(1, 1, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_ALPHA_MODE_PREMULTIPLIED, dcomp_surface.put()));
        //populate_1x1_bgra_premul_dcomp_surface(dcomp_surface.get(), Windows::UI::Colors::Transparent());
        check_hresult(m_v_caption_buttons->SetContent(dcomp_surface.get()));
        // TODO: Remove these
        /*com_ptr<IDCompositionSurface> dcomp_surface;
        check_hresult(m_dcomp_dev->CreateVisual(dcomp_v2.put()));
        dcomp_v2.as(m_v_caption_buttons); dcomp_v2 = nullptr;
        check_hresult(m_v_top->AddVisual(m_v_caption_buttons.get(), true, nullptr));
        check_hresult(m_v_caption_buttons->SetOffsetX(100));
        check_hresult(m_dcomp_dev->CreateSurface(1, 1, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_ALPHA_MODE_PREMULTIPLIED, dcomp_surface.put()));
        populate_1x1_bgra_dcomp_surface(dcomp_surface.get(), Windows::UI::Colors::White());
        check_hresult(m_v_caption_buttons->SetContent(dcomp_surface.get()));
        check_hresult(static_cast<IDCompositionVisual2*>(m_v_caption_buttons.get())->SetTransform(D2D1::Matrix3x2F::Scale({ 100, 120 })));*/

#if WIN32XAML_ENABLE_LAYOUT_SYNCHRONIZATION
        m_fn_commit_dcomp = [&]() -> std::function<void()> {
            using namespace ::Win32Xaml::sys_interface;
            if (auto comp_dev = m_dcomp_dev.try_as<v10_0_19045::IDCompositionDesktopDevicePartner6>()) {
                return [=] {
                    handle dcomp_handle;
                    GetResizeDCompositionSynchronizationObject(m_root_hwnd, dcomp_handle.put());
                    check_hresult(comp_dev->SynchronizedCommit(dcomp_handle.get()));
                };
            }
            if (auto comp_dev = m_dcomp_dev.try_as<v10_0_22621::IDCompositionDesktopDevicePartner6>()) {
                return [=] {
                    handle dcomp_handle;
                    GetResizeDCompositionSynchronizationObject(m_root_hwnd, dcomp_handle.put());
                    check_hresult(comp_dev->SynchronizedCommit(dcomp_handle.get()));
                };
            }
            throw hresult_error(E_FAIL, L"OS is unsupported (Invalid IDCompositionDesktopDevicePartner6)");
        }();
#endif
    }
    void Window::CommitDComp(void) {
#if WIN32XAML_ENABLE_LAYOUT_SYNCHRONIZATION
        m_fn_commit_dcomp();
#else
        check_hresult(m_dcomp_dev->Commit());
#endif
    }
    void Window::UpdateCaptionVisibility(bool visible) {
        // NOTE: Do NOT use SetVisible(false), which ruins the DWM thumbnail
        check_hresult(m_v_caption_buttons->SetOpacity(visible ? 1.0f : 0.0f));
    }
    bool Window::UpdateCaptionLayout(void) {
        bool needs_redraw{};

        RECT rt;
        GetClientRect(m_root_hwnd, &rt);
        rt.top = this->GetClientTopPadding();
        // TODO: Double check whether this is exact UWP metrics
        SIZE sz_caption_btn{
            CAPTION_BUTTON_WIDTH * m_dpi / 96, CAPTION_BUTTON_HEIGHT * m_dpi / 96
        };
        check_hresult(m_v_caption_buttons->SetOffsetX(static_cast<float>(rt.right - sz_caption_btn.cx * 3)));
        check_hresult(m_v_caption_buttons->SetOffsetY(static_cast<float>(rt.top)));
        check_hresult(m_v_caption_button_minimize->SetOffsetX(0.0f));
        check_hresult(m_v_caption_button_minimize->SetOffsetY(0.0f));
        check_hresult(m_v_caption_button_maximize_restore->SetOffsetX(static_cast<float>(sz_caption_btn.cx)));
        check_hresult(m_v_caption_button_maximize_restore->SetOffsetY(0.0f));
        check_hresult(m_v_caption_button_close->SetOffsetX(static_cast<float>(sz_caption_btn.cx * 2)));
        check_hresult(m_v_caption_button_close->SetOffsetY(0.0f));
        m_rt_btn_close = { rt.right - sz_caption_btn.cx, rt.top, rt.right, rt.top + sz_caption_btn.cy };
        m_rt_btn_maximize_restore = m_rt_btn_close;
        OffsetRect(&m_rt_btn_maximize_restore, -sz_caption_btn.cx, 0);
        m_rt_btn_minimize = m_rt_btn_maximize_restore;
        OffsetRect(&m_rt_btn_minimize, -sz_caption_btn.cx, 0);

        // If DPI changed, clear caption button visual contents
        if (m_rt_caption_button.right != sz_caption_btn.cx ||
            m_rt_caption_button.bottom != sz_caption_btn.cy)
        {
            m_rt_caption_button = { 0, 0, sz_caption_btn.cx, sz_caption_btn.cy };
            this->ResetCaptionResource();
            needs_redraw = true;
        }

        return needs_redraw;
    }
    void Window::RedrawCaptionButton(CaptionButtonKind kind) {
        BITMAP src_bmp_info;
        HBITMAP src_bmp;
        IDCompositionSurface* dcomp_surface;
        CaptionButtonState btn_state;
        Windows::UI::Color bkg_color;
        bool is_close_btn{};

        this->EnsureCaptionResource();

        switch (kind) {
        case CaptionButtonKind::Minimize:
            btn_state = m_bs_minimize;
            dcomp_surface = m_sf_caption_button_minimize.get();
            src_bmp = m_gdi_icon_sets[btn_state].bmp_minimize.get();
            break;
        case CaptionButtonKind::Maximize:
            btn_state = m_bs_maximize_restore;
            dcomp_surface = m_sf_caption_button_maximize_restore.get();
            src_bmp = m_gdi_icon_sets[btn_state].bmp_maximize.get();
            break;
        case CaptionButtonKind::Restore:
            btn_state = m_bs_maximize_restore;
            dcomp_surface = m_sf_caption_button_maximize_restore.get();
            src_bmp = m_gdi_icon_sets[btn_state].bmp_restore.get();
            break;
        case CaptionButtonKind::Close:
            is_close_btn = true;
            btn_state = m_bs_close;
            dcomp_surface = m_sf_caption_button_close.get();
            src_bmp = m_gdi_icon_sets[btn_state].bmp_close.get();
            break;
        default:
            throw hresult_invalid_argument(L"Invalid caption button kind");
        }

        switch (btn_state) {
        case CaptionButtonState::PointerOver:
            if (is_close_btn) {
                // Close button has fixed color
                bkg_color = { 0xff, 0xe8, 0x11, 0x23 };
            }
            else {
                bkg_color = m_clr_cbtn_hover_bkg;
            }
            break;
        case CaptionButtonState::Pressed:
            if (is_close_btn) {
                // Close button has fixed color
                bkg_color = { 0xff, 0xf1, 0x70, 0x7a };
            }
            else {
                bkg_color = m_clr_cbtn_pressed_bkg;
            }
            break;
        case CaptionButtonState::Inactive:
            bkg_color = m_clr_cbtn_inactive_bkg;
            break;
        case CaptionButtonState::None:
        default:
            bkg_color = m_clr_cbtn_none_bkg;
            break;
        }

        POINT update_offset;
        winrt::com_ptr<IDXGISurface1> dxgi_surface;
        dxgi_surface.capture(MakeDCompSurfBDCompatShim(dcomp_surface), nullptr, &update_offset);
        HDC hdc;
        RECT draw_rt = m_rt_caption_button;
        OffsetRect(&draw_rt, update_offset.x, update_offset.y);
        check_hresult(dxgi_surface->GetDC(true, &hdc));
        auto se_hdc = util::misc::scope_exit([&] { dxgi_surface->ReleaseDC(&draw_rt); });

        fill_rect_with_color_premul(hdc, draw_rt, bkg_color);
        HDC temp_dc = CreateCompatibleDC(nullptr);
        check_pointer(temp_dc);
        auto se_temp_dc = util::misc::scope_exit([&] { DeleteDC(temp_dc); });
        auto old_obj = SelectObject(temp_dc, src_bmp);
        check_bool(GetObjectW(src_bmp, sizeof src_bmp_info, &src_bmp_info));
        BLENDFUNCTION bf{
            .BlendOp = AC_SRC_OVER,
            .BlendFlags = 0,
            .SourceConstantAlpha = 0xff,
            .AlphaFormat = AC_SRC_ALPHA,
        };
        SIZE bmp_draw_sz{
            (m_rt_caption_button.right - src_bmp_info.bmWidth) / 2,
            (m_rt_caption_button.bottom - src_bmp_info.bmHeight) / 2,
        };
        GdiAlphaBlend(hdc, draw_rt.left + bmp_draw_sz.cx, draw_rt.top + bmp_draw_sz.cy,
            src_bmp_info.bmWidth, src_bmp_info.bmHeight,
            temp_dc, 0, 0, src_bmp_info.bmWidth, src_bmp_info.bmHeight, bf);
        SelectObject(temp_dc, old_obj);

        check_hresult(dcomp_surface->EndDraw());
    }
    void Window::RedrawCaption(void) {
        this->RedrawCaptionButton(CaptionButtonKind::Minimize);
        if (IsZoomed(m_root_hwnd)) {
            this->RedrawCaptionButton(CaptionButtonKind::Restore);
        }
        else {
            this->RedrawCaptionButton(CaptionButtonKind::Maximize);
        }
        this->RedrawCaptionButton(CaptionButtonKind::Close);
    }
    bool Window::UpdateAndRedrawCaption(
        CaptionButtonState bs_min, CaptionButtonState bs_max, CaptionButtonState bs_close
    ) {
        bool has_update{};
        if (m_bs_minimize != bs_min) {
            m_bs_minimize = bs_min;
            this->RedrawCaptionButton(CaptionButtonKind::Minimize);
            has_update = true;
        }
        if (m_bs_maximize_restore != bs_max) {
            m_bs_maximize_restore = bs_max;
            auto kind = IsZoomed(m_root_hwnd) ? CaptionButtonKind::Restore : CaptionButtonKind::Maximize;
            this->RedrawCaptionButton(kind);
            has_update = true;
        }
        if (m_bs_close != bs_close) {
            m_bs_close = bs_close;
            this->RedrawCaptionButton(CaptionButtonKind::Close);
            has_update = true;
        }
        return has_update;
    }
    void Window::EnsureCaptionResource(void) {
        auto ensure_button_content = [&](
            com_ptr<IDCompositionSurface>& surface, IDCompositionVisual* visual)
        {
            if (surface) { return; }
            check_hresult(m_dcomp_dev->CreateSurface(
                m_rt_caption_button.right, m_rt_caption_button.bottom,
                DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_ALPHA_MODE_PREMULTIPLIED,
                surface.put()
            ));
            check_hresult(visual->SetContent(surface.get()));
        };
        ensure_button_content(m_sf_caption_button_minimize, m_v_caption_button_minimize.get());
        ensure_button_content(m_sf_caption_button_maximize_restore, m_v_caption_button_maximize_restore.get());
        ensure_button_content(m_sf_caption_button_close, m_v_caption_button_close.get());
        for (size_t i = 0; i < CaptionButtonStateLastIndex; i++) {
            if (m_gdi_icon_sets[i].scale_factor == m_scale_factor) { continue; }
            Windows::UI::Color fore_color, close_fore_color;
            constexpr Windows::UI::Color clr_white = { 0xff, 0xff, 0xff, 0xff };
            constexpr Windows::UI::Color clr_black = { 0xff, 0x0, 0x0, 0x0 };
            switch (i) {
            case CaptionButtonState::PointerOver:
                fore_color = m_clr_cbtn_hover_fore; close_fore_color = clr_white;
                break;
            case CaptionButtonState::Pressed:
                fore_color = m_clr_cbtn_pressed_fore; close_fore_color = clr_black;
                break;
            case CaptionButtonState::Inactive:
                fore_color = m_clr_cbtn_inactive_fore; close_fore_color = m_clr_cbtn_inactive_fore;
                break;
            case CaptionButtonState::None:
            default:
                fore_color = m_clr_cbtn_none_fore; close_fore_color = m_clr_cbtn_none_fore;
                break;
            }
            m_gdi_icon_sets[i] = GdiIconSet::load_colored(m_scale_factor, fore_color, close_fore_color);
        }
    }
    void Window::ResetCaptionResource(void) {
        m_v_caption_button_minimize->SetContent(nullptr);
        m_v_caption_button_maximize_restore->SetContent(nullptr);
        m_v_caption_button_close->SetContent(nullptr);
        m_sf_caption_button_minimize = nullptr;
        m_sf_caption_button_maximize_restore = nullptr;
        m_sf_caption_button_close = nullptr;
        for (size_t i = 0; i < CaptionButtonStateLastIndex; i++) {
            m_gdi_icon_sets[i] = {};
        }
    }

    bool ShellIcon::IsActive() {
        throw hresult_not_implemented();
    }
    void ShellIcon::IsActive(bool value) {
        throw hresult_not_implemented();
    }
    winrt::Windows::Graphics::Imaging::SoftwareBitmap ShellIcon::IconImage() {
        throw hresult_not_implemented();
    }
    void ShellIcon::IconImage(winrt::Windows::Graphics::Imaging::SoftwareBitmap const& value) {
        throw hresult_not_implemented();
    }
    winrt::Windows::UI::Xaml::Controls::MenuFlyout ShellIcon::ContextFlyout() {
        throw hresult_not_implemented();
    }
    void ShellIcon::ContextFlyout(winrt::Windows::UI::Xaml::Controls::MenuFlyout const& value) {
        throw hresult_not_implemented();
    }
}
