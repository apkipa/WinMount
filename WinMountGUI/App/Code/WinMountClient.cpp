#include "pch.h"

#include "WinMountClient.hpp"

using namespace winrt;
using namespace Windows::Networking::Sockets;
using namespace Windows::Storage::Streams;
using namespace Windows::Data::Json;

struct MessageResponse {
    uint64_t syn;
    int32_t code;
    hstring msg;
    JsonValue data;
};

namespace json {
    template<typename T>
    T get(JsonObject const& jo, hstring const& key) {
        if constexpr (std::is_same_v<T, JsonValue>) {
            return jo.GetNamedValue(key);
        }
        else if constexpr (std::is_same_v<T, bool>) {
            return jo.GetNamedBoolean(key);
        }
        else if constexpr (std::is_arithmetic_v<T>) {
            return static_cast<T>(jo.GetNamedNumber(key));
        }
        else if constexpr (std::is_same_v<T, JsonArray>) {
            return jo.GetNamedArray(key);
        }
        else if constexpr (std::is_same_v<T, JsonObject>) {
            return jo.GetNamedObject(key);
        }
        else if constexpr (std::is_same_v<T, hstring>) {
            return jo.GetNamedString(key);
        }
        else if constexpr (std::is_same_v<T, guid>) {
            return util::winrt::to_guid(jo.GetNamedString(key));
        }
        else {
            static_assert(util::misc::always_false_v<T>, "invalid JSON get type");
        }
    }
    template<typename T>
    T get(IJsonValue const& jv) {
        if constexpr (std::is_same_v<T, bool>) {
            return jv.GetBoolean();
        }
        else if constexpr (std::is_arithmetic_v<T>) {
            return static_cast<T>(jv.GetNumber());
        }
        else if constexpr (std::is_same_v<T, JsonArray>) {
            return jv.GetArray();
        }
        else if constexpr (std::is_same_v<T, JsonObject>) {
            return jv.GetObject();
        }
        else if constexpr (std::is_same_v<T, hstring>) {
            return jv.GetString();
        }
        else if constexpr (std::is_same_v<T, guid>) {
            return util::winrt::to_guid(jv.GetString());
        }
        else {
            static_assert(util::misc::always_false_v<T>, "invalid JSON get type");
        }
    }
    template<typename T>
    void put(JsonObject const& jo, hstring const& key, T const& value) {
        if constexpr (std::is_base_of_v<IJsonValue, T>) {
            jo.Insert(key, value);
        }
        else {
            JsonValue jv{ nullptr };
            if constexpr (std::is_same_v<T, bool>) {
                jv = JsonValue::CreateBooleanValue(value);
            }
            else if constexpr (std::is_arithmetic_v<T>) {
                jv = JsonValue::CreateNumberValue(static_cast<double>(value));
            }
            else if constexpr (std::is_same_v<T, hstring>) {
                jv = JsonValue::CreateStringValue(value);
            }
            else if constexpr (std::is_same_v<T, guid>) {
                jv = JsonValue::CreateStringValue(util::winrt::to_hstring(value));
            }
            else {
                static_assert(util::misc::always_false_v<T>, "invalid JSON put type");
            }
            jo.Insert(key, jv);
        }
    }
    template<typename T>
    void read(JsonObject const& jo, hstring const& key, T& value) {
        value = get<T>(jo, key);
    }
    template<typename T>
    void read(IJsonValue const& jv, T& value) {
        value = get<T>(jv);
    }
    template<typename T>
    void read(JsonObject const& jo, hstring const& key, std::vector<T>& value) {
        value.clear();
        auto ja = get<JsonArray>(jo, key);
        auto size = ja.Size();
        value.reserve(size);
        for (uint32_t i = 0; i < size; i++) {
            T temp;
            read(ja.GetAt(i), temp);
            value.push_back(std::move(temp));
        }
    }
    template<typename T, size_t N>
    void read(JsonObject const& jo, hstring const& key, T(&value)[N]) {
        auto ja = get<JsonArray>(jo, key);
        auto size = ja.Size();
        if (size != N) {
            throw hresult_error(E_FAIL,
                std::format(L"JSON array size mismatch (expected {}, found {})", N, size));
        }
        for (uint32_t i = 0; i < size; i++) {
            read(ja.GetAt(i), value[i]);
        }
    }
    template<typename T>
    void read(IJsonValue const& jv, std::vector<T>& value) {
        value.clear();
        auto ja = get<JsonArray>(jv);
        auto size = ja.Size();
        value.reserve(size);
        for (uint32_t i = 0; i < size; i++) {
            T temp;
            read(ja.GetAt(i), temp);
            value.push_back(std::move(temp));
        }
    }
    template<typename T, size_t N>
    void read(IJsonValue const& jv, T (&value)[N]) {
        auto ja = get<JsonArray>(jv);
        auto size = ja.Size();
        if (size != N) {
            throw hresult_error(E_FAIL,
                std::format(L"JSON array size mismatch (expected {}, found {})", N, size));
        }
        for (uint32_t i = 0; i < size; i++) {
            read(ja.GetAt(i), value[i]);
        }
    }
}

// For custom parsing injection
namespace json {
    template<>
    void read(IJsonValue const& jv, ::WinMount::ListFileSystemItemData& value) {
        auto jo = get<JsonObject>(jv);
        read(jo, L"id", value.id);
        read(jo, L"name", value.name);
        read(jo, L"kind_id", value.kind_id);
        read(jo, L"is_running", value.is_running);
        read(jo, L"is_global", value.is_global);
    }
    template<>
    void read(IJsonValue const& jv, ::WinMount::ListFileSystemProviderItemData& value) {
        auto jo = get<JsonObject>(jv);
        read(jo, L"id", value.id);
        read(jo, L"name", value.name);
        read(jo, L"version", value.version);
        read(jo, L"template_config", value.template_config);
        read(jo, L"is_hidden", value.is_hidden);
    }
    template<>
    void read(IJsonValue const& jv, ::WinMount::ListFServerItemData& value) {
        auto jo = get<JsonObject>(jv);
        read(jo, L"id", value.id);
        read(jo, L"name", value.name);
        read(jo, L"kind_id", value.kind_id);
        read(jo, L"in_fs_id", value.in_fs_id);
        read(jo, L"is_running", value.is_running);
    }
    template<>
    void read(IJsonValue const& jv, ::WinMount::ListFServerProviderItemData& value) {
        auto jo = get<JsonObject>(jv);
        read(jo, L"id", value.id);
        read(jo, L"name", value.name);
        read(jo, L"version", value.version);
        read(jo, L"template_config", value.template_config);
    }
    template<>
    void read(IJsonValue const& jv, ::WinMount::GetFileSystemInfoData& value) {
        auto jo = get<JsonObject>(jv);
        read(jo, L"name", value.name);
        read(jo, L"kind_id", value.kind_id);
        read(jo, L"is_running", value.is_running);
        read(jo, L"is_global", value.is_global);
        read(jo, L"config", value.config);
    }
    template<>
    void read(IJsonValue const& jv, ::WinMount::GetFServerInfoData& value) {
        auto jo = get<JsonObject>(jv);
        read(jo, L"name", value.name);
        read(jo, L"kind_id", value.kind_id);
        read(jo, L"in_fs_id", value.in_fs_id);
        read(jo, L"is_running", value.is_running);
        read(jo, L"config", value.config);
    }
}

namespace WinMount {
    struct WinMountClientImpl final {
        WinMountClientImpl() {}
        ~WinMountClientImpl() { close(); }

        util::winrt::task<> initialize_connection(hstring url) {
            auto cancellation_token = co_await get_cancellation_token();
            cancellation_token.enable_propagation();

            co_await resume_background();

            m_ws = MessageWebSocket();
            m_ws.MessageReceived({ this, &WinMountClientImpl::WebSocketMsgReceived });
            auto ws_ctrl = m_ws.Control();
            ws_ctrl.ReceiveMode(MessageWebSocketReceiveMode::FullMessage);
            ws_ctrl.MessageType(SocketMessageType::Utf8);
            co_await m_ws.ConnectAsync(Windows::Foundation::Uri(url));

            // Perform handshake
            this->ws_write_str(hstring{ std::format(L"WinMount connect v{}", CLIENT_VERSION) });
            cancellation_token.callback([&] { this->close(); });
            m_handshake_completed.wait(false, std::memory_order_acquire);
            if (m_remote_ver.empty()) {
                throw hresult_error(E_FAIL, L"remote didn't accept the connection request");
            }
        }

        void close(void) {
            if (m_ws) {
                m_ws.Close();
                m_ws = nullptr;

                // Force complete operation
                m_handshake_completed.store(true, std::memory_order_release);
                m_handshake_completed.notify_one();
                {
                    std::vector<std::coroutine_handle<>> resp_resumes;
                    {
                        std::scoped_lock guard{ m_mutex_resp };
                        resp_resumes.swap(m_resp_resumes);
                    }
                    for (auto&& i : resp_resumes) { i(); }
                }
            }
        }

        hstring get_daemon_version() { return m_remote_ver; }

        util::winrt::task<guid> create_fs(hstring const& name, guid const& kind_id, IJsonValue const& config) {
            auto cancellation_token = co_await get_cancellation_token();
            cancellation_token.enable_propagation();

            auto jo = JsonObject();
            json::put(jo, L"name", name);
            json::put(jo, L"kind_id", kind_id);
            if (config) { json::put(jo, L"config", config); }
            auto resp = co_await ws_do_request(L"create-fs", jo);
            ensure_successful_response(resp);
            co_return json::get<guid>(resp.data.GetObject(), L"fs_id");
        }
        util::winrt::task<> remove_fs(guid const& id) {
            auto cancellation_token = co_await get_cancellation_token();
            cancellation_token.enable_propagation();

            auto jo = JsonObject();
            json::put(jo, L"id", id);
            auto resp = co_await ws_do_request(L"remove-fs", jo);
            ensure_successful_response(resp);
        }
        util::winrt::task<bool> start_fs(guid const& id) {
            auto cancellation_token = co_await get_cancellation_token();
            cancellation_token.enable_propagation();

            auto jo = JsonObject();
            json::put(jo, L"id", id);
            auto resp = co_await ws_do_request(L"start-fs", jo);
            ensure_successful_response(resp);
            co_return json::get<bool>(resp.data.GetObject(), L"new_started");
        }
        util::winrt::task<bool> stop_fs(guid const& id) {
            auto cancellation_token = co_await get_cancellation_token();
            cancellation_token.enable_propagation();

            auto jo = JsonObject();
            json::put(jo, L"id", id);
            auto resp = co_await ws_do_request(L"stop-fs", jo);
            ensure_successful_response(resp);
            co_return json::get<bool>(resp.data.GetObject(), L"new_stopped");
        }
        util::winrt::task<guid> create_fsrv(
            hstring const& name,
            guid const& kind_id,
            guid const& in_fs_id,
            IJsonValue const& config
        ) {
            auto cancellation_token = co_await get_cancellation_token();
            cancellation_token.enable_propagation();

            auto jo = JsonObject();
            json::put(jo, L"name", name);
            json::put(jo, L"kind_id", kind_id);
            json::put(jo, L"in_fs_id", in_fs_id);
            if (config) { json::put(jo, L"config", config); }
            auto resp = co_await ws_do_request(L"create-fsrv", jo);
            ensure_successful_response(resp);
            co_return json::get<guid>(resp.data.GetObject(), L"fsrv_id");
        }
        util::winrt::task<> remove_fsrv(guid const& id) {
            auto cancellation_token = co_await get_cancellation_token();
            cancellation_token.enable_propagation();

            auto jo = JsonObject();
            json::put(jo, L"id", id);
            auto resp = co_await ws_do_request(L"remove-fsrv", jo);
            ensure_successful_response(resp);
        }
        util::winrt::task<bool> start_fsrv(guid const& id) {
            auto cancellation_token = co_await get_cancellation_token();
            cancellation_token.enable_propagation();

            auto jo = JsonObject();
            json::put(jo, L"id", id);
            auto resp = co_await ws_do_request(L"start-fsrv", jo);
            ensure_successful_response(resp);
            co_return json::get<bool>(resp.data.GetObject(), L"new_started");
        }
        util::winrt::task<bool> stop_fsrv(guid const& id) {
            auto cancellation_token = co_await get_cancellation_token();
            cancellation_token.enable_propagation();

            auto jo = JsonObject();
            json::put(jo, L"id", id);
            auto resp = co_await ws_do_request(L"stop-fsrv", jo);
            ensure_successful_response(resp);
            co_return json::get<bool>(resp.data.GetObject(), L"new_stopped");
        }
        util::winrt::task<std::vector<ListFileSystemItemData>> list_fs() {
            auto cancellation_token = co_await get_cancellation_token();
            cancellation_token.enable_propagation();

            auto resp = co_await ws_do_request(L"list-fs", nullptr);
            ensure_successful_response(resp);
            std::vector<ListFileSystemItemData> result;
            json::read(resp.data.GetObject(), L"fs_list", result);
            co_return result;
        }
        util::winrt::task<std::vector<ListFileSystemProviderItemData>> list_fsp() {
            auto cancellation_token = co_await get_cancellation_token();
            cancellation_token.enable_propagation();

            auto resp = co_await ws_do_request(L"list-fsp", nullptr);
            ensure_successful_response(resp);
            std::vector<ListFileSystemProviderItemData> result;
            json::read(resp.data.GetObject(), L"fsp_list", result);
            co_return result;
        }
        util::winrt::task<std::vector<ListFServerItemData>> list_fsrv() {
            auto cancellation_token = co_await get_cancellation_token();
            cancellation_token.enable_propagation();

            auto resp = co_await ws_do_request(L"list-fsrv", nullptr);
            ensure_successful_response(resp);
            std::vector<ListFServerItemData> result;
            json::read(resp.data.GetObject(), L"fsrv_list", result);
            co_return result;
        }
        util::winrt::task<std::vector<ListFServerProviderItemData>> list_fsrvp() {
            auto cancellation_token = co_await get_cancellation_token();
            cancellation_token.enable_propagation();

            auto resp = co_await ws_do_request(L"list-fsrvp", nullptr);
            ensure_successful_response(resp);
            std::vector<ListFServerProviderItemData> result;
            json::read(resp.data.GetObject(), L"fsrvp_list", result);
            co_return result;
        }
        util::winrt::task<GetFileSystemInfoData> get_fs_info(guid const& id) {
            auto cancellation_token = co_await get_cancellation_token();
            cancellation_token.enable_propagation();

            auto jo = JsonObject();
            json::put(jo, L"id", id);
            auto resp = co_await ws_do_request(L"get-fs-info", jo);
            ensure_successful_response(resp);
            GetFileSystemInfoData result;
            json::read(resp.data, result);
            co_return result;
        }
        util::winrt::task<GetFServerInfoData> get_fsrv_info(guid const& id) {
            auto cancellation_token = co_await get_cancellation_token();
            cancellation_token.enable_propagation();

            auto jo = JsonObject();
            json::put(jo, L"id", id);
            auto resp = co_await ws_do_request(L"get-fsrv-info", jo);
            ensure_successful_response(resp);
            GetFServerInfoData result;
            json::read(resp.data, result);
            co_return result;
        }
        util::winrt::task<> update_fs_info(guid const& id, hstring const& name, IJsonValue const& config) {
            auto cancellation_token = co_await get_cancellation_token();
            cancellation_token.enable_propagation();

            auto jo = JsonObject();
            json::put(jo, L"id", id);
            if (!name.empty()) { json::put(jo, L"name", name); }
            if (config) { json::put(jo, L"config", config); }
            auto resp = co_await ws_do_request(L"update-fs-info", jo);
            ensure_successful_response(resp);
        }
        util::winrt::task<> update_fsrv_info(guid const& id, hstring const& name, IJsonValue const& config) {
            auto cancellation_token = co_await get_cancellation_token();
            cancellation_token.enable_propagation();

            auto jo = JsonObject();
            json::put(jo, L"id", id);
            if (!name.empty()) { json::put(jo, L"name", name); }
            if (config) { json::put(jo, L"config", config); }
            auto resp = co_await ws_do_request(L"update-fsrv-info", jo);
            ensure_successful_response(resp);
        }

    private:
        void WebSocketMsgReceived(MessageWebSocket const&, MessageWebSocketMessageReceivedEventArgs const& e) {
            auto msg_type = e.MessageType();
            if (msg_type == SocketMessageType::Utf8) {
                auto read_str_from_reader_fn = [](DataReader const& reader) {
                    reader.UnicodeEncoding(UnicodeEncoding::Utf8);
                    return reader.ReadString(reader.UnconsumedBufferLength());
                    };

                auto resp = read_str_from_reader_fn(e.GetDataReader());
                if (!m_handshake_completed.load(std::memory_order_relaxed)) {
                    // Perform handshake
                    static constexpr std::wstring_view RESP_ACCEPT_HEAD = L"WinMount accept v";
                    if (resp.starts_with(RESP_ACCEPT_HEAD)) {
                        // Daemon accepted our request
                        m_remote_ver = hstring{ std::wstring_view(resp).substr(RESP_ACCEPT_HEAD.size()) };
                    }
                    m_handshake_completed.store(true, std::memory_order_release);
                    m_handshake_completed.notify_one();
                }
                else {
                    // Receive normal messages
                    JsonObject jo{ nullptr };
                    if (!JsonObject::TryParse(resp, jo)) {
                        util::debug::log_warn(L"Received malformed JSON message");
                        return;
                    }
                    if (jo.GetNamedString(L"type") != L"response") {
                        util::debug::log_warn(L"Received unsupported message type");
                        return;
                    }
                    MessageResponse msg_resp{
                        static_cast<uint64_t>(jo.GetNamedNumber(L"syn")),
                        static_cast<int32_t>(jo.GetNamedNumber(L"code")),
                        jo.GetNamedString(L"msg"),
                        jo.GetNamedValue(L"data")
                    };
                    {
                        std::vector<std::coroutine_handle<>> resp_resumes;
                        {
                            std::scoped_lock guard{ m_mutex_resp };
                            m_resp_queue.push_back(std::move(msg_resp));
                            resp_resumes.swap(m_resp_resumes);
                        }
                        for (auto&& i : resp_resumes) { i(); }
                    }
                }
            }
            else {
                util::debug::log_warn(L"Received unsupported WebSocket message type");
            }
        }

        void ensure_not_closed() {
            if (!m_ws) { throw hresult_illegal_method_call(L"client already closed"); }
        }

        static void ensure_successful_response(MessageResponse const& resp) {
            if (resp.code < 0) {
                throw hresult_error(E_FAIL, std::format(L"RPC failed with code {}: {}", resp.code, resp.msg));
            }
        }

        util::winrt::task<> ws_write_str(hstring const& str) {
            auto cancellation_token = co_await get_cancellation_token();
            cancellation_token.enable_propagation();

            auto dw = DataWriter(m_ws.OutputStream());
            deferred([&] { dw.DetachStream(); });
            dw.WriteString(str);
            co_await dw.StoreAsync();
        }

        util::winrt::task<uint64_t> ws_send_request(hstring const& method, IJsonValue const& params) {
            auto cancellation_token = co_await get_cancellation_token();
            cancellation_token.enable_propagation();

            auto cur_syn = m_syn_counter.fetch_add(1, std::memory_order_relaxed);
            auto jo = JsonObject();
            jo.Insert(L"type", JsonValue::CreateStringValue(L"request"));
            jo.Insert(L"syn", JsonValue::CreateNumberValue(static_cast<double>(cur_syn)));
            jo.Insert(L"method", JsonValue::CreateStringValue(method));
            if (params) { jo.Insert(L"params", params); }
            co_await this->ws_write_str(jo.Stringify());
            co_return cur_syn;
        }
        util::winrt::task<MessageResponse> ws_read_response(uint64_t syn) {
            auto cancellation_token = co_await get_cancellation_token();
            cancellation_token.enable_propagation();

            // TODO: Unit test for awaitable
            struct awaitable : enable_await_cancellation {
                // PRECONDITION: Caller must hold the lock before awaiting
                awaitable(WinMountClientImpl* that, std::unique_lock<std::mutex>& guard) :
                    m_that(that), m_guard(guard) {}
                bool await_ready() const { return false; }
                void await_suspend(std::coroutine_handle<> resume) {
                    m_self = resume;
                    m_that->m_resp_resumes.push_back(resume);
                    m_guard.unlock();
                }
                void await_resume() const {
                    if (m_guard.owns_lock()) { throw hresult_canceled(); }
                    m_guard.lock();
                }
                void enable_cancellation(cancellable_promise* promise) {
                    promise->set_canceller([](void* p) {
                        auto that = static_cast<awaitable*>(p);
                        // SAFETY: Cancellation never runs on the same thread as resume at this time
                        std::unique_lock guard{ that->m_that->m_mutex_resp };
                        auto it = std::ranges::find_if(that->m_that->m_resp_resumes,
                            [&](std::coroutine_handle<> const& v) { return that->m_self.address() == v.address(); }
                        );
                        if (it != that->m_that->m_resp_resumes.end()) {
                            // Not resumed, cancel then resume immediately
                            that->m_guard = std::move(guard);
                            that->m_that->m_resp_resumes.erase(it);
                            // TODO: We can also clear stale responses?
                            // NOTE: We MUST resume on current thread, or UB will be invoked
                            //       (unique_lock cannot be moved across threads)
                            that->m_self();
                        }
                        else {
                            // Already resumed or pending resume, do nothing
                        }
                    }, this);
                }

            private:
                std::coroutine_handle<> m_self;
                WinMountClientImpl* m_that;
                std::unique_lock<std::mutex>& m_guard;
            };

            std::unique_lock guard{ m_mutex_resp };
            while (m_ws) {
                // First check for existing responses
                auto it = std::ranges::find_if(m_resp_queue, [&](MessageResponse const& v) {
                    return v.syn == syn;
                });
                if (it != m_resp_queue.end()) {
                    auto result = std::move(*it);
                    m_resp_queue.erase(it);
                    co_return result;
                }
                // Then wait for new responses
                co_await awaitable{ this, guard };
            }
            // Force throw exception
            ensure_not_closed();
        }
        util::winrt::task<MessageResponse> ws_do_request(hstring const& method, IJsonValue const& params) {
            auto cancellation_token = co_await get_cancellation_token();
            cancellation_token.enable_propagation();

            co_return co_await ws_read_response(co_await ws_send_request(method, params));
        }

        MessageWebSocket m_ws{ nullptr };

        std::atomic_bool m_handshake_completed{ false };
        hstring m_remote_ver;

        std::atomic_uint64_t m_syn_counter{ 0 };

        std::mutex m_mutex_resp;
        std::vector<std::coroutine_handle<>> m_resp_resumes;
        std::deque<MessageResponse> m_resp_queue;
    };

    util::winrt::task<WinMountClient> connect_winmount_client(hstring const& url) {
        auto cancellation_token = co_await get_cancellation_token();
        cancellation_token.enable_propagation();

        auto client = std::make_shared<WinMountClientImpl>();
        co_await client->initialize_connection(url);

        co_return client;
    }

    void WinMountClient::close() const {
        m_impl->close();
    }
    hstring WinMountClient::get_daemon_version() const {
        return m_impl->get_daemon_version();
    }
    util::winrt::task<guid> WinMountClient::create_fs(
        hstring const& name,
        guid const& kind_id,
        IJsonValue const& config
    ) const {
        return m_impl->create_fs(name, kind_id, config);
    }
    util::winrt::task<> WinMountClient::remove_fs(guid const& id) const {
        return m_impl->remove_fs(id);
    }
    util::winrt::task<bool> WinMountClient::start_fs(guid const& id) const {
        return m_impl->start_fs(id);
    }
    util::winrt::task<bool> WinMountClient::stop_fs(guid const& id) const {
        return m_impl->stop_fs(id);
    }
    util::winrt::task<guid> WinMountClient::create_fsrv(
        hstring const& name,
        guid const& kind_id,
        guid const& in_fs_id,
        IJsonValue const& config
    ) const {
        return m_impl->create_fsrv(name, kind_id, in_fs_id, config);
    }
    util::winrt::task<> WinMountClient::remove_fsrv(guid const& id) const {
        return m_impl->remove_fsrv(id);
    }
    util::winrt::task<bool> WinMountClient::start_fsrv(guid const& id) const {
        return m_impl->start_fsrv(id);
    }
    util::winrt::task<bool> WinMountClient::stop_fsrv(guid const& id) const {
        return m_impl->stop_fsrv(id);
    }
    util::winrt::task<std::vector<ListFileSystemItemData>> WinMountClient::list_fs() const {
        return m_impl->list_fs();
    }
    util::winrt::task<std::vector<ListFileSystemProviderItemData>> WinMountClient::list_fsp() const {
        return m_impl->list_fsp();
    }
    util::winrt::task<std::vector<ListFServerItemData>> WinMountClient::list_fsrv() const {
        return m_impl->list_fsrv();
    }
    util::winrt::task<std::vector<ListFServerProviderItemData>> WinMountClient::list_fsrvp() const {
        return m_impl->list_fsrvp();
    }
    util::winrt::task<GetFileSystemInfoData> WinMountClient::get_fs_info(guid const& id) const {
        return m_impl->get_fs_info(id);
    }
    util::winrt::task<GetFServerInfoData> WinMountClient::get_fsrv_info(guid const& id) const {
        return m_impl->get_fsrv_info(id);
    }
    util::winrt::task<> WinMountClient::update_fs_info(
        guid const& id,
        hstring const& name,
        IJsonValue const& config
    ) const {
        return m_impl->update_fs_info(id, name, config);
    }
    util::winrt::task<> WinMountClient::update_fsrv_info(
        guid const& id,
        hstring const& name,
        IJsonValue const& config
    ) const {
        return m_impl->update_fsrv_info(id, name, config);
    }
}
