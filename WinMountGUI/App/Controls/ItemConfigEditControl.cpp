#include "pch.h"

#include "util.hpp"

#include "Controls\ItemConfigEditControl.h"
#include "Controls\ItemConfigEditControl_RawConfigVM.g.cpp"
#include "Controls\ItemConfigEditControl_DokanFsrvConfigVM.g.cpp"
#include "Controls\ItemConfigEditControl.g.cpp"

#include "Items\Items.h"

using namespace winrt;
using namespace Windows::UI::Xaml;
using namespace Windows::UI::Xaml::Data;
using namespace Windows::UI::Core;

namespace winrt {
    namespace muxc = Microsoft::UI::Xaml::Controls;
}

namespace winrt::WinMount::App::Controls::implementation {
    IJsonValue ItemConfigEditControl_DokanFsrvConfigVM::GetConfigData() {
        if (!m_mount_point.empty()) {
            m_jo_cfg.Insert(L"mount_point", util::winrt::to_json_value(m_mount_point));
        }
        m_jo_cfg.Insert(L"enable_sys_dirs", util::winrt::to_json_value(this->EnableSysDirs()));
        m_jo_cfg.Insert(L"readonly_drive", util::winrt::to_json_value(this->ReadonlyDrive()));
        return m_jo_cfg;
    }
    void ItemConfigEditControl_DokanFsrvConfigVM::SetConfigData(IJsonValue const& value) {
        m_jo_cfg.Clear();
        {
            std::vector<IInspectable> mount_point_cb_items;
            for (wchar_t ch = L'A'; ch <= L'Z'; ch++) {
                wchar_t buf[] = L"X:\\";
                buf[0] = ch;
                mount_point_cb_items.push_back(box_value(buf));
            }
            m_mount_point_cb_items.ReplaceAll(mount_point_cb_items);
        }
        //this->MountPointComboBox_SelectedItem(nullptr);
        m_mount_point_cb_selected_item = nullptr;
        m_mount_point = {};
        m_enable_sys_dirs = {};

        if (!value) { return; }

        auto jo = value.GetObject();
        for (auto&& i : jo) {
            auto key = i.Key();
            if (key == L"mount_point") {
                this->MountPoint(i.Value().GetString());
                for (auto&& mp : m_mount_point_cb_items) {
                    if (unbox_value<hstring>(mp) == m_mount_point) {
                        //if (m_mount_point_cb_selected_item) {
                        //    // HACK: ComboBox will reset SelectedItem, so delay this
                        //    //       operation to properly update UI
                        //    auto et = std::make_shared_for_overwrite<event_token>();
                        //    *et = this->PropertyChanged([=](auto&&, PropertyChangedEventArgs const& e) {
                        //        this->PropertyChanged(*et);
                        //        this->MountPointComboBox_SelectedItem(mp);
                        //    });
                        //}
                        //else {
                        //    //this->MountPointComboBox_SelectedItem(mp);
                        //}
                        this->MountPointComboBox_SelectedItem(mp);
                        break;
                    }
                }
            }
            else if (key == L"enable_sys_dirs") {
                this->EnableSysDirs(i.Value().GetBoolean());
            }
            else if (key == L"readonly_drive") {
                this->ReadonlyDrive(i.Value().GetBoolean());
            }
            else {
                m_jo_cfg.Insert(key, util::winrt::clone_json_value(i.Value()));
            }
        }
    }

    ItemConfigEditControl::ItemConfigEditControl() {}
    void ItemConfigEditControl::InitializeComponent() {
        ItemConfigEditControlT::InitializeComponent();

        EditorTypeNV().SelectedItem(VisualEditorNVI());
    }
    void ItemConfigEditControl::EditorTypeNV_ItemInvoked(
        IInspectable const&,
        Windows::UI::Xaml::Controls::NavigationViewItemInvokedEventArgs const& e
    ) {
        auto nvi = e.InvokedItemContainer();
        //if (nvi.IsSelected()) { return; }
        //EditorTypeNV().SelectedItem(nvi);
        this->UpdateConfigVisualState();
    }
    void ItemConfigEditControl::ConfigTypeId(guid const& value) {
        //if (m_config_type_id == value) { return; }
        m_config_vm = nullptr;
        m_config_type_id = value;
        this->UpdateConfigVisualState();
    }
    IJsonValue ItemConfigEditControl::GetConfigData() {
        if (!m_config_vm) { return nullptr; }
        return m_config_vm.GetConfigData();
    }
    void ItemConfigEditControl::SetConfigData(IJsonValue const& value) {
        if (!m_config_vm) { return; }
        try { m_config_vm.SetConfigData(value); }
        catch (...) {
            util::winrt::log_current_exception();
            this->GoToInvalidConfigVisualState();
        }
    }
    void ItemConfigEditControl::UpdateConfigVisualState() {
        auto try_set_cfg_ui_fn = [this](ItemConfigEditControl_IConfigVM const& vm, hstring const& state) {
            IJsonValue jv{ nullptr };
            try {
                //if (this->ConfigTypes().CurrentState().Name() == state) { return true; }
                if (m_config_vm && vm) { jv = m_config_vm.GetConfigData(); }
                if (jv) { vm.SetConfigData(jv); }
                m_config_vm = vm;
                Bindings->Update();
                VisualStateManager::GoToState(*this, state, true);
            }
            catch (...) {
                util::winrt::log_current_exception();
                return false;
            }
            return true;
        };
        bool succeeded{};
        if (EditorTypeNV().SelectedItem() == RawEditorNVI()) {
            succeeded = try_set_cfg_ui_fn(make<ItemConfigEditControl_RawConfigVM>(), L"RawConfigType");
        }
        else if (m_config_type_id == DOKAN_FSERVER_ID) {
            succeeded = try_set_cfg_ui_fn(make<ItemConfigEditControl_DokanFsrvConfigVM>(), L"DokanFsrvConfigType");
        }
        if (!succeeded) { this->GoToInvalidConfigVisualState(); }
    }
    void ItemConfigEditControl::GoToInvalidConfigVisualState() {
        struct StubVM : implements<StubVM, ItemConfigEditControl_IConfigVM> {
            StubVM() = default;
            IJsonValue GetConfigData() { return m_jv; }
            void SetConfigData(IJsonValue const& value) { m_jv = value; }
        private:
            IJsonValue m_jv;
        };

        IJsonValue jv{ nullptr };
        try {
            if (m_config_vm) { jv = m_config_vm.GetConfigData(); }
        }
        catch (...) { util::winrt::log_current_exception(); }

        m_config_vm = make<StubVM>();
        Bindings->Update();
        if (jv) { m_config_vm.SetConfigData(jv); }
        VisualStateManager::GoToState(*this, L"UnsupportedConfigType", true);
    }
}
