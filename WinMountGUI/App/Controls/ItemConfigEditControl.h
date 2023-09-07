#pragma once

#include "util.hpp"

#include "Controls\ItemConfigEditControl_RawConfigVM.g.h"
#include "Controls\ItemConfigEditControl_DokanFsrvConfigVM.g.h"
#include "Controls\ItemConfigEditControl.g.h"

namespace winrt::WinMount::App::Controls::implementation {
    using namespace Windows::Data::Json;
    using Windows::UI::Xaml::Data::PropertyChangedEventArgs;

    struct ItemConfigEditControl_RawConfigVM : ItemConfigEditControl_RawConfigVMT<ItemConfigEditControl_RawConfigVM> {
        ItemConfigEditControl_RawConfigVM() = default;

        event_token PropertyChanged(Windows::UI::Xaml::Data::PropertyChangedEventHandler const& handler) {
            return m_PropertyChanged.add(handler);
        }
        void PropertyChanged(event_token const& token) noexcept {
            m_PropertyChanged.remove(token);
        }

        IJsonValue GetConfigData() {
            if (m_raw_string.empty()) { return nullptr; }
            JsonValue jv{ nullptr };
            if (!JsonValue::TryParse(m_raw_string, jv)) { return nullptr; }
            return jv;
        }
        void SetConfigData(IJsonValue const& value) {
            if (!value) { this->RawString({}); return; }
            this->RawString(value.Stringify());
        }

        hstring RawString() { return m_raw_string; }
        void RawString(hstring const& value) {
            if (m_raw_string == value) { return; }
            m_raw_string = value;
            m_PropertyChanged(*this, PropertyChangedEventArgs{ L"RawString" });
        }

    private:
        event<Windows::UI::Xaml::Data::PropertyChangedEventHandler> m_PropertyChanged;

        hstring m_raw_string{};
    };

    struct ItemConfigEditControl_DokanFsrvConfigVM : ItemConfigEditControl_DokanFsrvConfigVMT<ItemConfigEditControl_DokanFsrvConfigVM> {
        ItemConfigEditControl_DokanFsrvConfigVM() = default;

        event_token PropertyChanged(Windows::UI::Xaml::Data::PropertyChangedEventHandler const& handler) {
            return m_PropertyChanged.add(handler);
        }
        void PropertyChanged(event_token const& token) noexcept {
            m_PropertyChanged.remove(token);
        }

        IJsonValue GetConfigData();
        void SetConfigData(IJsonValue const& value);

        Windows::Foundation::Collections::IObservableVector<Windows::Foundation::IInspectable> MountPointComboBox_Items() {
            return m_mount_point_cb_items;
        }
        /*void MountPointComboBox_Items(Windows::Foundation::Collections::IObservableVector<Windows::Foundation::IInspectable> const& value) {
            if (m_mount_point_cb_items == value) { return; }
            m_mount_point_cb_items = value;
            m_PropertyChanged(*this, PropertyChangedEventArgs{ L"MountPointComboBox_Items" });
        }*/
        Windows::Foundation::IInspectable MountPointComboBox_SelectedItem() {
            return m_mount_point_cb_selected_item;
        }
        void MountPointComboBox_SelectedItem(Windows::Foundation::IInspectable const& value) {
            if (m_mount_point_cb_selected_item == value) { return; }
            // HACK: Reject ComboBox resetting SelectedItem
            if (value) {
                m_mount_point_cb_selected_item = value;
            }
            m_PropertyChanged(*this, PropertyChangedEventArgs{ L"MountPointComboBox_SelectedItem" });
            this->MountPoint(unbox_value<hstring>(m_mount_point_cb_selected_item));
        }
        hstring MountPoint() { return m_mount_point; }
        void MountPoint(hstring const& value) {
            if (m_mount_point == value) { return; }
            m_mount_point = value;
            m_PropertyChanged(*this, PropertyChangedEventArgs{ L"MountPoint" });
        }
        bool EnableSysDirs() { return m_enable_sys_dirs; }
        void EnableSysDirs(bool value) {
            if (m_enable_sys_dirs == value) { return; }
            m_enable_sys_dirs = value;
            m_PropertyChanged(*this, PropertyChangedEventArgs{ L"EnableSysDirs" });
        }
        bool ReadonlyDrive() { return m_readonly_drive; }
        void ReadonlyDrive(bool value) {
            if (m_readonly_drive == value) { return; }
            m_readonly_drive = value;
            m_PropertyChanged(*this, PropertyChangedEventArgs{ L"ReadonlyDrive" });
        }

    private:
        event<Windows::UI::Xaml::Data::PropertyChangedEventHandler> m_PropertyChanged;

        JsonObject m_jo_cfg;

        Windows::Foundation::Collections::IObservableVector<Windows::Foundation::IInspectable> m_mount_point_cb_items{
            util::winrt::make_stovi() };
        Windows::Foundation::IInspectable m_mount_point_cb_selected_item{ nullptr };
        hstring m_mount_point{};
        bool m_enable_sys_dirs{};
        bool m_readonly_drive{};
    };

    struct ItemConfigEditControl : ItemConfigEditControlT<ItemConfigEditControl> {
        ItemConfigEditControl();

        void InitializeComponent();

        void EditorTypeNV_ItemInvoked(
            Windows::Foundation::IInspectable const&,
            Windows::UI::Xaml::Controls::NavigationViewItemInvokedEventArgs const& e
        );

        guid ConfigTypeId() { return m_config_type_id; }
        void ConfigTypeId(guid const& value);

        IJsonValue GetConfigData();
        void SetConfigData(IJsonValue const& value);

        ItemConfigEditControl_IConfigVM ConfigVM() { return m_config_vm; }

    private:
        void UpdateConfigVisualState();
        void GoToInvalidConfigVisualState();

        guid m_config_type_id{};
        ItemConfigEditControl_IConfigVM m_config_vm{ nullptr };
    };
}

namespace winrt::WinMount::App::Controls::factory_implementation {
    struct ItemConfigEditControl : ItemConfigEditControlT<ItemConfigEditControl, implementation::ItemConfigEditControl> {};
}
