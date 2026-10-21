export { isSmallModule, taskArgs, watchOptions } from "../description.js"
export function loadDescription(project, root) {
  return { root, inputs: ["src"], modulePerFile: ["app.page"], hotSwapUpTo: 100 }
}
