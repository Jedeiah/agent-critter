# Agent Critter — Claude Code 桌宠小精灵

一只可爱的桌面小精灵，实时反映 Claude Code 的工作状态。

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
   /plugin install agent-critter@agent-critter
   ```

3. 让插件生效：
   ```
   /reload-plugins
   ```
