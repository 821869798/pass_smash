# Pass Smash

[English](README.en.md) | 中文

用 Rust + [GPUI Component](https://github.com/longbridge/gpui-component) 编写的桌面密码解锁工具。

## 功能

- 支持 **ZIP** / **7Z** / **RAR** / **PDF** / **Office** (docx/xlsx/pptx/doc) 密码爆破
- **拖入文件** 与 **批量选择**
- 字符集：数字 / 小写 / 大写 / 符号 / 自定义
- 最短 / 最长密码长度
- 多线程暴力枚举（自动按 CPU 核心数）
- 实时进度、速率、取消
- 界面 **中 / 英** 一键切换

## 截图 / 使用

1. 拖入或点击「添加文件」选择 ZIP / 7Z / RAR / PDF / Office  
2. 勾选字符类型，设置长度范围  
3. 点击「开始破解」；可随时「停止」  
4. 成功后在列表中显示密码  

界面右上角 **中 / EN** 可切换语言。

## 构建

需要较新的 Rust（edition 2024，建议 1.85+）。Windows 需 MSVC 与 Windows SDK。首次编译会拉取 GPUI / Zed 依赖。

```bash
cargo run --release
# 产物: target/release/pass_smash.exe
```

Release 版本在 Windows 上会隐藏控制台黑框（`windows_subsystem = "windows"`）；Debug 仍保留控制台便于日志输出。


## 依赖库（当前稳定版）

| 用途 | Crate |
|------|--------|
| ZIP | `zip` 8.x |
| 7Z | `sevenz-rust2` 0.21 |
| RAR | `unrar` 0.5（官方 UnRAR 库封装） |
| PDF | `lopdf` 0.44 |
| Office | `office-crypto` 0.2 |
| UI | `gpui` + `gpui-component` |

## 架构

```
src/
  main.rs
  app.rs                 # 界面 / 拖放 / 语言切换
  i18n.rs                # 中英文文案
  crack/
    types.rs
    charset.rs
    engine.rs
    handlers/
      zip.rs
      sevenz.rs
      rar.rs
      pdf.rs
      office.rs
fixtures/                # 测试样本
```

扩展新格式：实现 `PasswordHandler`，并在 `FileKind` / `handler_for` 注册。

## 测试

## 发布

打 tag 会触发 GitHub Actions，打包 Windows / Linux / macOS 产物并创建 Release：

```bash
# 版本需与 Cargo.toml 中 version 一致，例如 0.1.0
git tag v0.1.0
git push origin v0.1.0
```

产物示例：
- `pass_smash-windows-x64.zip`
- `pass_smash-linux-x64.tar.gz`
- `pass_smash-linux-arm64.tar.gz`
- `pass_smash-macos-arm64.tar.gz`


```bash
cargo test --bin pass_smash
```

含 ZIP / 7Z / RAR / PDF / Office 与字符集相关测试。

## 免责声明

仅供恢复自己遗忘的密码、授权渗透测试与安全研究使用。请勿用于未授权访问他人文件。
