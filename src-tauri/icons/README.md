# 图标文件说明

此目录应包含应用程序图标文件：

- 32x32.png
- 128x128.png
- 128x128@2x.png
- icon.icns (macOS)
- icon.ico (Windows)

你可以使用 Tauri 图标生成工具来生成这些图标：

```bash
npm install --save-dev @tauri-apps/cli
npm run tauri icon path/to/your/icon.png
```

或者手动创建这些图标文件并放置在此目录中。
