#pragma once

#include "Pages\FsItem.g.h"
#include "Pages\FspItem.g.h"
#include "Pages\FsrvItem.g.h"
#include "Pages\FsrvpItem.g.h"
#include "Pages\MainViewModel.g.h"

#include "WinMountClient.hpp"
#include "util.hpp"

static constexpr winrt::guid GLOBAL_FS_LOCALFS_ID{ "96DD6C88-CDB5-4446-8269-104F2DD82ACD" };

namespace winrt::WinMount::App::Pages::implementation {
    struct FspItem : FspItemT<FspItem> {
        FspItem(::WinMount::ListFileSystemProviderItemData const& data) : m_data(data) {}
        guid Id() { return m_data.id; }
        hstring Name() { return m_data.name; }
        SemVersion Version() { return { m_data.version[0], m_data.version[1], m_data.version[2] }; }
        Windows::Data::Json::JsonValue TemplateConfig() { return m_data.template_config; }
        bool IsHidden() { return m_data.is_hidden; }
    private:
        ::WinMount::ListFileSystemProviderItemData m_data;
    };

    struct FsItem : FsItemT<FsItem> {
        using PropertyChangedEventArgs = Windows::UI::Xaml::Data::PropertyChangedEventArgs;

        FsItem(::WinMount::ListFileSystemItemData const& data, hstring const& kind_disp_name) :
            m_data(data), m_kind_disp_name(kind_disp_name) {}

        event_token PropertyChanged(Windows::UI::Xaml::Data::PropertyChangedEventHandler const& handler) {
            return m_PropertyChanged.add(handler);
        }
        void PropertyChanged(event_token const& token) noexcept {
            m_PropertyChanged.remove(token);
        }

        guid Id() { return m_data.id; }
        hstring Name() { return m_data.name; }
        guid KindId() { return m_data.kind_id; }
        hstring KindDisplayName() { return m_kind_disp_name; }
        bool IsRunning() { return m_data.is_running; }
        void IsRunning(bool value) {
            if (m_data.is_running != value) {
                m_data.is_running = value;
                m_PropertyChanged(*this, PropertyChangedEventArgs{ L"IsRunning" });
                m_PropertyChanged(*this, PropertyChangedEventArgs{ L"StartStopButton_Text" });
            }
        }
        bool IsGlobal() { return m_data.is_global; }
        hstring StartStopButton_Text() {
            // 0xE768: Play, 0xE71A: Stop
            static constexpr wchar_t STR_PLAY[] = L"\xE768", STR_STOP[] = L"\xE71A";
            return this->IsRunning() ? STR_STOP : STR_PLAY;
        }

    private:
        event<Windows::UI::Xaml::Data::PropertyChangedEventHandler> m_PropertyChanged;

        ::WinMount::ListFileSystemItemData m_data;
        hstring m_kind_disp_name;
    };

    struct FsrvpItem : FsrvpItemT<FsrvpItem> {
        FsrvpItem(::WinMount::ListFServerProviderItemData const& data) : m_data(data) {}
        guid Id() { return m_data.id; }
        hstring Name() { return m_data.name; }
        SemVersion Version() { return { m_data.version[0], m_data.version[1], m_data.version[2] }; }
        Windows::Data::Json::JsonValue TemplateConfig() { return m_data.template_config; }
    private:
        ::WinMount::ListFServerProviderItemData m_data;
    };

    struct FsrvItem : FsrvItemT<FsrvItem> {
        using PropertyChangedEventArgs = Windows::UI::Xaml::Data::PropertyChangedEventArgs;

        FsrvItem(::WinMount::ListFServerItemData const& data,
            hstring const& kind_disp_name,
            hstring const& in_fs_disp_name
        ) : m_data(data), m_kind_disp_name(kind_disp_name), m_in_fs_disp_name(in_fs_disp_name) {}

        event_token PropertyChanged(Windows::UI::Xaml::Data::PropertyChangedEventHandler const& handler) {
            return m_PropertyChanged.add(handler);
        }
        void PropertyChanged(event_token const& token) noexcept {
            m_PropertyChanged.remove(token);
        }

        guid Id() { return m_data.id; }
        hstring Name() { return m_data.name; }
        guid KindId() { return m_data.kind_id; }
        hstring KindDisplayName() { return m_kind_disp_name; }
        guid InputFsId() { return m_data.in_fs_id; }
        hstring InputFsDisplayName() { return m_in_fs_disp_name; }
        bool IsRunning() { return m_data.is_running; }
        void IsRunning(bool value) {
            if (m_data.is_running != value) {
                m_data.is_running = value;
                m_PropertyChanged(*this, PropertyChangedEventArgs{ L"IsRunning" });
            }
        }

    private:
        event<Windows::UI::Xaml::Data::PropertyChangedEventHandler> m_PropertyChanged;

        ::WinMount::ListFServerItemData m_data;
        hstring m_kind_disp_name;
        hstring m_in_fs_disp_name;
    };
}

namespace winrt::WinMount::App::Pages::implementation {
    struct MainViewModel : MainViewModelT<MainViewModel> {
        using IGenericObservableVector = Windows::Foundation::Collections::IObservableVector<
            Windows::Foundation::IInspectable>;
        using GenericQueryObservableVector = util::winrt::QueryObservableVector<
            Windows::Foundation::IInspectable>;

        MainViewModel(::WinMount::WinMountClient const& client);

        IGenericObservableVector FspItems() { return m_fsp_items; }
        IGenericObservableVector FsItems() { return m_fs_items; }
        IGenericObservableVector FsrvpItems() { return m_fsrvp_items; }
        IGenericObservableVector FsrvItems() { return m_fsrv_items; }

        IGenericObservableVector FspItemsNoHidden() { return *m_fsp_items_no_hidden; }
        IGenericObservableVector FsItemsNoGlobal() { return *m_fs_items_no_global; }

        Windows::Foundation::IAsyncAction ReloadFsItemsAsync();
        Windows::Foundation::IAsyncAction ReloadFsrvItemsAsync();
        hstring GetFspNameFromId(guid const& id);
        hstring GetFsNameFromId(guid const& id);
        hstring GetFsrvpNameFromId(guid const& id);

        auto const& GetClient() { return m_client; }

    private:
        friend MainFsPage;
        friend MainFsrvPage;
        friend MainSettingsPage;
        friend MainAboutPage;

        util::winrt::task<> ReloadFsItemsAsyncInner();
        util::winrt::task<> ReloadFsrvItemsAsyncInner();

        util::winrt::typed_task_storage<> m_task_reload_fs_items, m_task_reload_fsrv_items;

        ::WinMount::WinMountClient m_client{ nullptr };

        std::vector<::WinMount::ListFileSystemProviderItemData> m_fsp_list;
        std::vector<::WinMount::ListFileSystemItemData> m_fs_list;
        std::vector<::WinMount::ListFServerProviderItemData> m_fsrvp_list;

        IGenericObservableVector m_fsp_items{ util::winrt::make_stovi() };
        IGenericObservableVector m_fs_items{ util::winrt::make_stovi() };
        IGenericObservableVector m_fsrvp_items{ util::winrt::make_stovi() };
        IGenericObservableVector m_fsrv_items{ util::winrt::make_stovi() };

        com_ptr<GenericQueryObservableVector> m_fsp_items_no_hidden{ nullptr };
        com_ptr<GenericQueryObservableVector> m_fs_items_no_global{ nullptr };
    };
}
