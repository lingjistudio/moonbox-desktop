# Microsoft Store 上架配置指南

本文记录将 MoonProxy 发布到 Microsoft Store（Win32 EXE 通道）所需的 Partner Center 配置、Azure AD 授权和 GitHub Actions Secrets 配置步骤。

---

## 前置条件

| 条件 | 说明 |
|---|---|
| Microsoft 开发者账号 | 在 [Partner Center](https://partner.microsoft.com) 注册，个人 $19（一次性），企业 $99/年 |
| Azure 账号 | 用于创建 Azure AD 应用注册（可与开发者账号同一 Microsoft 账号） |
| 应用已在 Partner Center 创建 | CI 只能发布**更新**，首次提交需手动完成 |

---

## 一、在 Partner Center 创建应用

1. 登录 [Partner Center](https://partner.microsoft.com/dashboard)
2. 左侧导航 → **应用和游戏** → **新产品** → **应用**
3. 选择类型：**EXE 或 MSI 应用**（不是 MSIX/PWA）
4. 输入应用名称：`月神代理` 或 `MoonProxy`，点击**保留产品名称**
5. 记录页面顶部的**产品 ID**（格式类似 `9NXXXXXXXX`）→ 填入 Secret `PARTNER_CENTER_PRODUCT_ID`

---

## 二、关联 Azure AD 目录

Partner Center 需要关联一个 Azure AD 目录才能创建 Service Principal 用于 CI 鉴权。

1. Partner Center → 右上角齿轮 → **账号设置**
2. 左侧 → **租户** → **关联 Azure AD**
3. 按提示使用你的 Azure 账号登录并授权关联
4. 关联完成后，记录页面显示的**租户 ID**（格式：`xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`）→ 填入 Secret `PARTNER_CENTER_TENANT_ID`

---

## 三、创建 Azure AD 应用注册（Service Principal）

CI 流水线通过 Azure AD 应用注册的凭据调用 Partner Center API。

### 3.1 创建应用注册

1. 打开 [Azure Portal](https://portal.azure.com) → **Microsoft Entra ID**（即 Azure AD）
2. 左侧 → **应用注册** → **新注册**
3. 填写：
   - 名称：`MoonProxy-GitHub-CI`（或任意便于识别的名称）
   - 受支持的账户类型：**仅此组织目录中的账户**
   - 重定向 URI：留空
4. 点击**注册**
5. 在应用概览页记录**应用程序（客户端）ID** → 填入 Secret `PARTNER_CENTER_CLIENT_ID`

### 3.2 创建客户端密码

1. 应用注册页面 → 左侧 **证书和密码** → **客户端密码** → **新建客户端密码**
2. 描述：`github-actions`，有效期：**24 个月**（到期前需轮换）
3. 点击**添加**，立即复制显示的**密码值**（离开页面后无法再查看）→ 填入 Secret `PARTNER_CENTER_CLIENT_SECRET`

### 3.3 在 Partner Center 添加该应用

1. Partner Center → **账号设置** → **用户管理** → **Azure AD 应用程序**
2. 点击**添加 Azure AD 应用程序**
3. 搜索并选择刚创建的 `MoonProxy-GitHub-CI`
4. 角色选择：**开发人员**（Developer）
5. 点击**添加**

---

## 四、获取 Seller ID

1. Partner Center → **账号设置** → **组织资料** → **法律信息** → **开发者设置**
2. 找到 **卖方 ID（Seller ID）**（纯数字，如 `12345678`）→ 填入 Secret `PARTNER_CENTER_SELLER_ID`

---

## 五、配置 GitHub Actions Secrets

在 GitHub 仓库 → **Settings** → **Secrets and variables** → **Actions** 中添加以下 5 个 Secret：

| Secret 名称 | 值来源 | 示例格式 |
|---|---|---|
| `PARTNER_CENTER_TENANT_ID` | Azure AD 租户 ID（步骤二） | `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` |
| `PARTNER_CENTER_SELLER_ID` | Partner Center 卖方 ID（步骤四） | `12345678` |
| `PARTNER_CENTER_CLIENT_ID` | Azure AD 应用注册客户端 ID（步骤三） | `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` |
| `PARTNER_CENTER_CLIENT_SECRET` | Azure AD 客户端密码值（步骤三） | 随机字符串 |
| `PARTNER_CENTER_PRODUCT_ID` | Partner Center 产品 ID（步骤一） | `9NXXXXXXXX` |

---

## 六、首次手动提交

CI 只能提交**更新**到已存在的草稿提交（Submission）。首次上架必须手动完成：

1. Partner Center → 应用页面 → **开始提交**
2. **定价和可用性**：选择国家/地区、价格（免费）
3. **属性**：
   - 类别：**生产力 > 实用工具**
   - 隐私政策 URL（如有）
4. **年龄分级**：填写问卷，通常评级为 **3+**
5. **程序包**：
   - 上传一个手动构建的 Store 版安装包（`pnpm tauri build --no-bundle && pnpm tauri bundle --config src-tauri/tauri.microsoftstore.conf.json`）
   - 安装程序参数填：`/S`
   - 勾选**支持无提示安装**
6. **Store 一览**：上传截图（至少 1 张 1366×768 或更大）、填写说明文字
7. 点击**提交到 Store**，等待微软审核（通常 1–3 个工作日）

首次审核通过后，后续版本更新即可通过 CI 自动提交。

---

## 七、CI 自动发布流程（上架后）

推送 tag 触发 CI，`publish-store` job 自动执行：

```
tag 推送
  └─→ build-x86_64-pc-windows-msvc-store
        ├─→ 编译（tauri build --no-bundle）
        └─→ 打包（tauri bundle --config tauri.microsoftstore.conf.json）
              产物：*_x64-setup.exe（含离线 WebView2，约 150MB）

  └─→ publish-store（needs: build）
        ├─→ 下载 store-x64 artifact
        ├─→ 上传到 S3：store/<tag>/*-setup.exe
        ├─→ msstore reconfigure（Partner Center 鉴权）
        ├─→ msstore submission update（提交 CloudFront CDN URL）
        └─→ msstore submission publish（触发 Store 审核）
```

提交后 Partner Center 进入**待审核**状态，微软通常在 1–3 个工作日内完成审核。

---

## 八、注意事项

### 与应用内更新的关系

Store 版本**不走 Tauri updater**（`tauri-plugin-updater`），用户通过 Microsoft Store 自动更新。两个渠道的版本号应保持一致（`tauri.conf.json` 中的 `version` 字段），但更新机制独立。

### 客户端密码轮换

`PARTNER_CENTER_CLIENT_SECRET` 有效期 24 个月，到期前需在 Azure AD 创建新密码并更新 GitHub Secret，否则 CI 的 Store 提交步骤会失败。

建议在日历中提前 30 天设置提醒。

### Store 审核时效

微软审核不保证时效，发布计划应预留 3–5 个工作日缓冲。紧急修复优先走 GitHub Release 渠道，Store 渠道随下次版本跟进。

### IAM 最小权限

`PARTNER_CENTER_CLIENT_ID` 对应的 Azure AD 应用只需 Partner Center **开发人员**角色，不需要 Azure 订阅级别的权限，IAM 风险面极小。
