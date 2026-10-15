/// <reference types="vite/client" />

// 兜底声明：只在类型检查器无法解析具体 .vue 文件时生效（本项目走 vue-tsc，会直接拿到真实组件类型）。
// 形参用 unknown 而不是 any —— any 会把整条组件类型链抹平，正是本轮 lint 要拦住的退化点。
declare module '*.vue' {
  import type { DefineComponent } from 'vue'
  const component: DefineComponent<Record<string, unknown>, Record<string, unknown>, unknown>
  export default component
}
