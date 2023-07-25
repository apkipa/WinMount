#pragma once

#include "App.g.h"

namespace winrt::WinMount::App::implementation {
    template <typename D, typename... I>
    struct App_baseWithProvider : public App_base<D, ::winrt::Windows::UI::Xaml::Markup::IXamlMetadataProvider> {
        using IXamlType = ::winrt::Windows::UI::Xaml::Markup::IXamlType;
        IXamlType GetXamlType(::winrt::Windows::UI::Xaml::Interop::TypeName const& type) {
            return _appProvider.GetXamlType(type);
        }
        IXamlType GetXamlType(::winrt::hstring const& fullName) {
            return _appProvider.GetXamlType(fullName);
        }
        ::winrt::com_array<::winrt::Windows::UI::Xaml::Markup::XmlnsDefinition> GetXmlnsDefinitions() {
            return _appProvider.GetXmlnsDefinitions();
        }
    private:
        bool _contentLoaded{ false };
        winrt::WinMount::App::XamlMetaDataProvider _appProvider;
    };
    template <typename D, typename... I>
    using AppT2 = App_baseWithProvider<D, I...>;
}

namespace winrt::WinMount::App::implementation {
    struct App : AppT2<App> {
        App();
        ~App();
        void OnLaunched(Windows::ApplicationModel::Activation::LaunchActivatedEventArgs const&);
        void OnSuspending(IInspectable const&, Windows::ApplicationModel::SuspendingEventArgs const&);
        void OnNavigationFailed(IInspectable const&, Windows::UI::Xaml::Navigation::NavigationFailedEventArgs const&);
    };
}

namespace winrt::WinMount::App::factory_implementation {
    struct App : AppT<App, implementation::App> {};
}
