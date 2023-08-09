#pragma once

#include "util.hpp"

namespace WinMount {
    static constexpr wchar_t CLIENT_VERSION[] = L"0.1.0";

    struct WinMountClientImpl;

    struct ListFileSystemItemData {
        winrt::guid id;
        winrt::hstring name;
        winrt::guid kind_id;
        bool is_running;
    };
    struct ListFileSystemProviderItemData {
        winrt::guid id;
        winrt::hstring name;
    };
    struct ListFServerItemData {
        winrt::guid id;
        winrt::hstring name;
        winrt::guid kind_id;
        winrt::guid in_fs_id;
        bool is_running;
    };
    struct ListFServerProviderItemData {
        winrt::guid id;
        winrt::hstring name;
    };
    struct GetFileSystemInfoData {
        winrt::hstring name;
        winrt::guid kind_id;
        bool is_running;
        winrt::Windows::Data::Json::JsonValue config{ nullptr };
    };
    struct GetFServerInfoData {
        winrt::hstring name;
        winrt::guid kind_id;
        winrt::guid in_fs_id;
        bool is_running;
        winrt::Windows::Data::Json::JsonValue config{ nullptr };
    };

    // Projection of WinMountClientImpl
    struct WinMountClient {
        WinMountClient(std::nullptr_t) : m_impl(nullptr) {}
        WinMountClient(std::shared_ptr<WinMountClientImpl> impl) : m_impl(std::move(impl)) {}
        explicit operator bool() const noexcept { return m_impl.operator bool(); }
        bool operator==(WinMountClient const& rhs) const = default;
        bool operator==(std::nullptr_t) const { return m_impl == nullptr; };

        void close() const;

        winrt::hstring get_daemon_version() const;

        util::winrt::task<winrt::guid> create_fs(
            winrt::hstring const& name,
            winrt::guid const& kind_id,
            winrt::Windows::Data::Json::IJsonValue const& config
        ) const;
        util::winrt::task<> remove_fs(winrt::guid const& id) const;
        util::winrt::task<bool> start_fs(winrt::guid const& id) const;
        util::winrt::task<bool> stop_fs(winrt::guid const& id) const;
        util::winrt::task<winrt::guid> create_fsrv(
            winrt::hstring const& name,
            winrt::guid const& kind_id,
            winrt::guid const& in_fs_id,
            winrt::Windows::Data::Json::IJsonValue const& config
        ) const;
        util::winrt::task<> remove_fsrv(winrt::guid const& id) const;
        util::winrt::task<bool> start_fsrv(winrt::guid const& id) const;
        util::winrt::task<bool> stop_fsrv(winrt::guid const& id) const;
        util::winrt::task<std::vector<ListFileSystemItemData>> list_fs() const;
        util::winrt::task<std::vector<ListFileSystemProviderItemData>> list_fsp() const;
        util::winrt::task<std::vector<ListFServerItemData>> list_fsrv() const;
        util::winrt::task<std::vector<ListFServerProviderItemData>> list_fsrvp() const;
        util::winrt::task<GetFileSystemInfoData> get_fs_info(winrt::guid const& id) const;
        util::winrt::task<GetFServerInfoData> get_fsrv_info(winrt::guid const& id) const;
        util::winrt::task<> update_fs_info(
            winrt::guid const& id,
            winrt::hstring const& name,
            winrt::Windows::Data::Json::IJsonValue const& config
        ) const;
        util::winrt::task<> update_fsrv_info(
            winrt::guid const& id,
            winrt::hstring const& name,
            winrt::Windows::Data::Json::IJsonValue const& config
        ) const;

    private:
        std::shared_ptr<WinMountClientImpl> m_impl;
    };

    util::winrt::task<WinMountClient> connect_winmount_client(winrt::hstring const& url);
}
