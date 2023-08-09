#include "pch.h"

#include "Pages\Items.h"
#include "Pages\FsItem.g.cpp"
#include "Pages\FspItem.g.cpp"
#include "Pages\MainViewModel.g.cpp"

using namespace winrt;
using namespace winrt::Windows::Foundation;

namespace winrt::WinMount::App::Pages::implementation {
    MainViewModel::MainViewModel(::WinMount::WinMountClient const& client) : m_client(client) {}
    hstring MainViewModel::GetFspNameFromId(guid const& id) {
        for (auto const& fsp : m_fsp_list) {
            if (id == fsp.id) {
                return fsp.name;
            }
        }
        return hstring{ L"<" + util::winrt::to_wstring(id) + L">" };
    }
    hstring MainViewModel::GetFsNameFromId(guid const& id) {
        for (auto&& fs : m_fs_items) {
            auto fs_item = fs.as<FsItem>();
            if (fs_item->Id() == id) {
                return fs_item->Name();
            }
        }
        return hstring{ L"<" + util::winrt::to_wstring(id) + L">" };
    }
    hstring MainViewModel::GetFsrvpNameFromId(guid const& id) {
        for (auto const& fsrvp : m_fsrvp_list) {
            if (id == fsrvp.id) {
                return fsrvp.name;
            }
        }
        return hstring{ L"<" + util::winrt::to_wstring(id) + L">" };
    }
    IAsyncAction MainViewModel::ReloadFsItemsAsync() {
        auto cancellation_token = co_await get_cancellation_token();
        cancellation_token.enable_propagation();

        auto strong_this = get_strong();

        co_await m_task_reload_fs_items.run_if_idle(&MainViewModel::ReloadFsItemsAsyncInner, this);
    }
    IAsyncAction MainViewModel::ReloadFsrvItemsAsync() {
        auto cancellation_token = co_await get_cancellation_token();
        cancellation_token.enable_propagation();

        auto strong_this = get_strong();

        co_await m_task_reload_fs_items.run_if_idle(&MainViewModel::ReloadFsrvItemsAsyncInner, this);
    }
    util::winrt::task<> MainViewModel::ReloadFsItemsAsyncInner() {
        auto cancellation_token = co_await get_cancellation_token();
        cancellation_token.enable_propagation();

        m_fs_items.Clear();
        m_fs_list.clear();

        // Load filesystem providers first
        if (m_fsp_list.empty()) {
            m_fsp_list = co_await m_client.list_fsp();
            for (auto const& i : m_fsp_list) {
                m_fsp_items.Append(make<FspItem>(i));
            }
        }

        m_fs_list = co_await m_client.list_fs();
        for (auto const& i : m_fs_list) {
            /*
            hstring kind_disp_name{};
            for (auto const& fsp : m_fsp_list) {
                if (i.kind_id == fsp.id) {
                    kind_disp_name = fsp.name;
                    break;
                }
            }
            if (kind_disp_name.empty()) {
                kind_disp_name = L"<" + util::winrt::to_wstring(i.kind_id) + L">";
            }
            m_fs_items.Append(make<FsItem>(i, kind_disp_name));
            */
            m_fs_items.Append(make<FsItem>(i, this->GetFspNameFromId(i.kind_id)));
        }
    }
    util::winrt::task<> MainViewModel::ReloadFsrvItemsAsyncInner() {
        auto cancellation_token = co_await get_cancellation_token();
        cancellation_token.enable_propagation();

        m_fsrv_items.Clear();

        // Load filesystem server providers first
        if (m_fsrvp_list.empty()) {
            m_fsrvp_list = co_await m_client.list_fsrvp();
            for (auto const& i : m_fsrvp_list) {
                m_fsrvp_items.Append(make<FsrvpItem>(i));
            }
        }

        // Also load filesystems
        if (m_fs_items.Size() == 0) {
            co_await m_task_reload_fs_items.run_if_idle(&MainViewModel::ReloadFsItemsAsyncInner, this);
        }

        auto fsrv_list = co_await m_client.list_fsrv();
        for (auto const& i : fsrv_list) {
            m_fsrv_items.Append(make<FsrvItem>(i,
                this->GetFsrvpNameFromId(i.kind_id),
                this->GetFsNameFromId(i.in_fs_id)
            ));
        }
    }
}
