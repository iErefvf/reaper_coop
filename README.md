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
- **增轨操作**：拦截单轨插入，并在批量/模板 Action 执行后复制新增轨道的完整 track state chunk，房间成员可在不同 REAPER 7.x 版本间同步轨道状态。

规划中：

- 更多操作类型：轨道删除、对象增删改。
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

插件加载时会自动检查本机网络地址，以及 TCP `38081` 和 UDP `38080` 是否可用；
检查失败会立即显示在 REAPER 控制台中。端口检查不会自动修改 Windows 防火墙规则，
因为创建防火墙规则需要管理员权限，仍需按下面的 PowerShell 命令显式放行。

## 测试步骤

### 1. 真实双机 / 虚拟机测试

1. A 机作为房主，运行 `协作: 创建房间`。
2. B 机作为成员，运行 `协作: 加入房间`，输入 A 机 IP。
3. 在两边都运行 `协作: 显示日志`，确认成员连接成功。
4. 在 A 机上运行 `协作: 测试插入轨道`。
5. 观察 B 机工程中是否出现同步新增轨道。
6. 再反向验证一次：在 B 机上触发一次测试插入，A 机是否同步。
7. B 机退出 REAPER 后，A 机再次查看日志，确认成员离开事件。

如果 B 机搜索不到房间，或输入 IP 后出现 Windows `os error 10060`：

1. 在 A 机执行 `ipconfig`，使用两台电脑共同 WiFi 网卡的 IPv4 地址，通常形如 `192.168.x.x` 或 `172.16-31.x.x`；不要使用 `127.0.0.1`、`169.254.x.x`、VMware 或 VPN 网卡地址。
2. 在 B 机执行 `Test-NetConnection <A机IPv4> -Port 38081`。若 `TcpTestSucceeded` 为 `False`，问题在 Windows 防火墙或 WiFi 的客户端隔离，不在加入协议本身。
3. 首次创建房间时允许 REAPER 通过 Windows 防火墙的“专用网络”。也可以由管理员执行：

  ```powershell
  New-NetFirewallRule -DisplayName "REAPER Coop TCP 38081" -Direction Inbound -Action Allow -Protocol TCP -LocalPort 38081 -Profile Private
  New-NetFirewallRule -DisplayName "REAPER Coop UDP 38080" -Direction Inbound -Action Allow -Protocol UDP -LocalPort 38080 -Profile Private
  ```

4. 确认两台机器连接的是同一个主 WiFi，而不是访客网络；路由器开启“AP isolation / 客户端隔离”时，广播和 TCP 都会被阻断。

#### 双机测试排查记录

- 房主本机搜索到自己的房间不代表局域网发现成功。程序会额外向 `127.0.0.1:38080` 发送一份发现报文用于单机自测；另一台电脑能否收到，仍需单独验证。
- 手机热点可能允许设备之间建立 TCP 连接，但阻止 UDP 广播。因此可能出现“搜索不到房间，但手动输入房主 IP 可以加入”。此时直接输入房主无线网卡的 IPv4 地址即可测试 TCP 加入流程。
- 在成员电脑上测试房主 TCP 端口：

  ```powershell
  Test-NetConnection <房主IPv4> -Port 38081
  ```

  `TcpTestSucceeded : True` 表示 TCP 服务可达；`False` 通常表示防火墙、热点设备隔离或房主服务没有监听。房主电脑可以执行：

  ```powershell
  Get-NetTCPConnection -LocalPort 38081
  ```

- `ping` 使用 ICMP，可能被 Windows 或手机热点禁止。即使 `ping` 超时，只要 `Test-NetConnection` 成功，TCP 加入仍然可以正常工作。
- 创建 Windows 防火墙规则必须使用“以管理员身份运行”的 PowerShell。需要放行 TCP `38081` 和 UDP `38080`；如果 PowerShell 报“拒绝访问”，说明当前终端没有管理员权限。
- 两台电脑应连接同一个普通 WiFi，不能是访客网络；同时检查路由器是否开启 `AP isolation`、`Client isolation` 或“无线客户端隔离”。

#### 轨道插入同步排查

- 普通 REAPER Action（包括快捷键和右键菜单）需要使用 `hookcommand` 拦截。`hookcommand2` 主要用于 MIDI CC 和鼠标轮动作，不能作为普通 Action 的通用替代。
- 插件现在观察 Main Section 的每一个 Action，不再依赖固定的命令 ID 白名单。Action 执行前记录轨道 GUID，执行后比较新增 GUID；只有确实新增轨道时才发送同步操作，并在日志中记录触发它的命令 ID。
- 单轨、批量、模板、自定义 Action 都由本机 REAPER 正常执行，插件通过前后 GUID 差异识别新增轨道，再使用 `GetTrackStateChunk` 复制完整轨道状态；成员使用 `SetTrackStateChunk` 还原，因此不会要求两台机器具有相同的 Action 按钮。
- 通用 Action 观察器会为每次 Main Section Action 保存命令 ID、执行前后轨道快照，并计算新增轨道、删除轨道和轨道顺序变化。当前只对新增轨道发起同步；其他变化先保留在观察结果中，后续可接入轨道删除、移动、媒体项和 FX 同步。
- Action 维护表位于 `src/sync/actions.rs`。REAPER 新版本增加或变更命令 ID 时，只需在 `ACTIONS` 表中增加或更新一项，填写 ID、名称、分类和适用版本；未登记的 Action 仍会被通用观察器检测，并在日志中标记为“未登记 Action”。
- 模板引用的音频文件、插件和外部资源不会随 track state chunk 传输；成员电脑仍必须安装相同插件，并能访问相同路径或资源。
- 成员执行批量/模板轨道插入后，房主日志应出现 `房主收到 OpRequest`；发起者不会再次插入自己的操作，其他成员会应用广播的完整轨道状态。
- 重新构建插件后，必须替换两台电脑 `UserPlugins` 中的 DLL，并完全退出、重新启动 REAPER，避免仍加载旧版本。

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
    ├── hook.rs             普通 Action hook 拦截
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
  普通 Action hook / Action → send_op → TCP 发给房主
  房主广播 OpApply → 读线程 push 到队列 → 定时器 drain → apply_op_locally

房主端：
  普通 Action hook / Action → send_op → host_submit_op
    → 分配 id / seq → 广播 OpApply
  定时器 drain → apply_op_locally
```

## 端口约定

| 用途 | 协议 | 端口 |
|------|------|------|
| 房间发现（UDP 广播） | UDP | 38080 |
| 控制通道（成员与房主） | TCP | 38081 |
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