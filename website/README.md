# OpenSunstar 官网（GitHub Pages）

面向 [opensunstar.github.io](https://opensunstar.github.io/) 的静态发布站点，风格参考 [Yuque AI Ecosystem](https://yuque.github.io/yuque-ecosystem/)。

## 本地预览

```bash
# 在项目根目录（推荐，端口 3000）
pnpm website:dev

# 或在 website 目录
cd website
npx serve . -l 3000
```

浏览器打开 `http://localhost:3000`，修改 CSS 后请 **硬刷新**（Ctrl+Shift+R）或确认加载 `style.css?v=3`。

视觉风格对齐 [Yuque AI Ecosystem](https://yuque.github.io/yuque-ecosystem/)：深色全局背景 `#0d1117`、卡片 `#161b22`、主色 `#00B96B`。

## 发布到 opensunstar.github.io

### 方式 A：独立 Pages 仓库（推荐）

1. 创建 GitHub 仓库 `opensunstar/opensunstar.github.io`
2. 将 `website/` 目录下**全部文件**复制到该仓库根目录（不是整个 `website` 文件夹）
3. 推送至 `main` 分支
4. GitHub → Settings → Pages → Source: **Deploy from branch / main / root**

约 1–2 分钟后访问 https://opensunstar.github.io/

### 方式 B：从本仓库子目录发布

若使用 GitHub Actions 从 monorepo 部署，可参考：

```yaml
# .github/workflows/pages.yml（示例，需放在 opensunstar.github.io 仓库或本仓库）
name: Deploy Pages
on:
  push:
    branches: [main]
    paths: ['website/**']
jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/upload-pages-artifact@v3
        with:
          path: website
      - uses: actions/deploy-pages@v4
```

## 目录结构

```
website/
├── index.html          # 单页官网
├── css/style.css       # 样式（语雀生态风格：留白、卡片、分区锚点）
├── js/main.js          # 导航高亮、平台 Tab、复制命令
├── assets/
│   ├── icon.png        # Favicon（512，透明 PNG）
│   ├── logo-nav.png    # 导航品牌 LOGO（32）
│   ├── logo-sm.png     # 页脚 / Hero 示意（22）
│   └── screenshots/    # 产品截图
└── README.md
```

## 更新版本号

发布新版本时，同步修改 `index.html` 中的版本号（当前 **v0.1.0**）及下载文件名。

## 资源说明

- 截图来自 `assets/screenshots/`（与主仓库 README 同源）
- 下载链接指向 [GitHub Releases](https://github.com/alisunstar/OpenSunstar/releases/latest)
- 官方站点链接：https://OpenSunstar.io
