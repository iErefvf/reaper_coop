# reaper_coop / REAPER 协作插件

这是一个基于 Rust + reaper-rs 的 REAPER 局域网协作插件，目标是在同一局域网内让多台机器协作编辑相同的 REAPER 工程。

当前这个项目仍属于实验性/原型阶段，但它已经具备了一个可工作的基础结构：插件入口、Action 注册、UDP 广播、TCP 连接、成员管理，以及一条可手动触发的基本操作同步链路。

## 当前状态

- [x] 插件骨架与 REAPER Action 注册
- [x] 消息协议编解码（serde + serde_json）
- [x] UDP 房间广播与发现
- [x] TCP 控制通道与成员管理
- [x] 基础操作同步链路：发送操作、排队、定时器应用
- [x] 轨道新增的本地应用逻辑
- [ ] 真正的双机 / 虚拟机稳定验证
- [ ] 更完整的 REAPER 原生 hook 接入
- [ ] 扩展更多操作类型（删除、FX、对象等）
- [ ] 撤销、工程同步、光标共享、VST 缺失处理

## 功能概览

已完成：

- **创建房间**：房主启动 UDP 广播和 TCP Server，等待成员加入。
- **搜索房间**：成员可以在局域网内查找可用房间。
- **加入房间**：成员输入房主 IP 后建立 TCP 连接。
- **成员管理**：房主维护成员列表，处理成员加入和离开。
- **基础同步链路**：支持将操作请求发给房主、在房主侧处理后广播，并由成员本地队列 + 定时器执行应用。
- **增轨操作**：当前已实现 `TrackAdd` 的本地应用逻辑，并通过手动 Action 测试验证过基础链路。

规划中：

- 更多操作类型：轨道删除、FX 增删改、对象增删改。
- 按作者撤销：仅撤销自己的操作，不影响其他成员。
- 光标共享：在 Arrange View 中看到其他成员的光标位置。
- 工程文件传输：成员加入时自动加载房主工程。
- VST 缺失处理：缺失的 VST 走占位符/静音方案。

> 注意：当前代码中已经存在 hook 和 timer 的注册逻辑，但真正的 REAPER 运行时行为仍需要在实际插件环境中做稳定验证。

## 环境要求

- **REAPER** 7.x
- **Rust** 工具链（[rustup](https://rustup.rs/)）
- **C++ 构建工具**
  - Windows：Visual Studio Build Tools（勾选“使用 C++ 的桌面开发”）
  - macOS：`xcode-select --install`
  - Linux：`build-essential`
- **SWS 扩展**（后续光标共享功能需要）
- **ReaImGui**（后续 UI 功能需要）

## 构建

在项目根目录执行：

```bash
cargo build --release
```

产物位于 `target/release/`：

- Windows：`reaper_coop.dll`
- macOS：`libreaper_coop.dylib`
- Linux：`libreaper_coop.so`

将产物复制到 REAPER 的 `UserPlugins` 目录（可通过 `Options → Show REAPER resource path` 找到），然后**完全退出 REAPER**后重新启动。

## 使用

打开 REAPER 的 Actions 列表（快捷键 `?`），搜索 `协作`，可以看到以下 Action：

| Action | 说明 |
|--------|------|
| 协作: 创建房间 | 启动 UDP 广播和 TCP Server，作为房主等待成员加入 |
| 协作: 搜索房间 | 在局域网内搜索可用的协作房间 |
| 协作: 加入房间 | 弹出输入框填写房主 IP，加入房间 |
| 协作: 显示日志 | 查看后台线程收集的运行日志 |
| 协作: 测试插入轨道 | 手动发送一次增加轨道操作，用于验证同步链路 |

## 测试步骤

### 1. 真实双机 / 虚拟机测试

1. A 机作为房主，运行 `协作: 创建房间`。
2. B 机作为成员，运行 `协作: 加入房间`，输入 A 机 IP。
3. 在两边都运行 `协作: 显示日志`，确认成员连接成功。
4. 在 A 机上运行 `协作: 测试插入轨道`。
5. 观察 B 机工程中是否出现同步新增轨道。
6. 再反向验证一次：在 B 机上触发一次测试插入，A 机是否同步。
7. B 机退出 REAPER 后，A 机再次查看日志，确认成员离开事件。

### 2. 单机自测

如果没有第二台机器，可以在同一台电脑上做自连验证：

1. 运行 `协作: 创建房间`。
2. 运行 `协作: 加入房间`，输入 `127.0.0.1`。
3. 运行 `协作: 测试插入轨道`。
4. 运行 `协作: 显示日志`，确认看到本地链路中发出的日志。

> 说明：自连测试只用于验证本地逻辑和回路，不等于真实双机同步验证。

## 项目结构

```text
src/
├── lib.rs                  插件入口、Action 注册
├── protocol.rs             消息类型定义、编解码
├── network/
│   ├── mod.rs              网络模块入口、全局日志缓冲
│   ├── discovery.rs        UDP 房间广播与发现
│   ├── server.rs           TCP Server、成员管理
│   └── client.rs           TCP Client、连接房主
└── sync/
    ├── mod.rs              全局操作发送通道
    ├── hook.rs             hookcommand2 拦截
    ├── queue.rs            待应用操作队列
    └── apply.rs            本地应用操作
```

## 技术栈

- **Rust**：内存安全、并发安全
- **reaper-rs**：REAPER 的 Rust 绑定，使用 master 分支
- **serde + serde_json**：消息序列化
- **std::net**：TCP / UDP 通信

## 架构

采用 **客户端-服务器（C/S）** 架构：

- 房主同时扮演 Server 和 Client 两个角色。
- 所有成员只和房主通信，成员之间不直接连接。
- 统一将操作请求发送给房主，房主在分配顺序号后广播给所有成员。
- 各成员按同样顺序应用本地操作，从而尽量保持状态一致。

```text
成员A ──┐
成员B ──┼── TCP/UDP ── 房主（Server + Client）
成员C ──┘
```

### 操作同步数据流

```text
成员端：
  hookcommand2 / Action → send_op → TCP 发给房主
  房主广播 OpApply → 读线程 push 到队列 → 定时器 drain → apply_op_locally

房主端：
  hookcommand2 / Action → send_op → host_submit_op
    → 分配 id / seq → 广播 OpApply
  定时器 drain → apply_op_locally
```

## 端口约定

| 用途 | 协议 | 端口 |
|------|------|------|
| 房间发现（UDP 广播） | UDP | 22222 |
| 控制通道（成员与房主） | TCP | 22223 |
| 光标转发 | UDP | 22224（规划中） |

## 开发说明

- **reaper-rs 必须使用 master 分支**，crates.io 上的版本过时。
- **后台线程不能直接调用 REAPER API**，日志及工程修改必须通过主线程/队列来处理。
- **Action 注册后建议 `std::mem::forget`**，否则 `RegisteredAction` 一旦 Drop 就会被反注册。
- **插件初始化必须先调用 `reaper.wake_up()?`**，再注册 Action 与 hook。
- **当前实现中，Action 触发的轨道同步链路已具备基本可用性**；真实场景下仍需在 REAPER 环境中进行网络/一致性验证。

## 已知限制

- 成员加入时不会自动加载房主工程文件，需要手动打开同一个工程。
- 真实 REAPER 原生操作 hook 的覆盖范围还需继续扩展。
- 还没有完整处理撤销、FX、对象同步等更复杂操作。
- 工程文件传输、VST 缺失处理、光标共享等功能仍在规划中。

## 路线图

1. 稳定真机 / 虚拟机网络验证
2. 增加更多操作类型（删除、FX、对象）
3. 支持按作者撤销
4. 增加共享光标
5. 处理工程文件同步与 VST 缺失问题

## 许可证

MIT