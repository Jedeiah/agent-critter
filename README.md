# Traffic Light — Claude Code 状态指示灯 (Rust 版)

浮动窗口实时显示 Claude Code 运行状态。

## 构建

```bash
cargo build --release
```

## 安装

1. 在 Claude Code 中添加本地 marketplace：
   ```
   /plugin marketplace add /path/to/插件目录
   ```

2. 安装插件：
   ```
   /plugin install claude-traffic-light@claude-traffic-light
   ```

3. 让插件生效：
   ```
   /reload-plugins
   ```
