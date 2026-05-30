# Agent Critter — 插件使用指南

## 安装方式

### 方式一：GitHub 市场（推荐）

无需下载任何东西，直接在 Claude Code 中添加：

```
/plugin marketplace add github.com/Jedeiah/agent-critter
/plugin install agent-critter@agent-critter
```

Claude Code 会自动从 GitHub 拉取插件配置和二进制文件。

### 方式二：本地安装

1. 从 [Releases](https://github.com/Jedeiah/agent-critter/releases) 下载插件包并解压
2. 添加本地 marketplace：
   ```
   /plugin marketplace add /path/to/解压目录
   ```
3. 安装：
   ```
   /plugin install agent-critter@agent-critter
   ```

## 使用

插件安装后自动启动桌宠。无需额外配置。

### 交互

| 操作 | 效果 |
|------|------|
| **单击宠物** | 随机互动文案 + 动作 |
| **双击宠物** | 显示当前 Hook 状态（空闲/工作中/等待确认...） |
| **右键** | 打开菜单（见下方） |

### 右键菜单

```
🐾 切换宠物
  ├─ Boba / Dwight / ...  ← 已安装的宠物，点击切换
  ├──────────────────────
  🔍 大小 x1.0  [−] [+]  ← 缩放 0.5x ~ 1.5x，窗口跟随
  ├──────────────────────
  × 退出                 ← 关闭桌宠
  ⭐ Star on GitHub      ← 打开项目主页
```
| **拖拽** | 移动桌宠位置（拖背景框） |

### 安装更多宠物

支持 Petdex 社区 2700+ 精灵：

```bash
npx -y petdex install <名字>
```

右键菜单可切换已安装宠物。

## 状态说明

桌宠会根据 Claude Code 的工作状态自动切换动画：

| 动画 | 触发条件 |
|------|---------|
| 😴 呼吸待机 | 空闲 / SessionStart / Stop |
| 🏃 左右奔跑 | 工作中（PreToolUse / PostToolUse 等） |
| ⏳ 等待 | 等待确认（PermissionRequest / 弹窗） |
| 🔍 检查 | 工具异常（PostToolUseFailure / 限流） |
| 💥 崩溃 | 严重错误（认证失败 / 账单 / 模型） |

## 关于 Petdex

Agent Critter 完全兼容 [Petdex](https://petdex.crafter.run) 社区生态：

- **2700+ 精灵** — 免费开源，一键安装
- **社区创作** — 用户可上传图片生成自定义精灵（[Submit](https://petdex.crafter.run)）
- **精灵格式** — 单张 8 列 × 9 行 spritesheet（webp），每行一个动作

```bash
# 浏览精灵库
open https://petdex.crafter.run

# 安装任意精灵
npx petdex install <名字>
```

## 数据存储

位置、缩放等配置保存在 `~/.agent-critter/data/`，跨项目持久。

## 卸载

```
/plugin uninstall agent-critter
```

## 故障排除

### 桌宠不显示

```bash
pkill agent-critter
```

然后重启 Claude Code。

### 端口 7890 被占用

```bash
pkill agent-critter
```
