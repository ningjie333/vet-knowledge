/**
 * ESLint 9 flat config —— Vue 3 + TypeScript 前端。
 *
 * 规则基调（刻意保守，只开"能抓真问题"的等级，风格噪音交给 prettier）：
 *   1. eslint:recommended                —— JS/TS 通用正确性
 *   2. typescript-eslint recommended     —— non-type-checked 版（不依赖 project service，避免类型层噪音）
 *   3. eslint-plugin-vue flat/essential  —— 等价于 vue/essential：只拦会导致运行时/模板失效的写法
 *   4. eslint-config-prettier            —— 放在最后，关闭所有与 prettier 冲突的格式类规则
 *
 * 约定：缩进/引号/换行等格式问题一律由 prettier 负责，eslint 不做重复报错。
 */
import js from '@eslint/js'
import tseslint from 'typescript-eslint'
import pluginVue from 'eslint-plugin-vue'
import prettierConfig from 'eslint-config-prettier'
import globals from 'globals'

// <script lang="ts"> 与 .ts 同等待遇：TS 规则要覆盖到 .vue，
// 否则视图里的类型退化（any、未用变量）就成了 lint 盲区。
const scriptFiles = ['**/*.ts', '**/*.vue']

export default tseslint.config(
  {
    name: 'vet-knowledge/ignore',
    // 只 lint 前端源码（src/）与仓库自身的配置文件。
    // 其余目录要么不是 JS/TS（data/、_knowledge/ 是知识库 JSON，tools/ 是 Python 脚本），
    // 要么由他人负责（src-tauri/ 属 Rust），要么是构建产物/缓存。
    ignores: [
      'node_modules/**',
      'dist/**',
      'src-tauri/**',
      'docs/**',
      'data/**',
      'tools/**',
      '_tools/**',
      '_knowledge/**',
      'vendor/**',
      'repro_test/**',
      '.npm-cache/**',
      '.ui-probe/**',
      '**/*.md',
      '**/*.{txt,log,bat,json,html,css}',
    ],
  },

  js.configs.recommended,

  {
    name: 'vet-knowledge/typescript',
    files: scriptFiles,
    extends: [tseslint.configs.recommended],
    languageOptions: {
      ecmaVersion: 'latest',
      sourceType: 'module',
      parserOptions: {
        ecmaVersion: 'latest',
        sourceType: 'module',
      },
    },
    rules: {
      // 未用标识符：基础规则不认识 TS 的类型标注/装饰器语义，交给 TS 版；
      // 前缀 `_` 的形参/捕获变量是"刻意不用"的约定写法，放行。
      'no-unused-vars': 'off',
      '@typescript-eslint/no-unused-vars': [
        'error',
        {
          args: 'after-used',
          argsIgnorePattern: '^_',
          caughtErrorsIgnorePattern: '^_',
          varsIgnorePattern: '^_',
        },
      ],
      // `any` 会抹掉实体类型的约束（本轮 G4 就是把散落的局部类型收回 @/types），
      // 保持 error：确有必要时用 eslint-disable-next-line 写明理由。
      '@typescript-eslint/no-explicit-any': 'error',
    },
  },

  {
    name: 'vet-knowledge/vue',
    files: ['**/*.vue'],
    extends: [pluginVue.configs['flat/essential']],
    languageOptions: {
      // vue-eslint-parser 负责拆 SFC，<script lang="ts"> 内层交给 TS 解析器。
      globals: { ...globals.browser },
      parserOptions: {
        parser: tseslint.parser,
        ecmaVersion: 'latest',
        sourceType: 'module',
      },
    },
    rules: {
      // 模板里的迭代变量同样受约束；`v-for="(_, i) in n"` 的下标占位放行（由 argsIgnorePattern 兜住）。
      'vue/no-unused-vars': 'error',
      // Home.vue / Sidebar.vue 是既有单字组件名（规则本身已豁免 App）。它们不会与原生 HTML 元素重名，
      // 而重命名要牵动 router 与全站引用 —— 不属于"lint 配套"该做的改动，这里按规则提供的
      // ignores 名单逐个豁免（不是关掉规则，新增单字组件仍会报错）。
      'vue/multi-word-component-names': ['error', { ignores: ['Home', 'Sidebar'] }],
      // self-closing / 属性顺序等排版类规则属于 vue/strongly-recommended，
      // 与 prettier 打架，故停留在 essential 级（见本文件顶部说明）。
    },
  },

  {
    name: 'vet-knowledge/node-config-files',
    files: ['vite.config.ts', 'eslint.config.js'],
    languageOptions: {
      // 这两个文件跑在 Node 构建环境，需要 __dirname / process 等 Node 全局。
      globals: { ...globals.node },
    },
  },

  // 必须放最后：关掉与 prettier 冲突的规则（quotes / semi / 换行 / vue 模板排版…）。
  prettierConfig,
)
