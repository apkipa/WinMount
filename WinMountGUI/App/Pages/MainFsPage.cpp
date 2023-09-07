#include "pch.h"
#include "Pages\MainFsPage.h"
#if __has_include("Pages\MainFsPage.g.cpp")
#include "Pages\MainFsPage.g.cpp"
#endif

#include "WinMountClient.hpp"

#include "Items\Items.h"

using namespace winrt;
using namespace Windows::Foundation;
using namespace Windows::UI;
using namespace Windows::UI::Xaml;
using namespace Windows::UI::Xaml::Navigation;
using namespace Windows::UI::Xaml::Controls;
using namespace Windows::UI::Xaml::Input;
using namespace Windows::Data::Json;

namespace winrt::WinMount::App::Pages::implementation {
    MainFsPage::MainFsPage() {}
    MainFsPage::~MainFsPage() {
        this->FsListView().ItemsSource(nullptr);
        this->DetailsAddNew_FsTypeComboBox().ItemsSource(nullptr);
    }
    void MainFsPage::OnNavigatedTo(NavigationEventArgs const& e) {
        m_parent = e.Parameter().as<MainPage>().get();

        m_async.cancel_and_run(&MainFsPage::ReloadFsListAsync, this);
        // Prevent items display duplication bug
        util::winrt::run_when_loaded([this](auto&&) {
            auto vm = m_parent->ViewModel();
            this->FsListView().ItemsSource(vm.FsItemsNoGlobal());
            this->DetailsAddNew_FsTypeComboBox().ItemsSource(vm.FspItemsNoHidden());
        }, this);

        util::winrt::fix_scroll_viewer_focus(this->DetailsAddNewRoot().Children().GetAt(0).as<ScrollViewer>());
        util::winrt::fix_scroll_viewer_focus(this->DetailsEditCurrentRoot().Children().GetAt(0).as<ScrollViewer>());
    }
    void MainFsPage::AddNewFsButton_Click(IInspectable const&, RoutedEventArgs const&) {
        this->FsListView().SelectedItem(nullptr);
        this->DetailsAddNew_FsNameTextBox().Text({});
        this->DetailsAddNew_FsTypeComboBox().SelectedIndex(-1);
        auto cfg_ctrl = this->DetailsAddNew_FsConfigEditCtrl();
        cfg_ctrl.ConfigTypeId({});
        cfg_ctrl.SetConfigData(nullptr);
        VisualStateManager::GoToState(*this, L"AddNewFsItem", true);
    }
    void MainFsPage::ReloadFsListButton_Click(IInspectable const&, RoutedEventArgs const&) {
        m_async.cancel_and_run(&MainFsPage::ReloadFsListAsync, this);
    }
    void MainFsPage::FsItem_StartStopButton_Click(IInspectable const& sender, RoutedEventArgs const&) {
        auto fs_item = sender.as<FrameworkElement>().DataContext().as<Items::implementation::FsItem>();
        if (fs_item->IsRunning()) {
            m_async.cancel_and_run([](MainFsPage* that, com_ptr<Items::implementation::FsItem> fs_item) -> IAsyncAction {
                auto cancellation_token = co_await get_cancellation_token();
                cancellation_token.enable_propagation();

                auto new_stopped = co_await that->StopFsAsync(fs_item->Id());
                fs_item->IsRunning(false);
            }, this, std::move(fs_item));
        }
        else {
            m_async.cancel_and_run([](MainFsPage* that, com_ptr<Items::implementation::FsItem> fs_item) -> IAsyncAction {
                auto cancellation_token = co_await get_cancellation_token();
                cancellation_token.enable_propagation();

                auto new_started = co_await that->StartFsAsync(fs_item->Id());
                fs_item->IsRunning(true);
            }, this, std::move(fs_item));
        }
    }
    void MainFsPage::FsListView_SelectionChanged(IInspectable const&, SelectionChangedEventArgs const& e) {
        auto added_items = e.AddedItems();
        if (added_items.Size() == 0) {
            VisualStateManager::GoToState(*this, L"Empty", true);
        }
        else {
            auto fs_item = e.AddedItems().GetAt(0).as<Items::implementation::FsItem>();
            m_async.cancel_and_run([](MainFsPage* that, guid const& id) -> IAsyncAction {
                auto cancellation_token = co_await get_cancellation_token();
                cancellation_token.enable_propagation();

                auto const& client = that->m_parent->m_client;
                auto vm = that->m_parent->ViewModel();

                auto fs_info = co_await client.get_fs_info(id);
                that->DetailsEditCurrent_FsNameTextBox().Text(fs_info.name);
                {
                    auto fs_type_cb = that->DetailsEditCurrent_FsTypeComboBox();
                    fs_type_cb.Items().ReplaceAll({ box_value(vm.GetFspNameFromId(fs_info.kind_id)) });
                    fs_type_cb.SelectedIndex(0);
                }
                auto cfg_ctrl = that->DetailsEditCurrent_FsConfigEditCtrl();
                cfg_ctrl.ConfigTypeId(fs_info.kind_id);
                cfg_ctrl.SetConfigData(fs_info.config);
                VisualStateManager::GoToState(*that, L"EditFsItem", true);
            }, this, fs_item->Id());
        }
    }
    void MainFsPage::DetailsAddNew_FsTypeComboBox_SelectionChanged(
        IInspectable const&, SelectionChangedEventArgs const& e
    ) {
        auto added_items = e.AddedItems();
        if (added_items.Size() == 0) { return; }
        auto item = added_items.GetAt(0).as<Items::implementation::FspItem>();
        auto cfg_ctrl = DetailsAddNew_FsConfigEditCtrl();
        cfg_ctrl.ConfigTypeId(item->Id());
        cfg_ctrl.SetConfigData(item->TemplateConfig());
    }
    void MainFsPage::DetailsAddNew_CreateButton_Click(IInspectable const&, RoutedEventArgs const&) {
        // TODO: Verify input
        m_async.cancel_and_run([](MainFsPage* that) -> IAsyncAction {
            auto cancellation_token = co_await get_cancellation_token();
            cancellation_token.enable_propagation();

            auto const& client = that->m_parent->m_client;

            auto name = that->DetailsAddNew_FsNameTextBox().Text();
            auto fsp_item = that->DetailsAddNew_FsTypeComboBox().SelectedItem().try_as<Items::implementation::FspItem>();
            if (!fsp_item) {
                throw hresult_error(E_FAIL, L"invalid filesystem provider selection");
            }
            auto kind_id = fsp_item->Id();
            auto config = that->DetailsAddNew_FsConfigEditCtrl().GetConfigData();
            auto fs_id = co_await client.create_fs(name, kind_id, config);

            co_await that->ReloadFsListAsync();
        }, this);
    }
    void MainFsPage::DetailsEditCurrent_DeleteButton_Click(IInspectable const&, RoutedEventArgs const&) {
        m_async.cancel_and_run([](MainFsPage* that) -> IAsyncAction {
            auto cancellation_token = co_await get_cancellation_token();
            cancellation_token.enable_propagation();

            /*that->IsHitTestVisible(false);
            deferred([&] {that->IsHitTestVisible(true); });*/
            auto const& client = that->m_parent->m_client;
            auto fs_item = that->FsListView().SelectedItem().as<Items::implementation::FsItem>();
            ContentDialog cd;
            cd.XamlRoot(that->XamlRoot());
            cd.Title(box_value(L"Delete This Filesystem?"));
            cd.Content(box_value(std::format(L""
                "All data of `{}` will be removed permanently and cannot be undone.",
                fs_item->Name()
            )));
            cd.PrimaryButtonText(L"Yes, delete");
            cd.CloseButtonText(L"No");
            //cd.DefaultButton(ContentDialogButton::Primary);
            auto result = co_await cd.ShowAsync();
            if (result == ContentDialogResult::Primary) {
                co_await client.remove_fs(fs_item->Id());
                co_await that->ReloadFsListAsync();
            }
        }, this);
    }
    void MainFsPage::DetailsEditCurrent_CommitButton_Click(IInspectable const&, RoutedEventArgs const&) {
        // TODO: Verify input
        m_async.cancel_and_run([](MainFsPage* that) -> IAsyncAction {
            auto cancellation_token = co_await get_cancellation_token();
            cancellation_token.enable_propagation();

            auto const& client = that->m_parent->m_client;

            auto fs_item = that->FsListView().SelectedItem().as<Items::implementation::FsItem>();
            auto id = fs_item->Id();
            auto name = that->DetailsEditCurrent_FsNameTextBox().Text();
            auto config = that->DetailsEditCurrent_FsConfigEditCtrl().GetConfigData();
            co_await client.update_fs_info(id, name, config);

            co_await that->ReloadFsListAsync();
            that->SelectFsItemById(id);
        }, this);
    }
    IAsyncAction MainFsPage::ReloadFsListAsync() {
        auto cancellation_token = co_await get_cancellation_token();
        cancellation_token.enable_propagation();

        VisualStateManager::GoToState(*this, L"Empty", true);
        co_await m_parent->ViewModel().ReloadFsItemsAsync();
    }
    bool MainFsPage::SelectFsItemById(guid const& id) {
        auto fs_list_view = this->FsListView();
        auto fs_items = fs_list_view.ItemsSource().as<Items::implementation::MainViewModel::IGenericObservableVector>();
        uint32_t size = fs_items.Size();
        for (uint32_t i = 0; i < size; i++) {
            auto fs_item = fs_items.GetAt(i).as<Items::implementation::FsItem>();
            if (fs_item->Id() == id) {
                fs_list_view.SelectedIndex(static_cast<int32_t>(i));
                return true;
            }
        }
        return false;
    }
    IAsyncOperation<bool> MainFsPage::StartFsAsync(guid id) {
        auto cancellation_token = co_await get_cancellation_token();
        cancellation_token.enable_propagation();

        auto const& client = m_parent->m_client;

        co_return co_await client.start_fs(id);
    }
    IAsyncOperation<bool> MainFsPage::StopFsAsync(guid id) {
        auto cancellation_token = co_await get_cancellation_token();
        cancellation_token.enable_propagation();

        auto const& client = m_parent->m_client;

        co_return co_await client.stop_fs(id);
    }
    void MainFsPage::StartFs(guid const& id) {
        m_async.cancel_and_run(&MainFsPage::StartFsAsync, this, id);
    }
    void MainFsPage::StopFs(guid const& id) {
        m_async.cancel_and_run(&MainFsPage::StopFsAsync, this, id);
    }
}
