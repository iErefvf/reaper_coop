# reaper_coop/reaper协作插件

这个项目本质是我初学rust了解大致结构时vibe coding出来的产物，可能会很烂。等之后慢慢学了更多东西之后会进行更多优化。

大概目标是做到能够联机使用reaper并编辑工程文件，因为各个用户的vst基本上无法完全相同的原因，目前只希望能够做到对项目的摆放同步，这样就可以完成一些基础的音MAD音频制作的联机功能了。

目前的进度连大致的框架都还没成型。

# reaper_coop

REAPER 局域网协作插件。基于 Rust 和 reaper-rs 开发，目标是让多台机器在同一个局域网内协作编辑同一个 REAPER 工程。

## 当前进度

- [x] 阶段 1：插件骨架、Action 注册
- [x] 阶段 2：消息协议编解码
- [x] 阶段 3：UDP 房间广播与发现
- [x] 阶段 4：TCP 连接与成员管理
- [ ] 阶段 5：操作同步核心链路（进行中）
- [ ] 阶段 6：扩展操作类型（轨道删除、FX、对象）
- [ ] 阶段 7：按作者撤销
- [ ] 阶段 8：鼠标光标共享
- [ ] 阶段 9：工程文件传输
- [ ] 阶段 10：VST 缺失处理

## 功能概览

已完成：

- **创建房间**：房主启动 UDP 广播和 TCP Server，等待成员加入。
- **搜索房间**：成员在局域网内搜索可用房间。
- **加入房间**：成员输入房主 IP，通过 TCP 连接加入。
- **成员管理**：房主维护成员列表，处理加入和离开。

规划中：

- 操作同步：轨道、FX、对象的增删改在各成员间实时同步。
- 按作者撤销：Ctrl+Z 只撤销自己的操作，不影响他人。
- 光标共享：在 Arrange View 中看到其他成员的鼠标位置。
- 工程文件传输：成员加入时自动获取房主的工程文件。
- VST 缺失处理：缺失的 VST 走占位符静音，参数同步自动过滤。

## 环境要求

- **REAPER** 7.x
- **Rust** 工具链（[rustup](https://rustup.rs/)）
- **C++ 构建工具**
  - Windows：Visual Studio Build Tools（勾选“使用 C++ 的桌面开发”）
  - macOS：`xcode-select --install`
  - Linux：`build-essential`
- **SWS 扩展**（后续光标共享功能需要）
- **ReaImGui**（通过 ReaPack 安装，后续 UI 功能需要）

## 构建

```bash
cargo build --release
```

产物在 `target/release/` 下：

- Windows：`reaper_coop.dll`
- macOS：`libreaper_coop.dylib`
- Linux：`libreaper_coop.so`

把产物复制到 REAPER 的 `UserPlugins` 目录（通过 `Options → Show REAPER resource path` 找到），完全退出 REAPER 后重新启动。

## 使用

打开 REAPER 的 Actions 列表（快捷键 `?`），搜索 `协作`，可以看到以下 Action：

| Action | 说明 |
|--------|------|
| 协作: 创建房间 | 启动 UDP 广播和 TCP Server，作为房主等待成员加入 |
| 协作: 搜索房间 | 在局域网内搜索可用的协作房间，3 秒后输出结果 |
| 协作: 加入房间 | 弹出输入框填写房主 IP，加入房间 |
| 协作: 显示日志 | 查看后台线程收集的运行日志 |

### 双机测试流程

1. **A 机**（房主）：运行 `协作: 创建房间`，记下控制台显示的 IP。
2. **B 机**（成员）：运行 `协作: 加入房间`，输入 A 机的 IP。
3. **A 机**：运行 `协作: 显示日志`，确认看到成员加入。
4. **B 机**：完全退出 REAPER。
5. **A 机**：再运行 `协作: 显示日志`，确认看到成员离开。

## 项目结构

```
src/
├── lib.rs                  插件入口、Action 注册
├── protocol.rs             消息类型定义、编解码
├── network/
│   ├── mod.rs              网络模块入口、全局日志缓冲
│   ├── discovery.rs        UDP 房间广播与发现
│   ├── server.rs           TCP Server、成员管理
│   └── client.rs           TCP Client、连接房主
└── sync/                   （阶段 5 新增）
    ├── mod.rs              操作发送通道
    ├── hook.rs             hookcommand2 拦截
    └── queue.rs            待应用操作队列
```

## 技术栈

- **Rust**：内存安全、并发安全、零成本抽象
- **reaper-rs**：REAPER 的 Rust 绑定，使用 master 分支
- **serde + serde_json**：消息序列化
- **std::net**：TCP/UDP 通信

## 架构

采用 **客户端-服务器（C/S）** 架构：

- 房主同时扮演 Server 和 Client 两个角色。
- 所有成员只和房主通信，成员之间不直接连接。
- 操作请求统一发给房主，房主分配全局顺序号后广播给所有成员。
- 这样保证了所有客户端按相同顺序应用操作，状态一致。

```
成员A ──┐
成员B ──┼── TCP/UDP ── 房主（Server + Client）
成员C ──┘
```

## 端口约定

| 用途 | 协议 | 端口 |
|------|------|------|
| 房间发现（UDP 广播） | UDP | 22222 |
| 控制通道（成员与房主） | TCP | 22223 |
| 光标转发 | UDP | 22224（规划中） |

## 开发说明

- **reaper-rs 必须使用 master 分支**，crates.io 上的版本已严重过时。
- **后台线程不能直接调用 REAPER API**，所有日志走 `network::push_log`，所有工程修改走 `sync::queue`。
- **Action 注册后必须 `std::mem::forget`**，否则 `RegisteredAction` 被 Drop 时会反注册。
- **插件初始化必须先调用 `reaper.wake_up()?`**，等待 REAPER 完全启动后再注册 Action。

## 许可证

MIT