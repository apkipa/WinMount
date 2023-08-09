#include "pch.h"
#include "Pages\MainFsrvPage.h"
#if __has_include("Pages\MainFsrvPage.g.cpp")
#include "Pages\MainFsrvPage.g.cpp"
#endif

#include "WinMountClient.hpp"

using namespace winrt;
using namespace Windows::Foundation;
using namespace Windows::UI;
using namespace Windows::UI::Xaml;
using namespace Windows::UI::Xaml::Navigation;
using namespace Windows::UI::Xaml::Controls;
using namespace Windows::UI::Xaml::Input;
using namespace Windows::Data::Json;

namespace winrt::WinMount::App::Pages::implementation {
    MainFsrvPage::MainFsrvPage() {}
    MainFsrvPage::~MainFsrvPage() {
        this->FsrvListView().ItemsSource(nullptr);
        this->DetailsAddNew_FsrvTypeComboBox().ItemsSource(nullptr);
        this->DetailsAddNew_InputFsComboBox().ItemsSource(nullptr);
    }
    void MainFsrvPage::OnNavigatedTo(NavigationEventArgs const& e) {
        m_main_vm = e.Parameter().as<MainPage>().ViewModel().as<MainViewModel>();

        m_async.cancel_and_run(&MainFsrvPage::ReloadFsrvListAsync, this);
        // Prevent items display duplication bug
        util::winrt::run_when_loaded([this](auto&&) {
            this->FsrvListView().ItemsSource(m_main_vm->FsrvItems());
            this->DetailsAddNew_InputFsComboBox().ItemsSource(m_main_vm->FsItems());
            this->DetailsAddNew_FsrvTypeComboBox().ItemsSource(m_main_vm->FsrvpItems());
        }, this);

        util::winrt::fix_scroll_viewer_focus(this->DetailsAddNewRoot().Children().GetAt(0).as<ScrollViewer>());
        util::winrt::fix_scroll_viewer_focus(this->DetailsEditCurrentRoot().Children().GetAt(0).as<ScrollViewer>());
    }
    void MainFsrvPage::AddNewFsrvButton_Click(IInspectable const&, RoutedEventArgs const&) {
        this->FsrvListView().SelectedItem(nullptr);
        this->DetailsAddNew_FsrvNameTextBox().Text({});
        this->DetailsAddNew_InputFsComboBox().SelectedIndex(-1);
        this->DetailsAddNew_FsrvTypeComboBox().SelectedIndex(-1);
        this->DetailsAddNew_FsrvConfigTextBox().Text({});
        VisualStateManager::GoToState(*this, L"AddNewFsrvItem", true);
    }
    void MainFsrvPage::ReloadFsrvListButton_Click(IInspectable const&, RoutedEventArgs const&) {
        m_async.cancel_and_run(&MainFsrvPage::ReloadFsrvListAsync, this);
    }
    void MainFsrvPage::FsrvItem_StartStopButton_Click(IInspectable const& sender, RoutedEventArgs const&) {
        auto fsrv_item = sender.as<FrameworkElement>().DataContext().as<FsrvItem>();
        if (fsrv_item->IsRunning()) {
            m_async.cancel_and_run([](MainFsrvPage* that, com_ptr<FsrvItem> fsrv_item) -> IAsyncAction {
                auto cancellation_token = co_await get_cancellation_token();
                cancellation_token.enable_propagation();

                auto new_stopped = co_await that->StopFsrvAsync(fsrv_item->Id());
                fsrv_item->IsRunning(false);
            }, this, std::move(fsrv_item));
        }
        else {
            m_async.cancel_and_run([](MainFsrvPage* that, com_ptr<FsrvItem> fsrv_item) -> IAsyncAction {
                auto cancellation_token = co_await get_cancellation_token();
                cancellation_token.enable_propagation();

                auto new_started = co_await that->StartFsrvAsync(fsrv_item->Id());
                fsrv_item->IsRunning(true);
            }, this, std::move(fsrv_item));
        }
    }
    void MainFsrvPage::FsrvListView_SelectionChanged(IInspectable const&, SelectionChangedEventArgs const& e) {
        auto added_items = e.AddedItems();
        if (added_items.Size() == 0) {
            VisualStateManager::GoToState(*this, L"Empty", true);
        }
        else {
            auto fsrv_item = e.AddedItems().GetAt(0).as<FsrvItem>();
            m_async.cancel_and_run([](MainFsrvPage* that, guid const& id) -> IAsyncAction {
                auto cancellation_token = co_await get_cancellation_token();
                cancellation_token.enable_propagation();

                auto const& vm = that->m_main_vm;
                auto client = vm->GetClient();

                auto fsrv_info = co_await client.get_fsrv_info(id);
                that->DetailsEditCurrent_FsrvNameTextBox().Text(fsrv_info.name);
                {
                    auto in_fs_cb = that->DetailsEditCurrent_InputFsComboBox();
                    in_fs_cb.Items().ReplaceAll({ box_value(vm->GetFsNameFromId(fsrv_info.in_fs_id)) });
                    in_fs_cb.SelectedIndex(0);
                }
                {
                    auto fsrv_type_cb = that->DetailsEditCurrent_FsrvTypeComboBox();
                    fsrv_type_cb.Items().ReplaceAll({ box_value(vm->GetFsrvpNameFromId(fsrv_info.kind_id)) });
                    fsrv_type_cb.SelectedIndex(0);
                }
                that->DetailsEditCurrent_FsrvConfigTextBox().Text(fsrv_info.config.ToString());
                VisualStateManager::GoToState(*that, L"EditFsrvItem", true);
            }, this, fsrv_item->Id());
        }
    }
    void MainFsrvPage::DetailsAddNew_CreateButton_Click(IInspectable const&, RoutedEventArgs const&) {
        // TODO: Verify input
        m_async.cancel_and_run([](MainFsrvPage* that) -> IAsyncAction {
            auto cancellation_token = co_await get_cancellation_token();
            cancellation_token.enable_propagation();

            auto client = that->m_main_vm->GetClient();

            auto name = that->DetailsAddNew_FsrvNameTextBox().Text();
            auto in_fs_item = that->DetailsAddNew_InputFsComboBox().SelectedItem().try_as<FsItem>();
            if (!in_fs_item) {
                throw hresult_error(E_FAIL, L"invalid input filesystem selection");
            }
            auto fsrvp_item = that->DetailsAddNew_FsrvTypeComboBox().SelectedItem().try_as<FsrvpItem>();
            if (!fsrvp_item) {
                throw hresult_error(E_FAIL, L"invalid filesystem server provider selection");
            }
            auto kind_id = fsrvp_item->Id();
            JsonValue config{ nullptr };
            JsonValue::TryParse(that->DetailsAddNew_FsrvConfigTextBox().Text(), config);
            auto fsrv_id = co_await client.create_fsrv(name, kind_id, in_fs_item->Id(), config);

            co_await that->ReloadFsrvListAsync();
        }, this);
    }
    void MainFsrvPage::DetailsEditCurrent_DeleteButton_Click(IInspectable const&, RoutedEventArgs const&) {
        m_async.cancel_and_run([](MainFsrvPage* that) -> IAsyncAction {
            auto cancellation_token = co_await get_cancellation_token();
            cancellation_token.enable_propagation();

            /*that->IsHitTestVisible(false);
            deferred([&] {that->IsHitTestVisible(true); });*/
            auto client = that->m_main_vm->GetClient();
            auto fsrv_item = that->FsrvListView().SelectedItem().as<FsrvItem>();
            ContentDialog cd;
            cd.XamlRoot(that->XamlRoot());
            cd.Title(box_value(L"Delete This Filesystem Server?"));
            cd.Content(box_value(std::format(L""
                "All data of `{}` will be removed permanently and cannot be undone.",
                fsrv_item->Name()
            )));
            cd.PrimaryButtonText(L"Yes, delete");
            cd.CloseButtonText(L"No");
            //cd.DefaultButton(ContentDialogButton::Primary);
            auto result = co_await cd.ShowAsync();
            if (result == ContentDialogResult::Primary) {
                co_await client.remove_fsrv(fsrv_item->Id());
                co_await that->ReloadFsrvListAsync();
            }
        }, this);
    }
    void MainFsrvPage::DetailsEditCurrent_CommitButton_Click(IInspectable const&, RoutedEventArgs const&) {
        // TODO: Verify input
        m_async.cancel_and_run([](MainFsrvPage* that) -> IAsyncAction {
            auto cancellation_token = co_await get_cancellation_token();
            cancellation_token.enable_propagation();

            auto client = that->m_main_vm->GetClient();

            auto fsrv_item = that->FsrvListView().SelectedItem().as<FsrvItem>();
            auto id = fsrv_item->Id();
            auto name = that->DetailsEditCurrent_FsrvNameTextBox().Text();
            JsonValue config{ nullptr };
            JsonValue::TryParse(that->DetailsEditCurrent_FsrvConfigTextBox().Text(), config);
            co_await client.update_fsrv_info(id, name, config);

            co_await that->ReloadFsrvListAsync();
            that->SelectFsrvItemById(id);
        }, this);
    }
    IAsyncAction MainFsrvPage::ReloadFsrvListAsync() {
        auto cancellation_token = co_await get_cancellation_token();
        cancellation_token.enable_propagation();

        VisualStateManager::GoToState(*this, L"Empty", true);
        co_await m_main_vm->ReloadFsrvItemsAsync();
    }
    bool MainFsrvPage::SelectFsrvItemById(guid const& id) {
        auto fsrv_items = m_main_vm->FsrvItems();
        uint32_t size = fsrv_items.Size();
        for (uint32_t i = 0; i < size; i++) {
            auto fsrv_item = fsrv_items.GetAt(i).as<FsrvItem>();
            if (fsrv_item->Id() == id) {
                this->FsrvListView().SelectedIndex(static_cast<int32_t>(i));
                return true;
            }
        }
        return false;
    }
    IAsyncOperation<bool> MainFsrvPage::StartFsrvAsync(guid id) {
        auto cancellation_token = co_await get_cancellation_token();
        cancellation_token.enable_propagation();

        auto client = m_main_vm->GetClient();

        co_return co_await client.start_fsrv(id);
    }
    IAsyncOperation<bool> MainFsrvPage::StopFsrvAsync(guid id) {
        auto cancellation_token = co_await get_cancellation_token();
        cancellation_token.enable_propagation();

        auto client = m_main_vm->GetClient();

        co_return co_await client.stop_fsrv(id);
    }
    hstring MainFsrvPage::IsRunningToStartStopButtonText(bool isRunning) {
        // 0xE768: Play, 0xE71A: Stop
        static constexpr wchar_t STR_PLAY[] = L"\xE768", STR_STOP[] = L"\xE71A";
        return isRunning ? STR_STOP : STR_PLAY;
    }
}
