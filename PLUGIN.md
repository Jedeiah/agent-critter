# Agent Critter — 插件使用指南

## 安装

1. 在 Claude Code 中添加本地 marketplace：
   ```
   /plugin marketplace add <插件包目录路径>
   ```
2. 安装插件：
   ```
   /plugin install agent-critter@agent-critter
   ```
3. 重启 Claude Code 或执行：
   ```
   /reload-plugins
   ```

## 使用

插件安装后会自动启动桌宠守护进程。无需额外配置。

### 交互

| 操作 | 效果 |
|------|------|
| **单击宠物** | 随机互动文案 + 动作 |
| **双击宠物** | 显示当前 Hook 状态 |
| **右键** | 宠物列表 + 缩放调节 + 退出 |
| **拖拽宠物** | 移动桌宠位置 |
| **缩放** | 右键菜单 `−` `+` 按钮调节 0.5x ~ 1.5x |

### 下载更多宠物

```bash
npx -y petdex install <名字>
```

右键菜单中可切换已安装的宠物。

## 卸载

```
/plugin uninstall agent-critter
```

## 故障排除

### 桌宠不显示

确保二进制文件 `agent-critter` 在插件目录根目录下，且具有执行权限：
```bash
chmod +x agent-critter
```

### 端口冲突

桌宠使用端口 7890。如果冲突，杀掉旧进程：
```bash
pkill agent-critter
```
