// React Fast Refresh for the modules of `teq --hot`: the part of react-refresh's runtime (MIT,
// Meta Platforms) that a build without signatures needs, so that a development server does not
// depend on react-refresh. `--hot` writes this file into the split output directory, unchanged,
// as `hot-refresh.mjs`, so any server can serve it: a page's entry imports it directly, or a
// plugin injects it, ahead of React DOM.
//
// teq reports every object it constructs (`$hotObj`); the component types among the object's
// fields, and one level down (scalajs-react's components hold theirs in `raw`), are registered as
// families under the object's qualified name and the field path. A module that a hot swap
// re-executes constructs its objects anew (`$hot()`), which registers new types under the same
// names, and the refresh that follows makes React render the mounted instances of the old types
// with the new ones: in place with their hook state for a function component, remounted for a
// class, whose instance keeps the methods of its old prototype.
//
// Loaded before React DOM, whose development build hands the refresh entry points to the DevTools
// hook while it initialises. It does nothing without a development server's `import.meta.hot`
// (under node, in a bundle). teq writes it with the footer every module of `--hot` has.
//
// The part of react-refresh's runtime is adapted from React's
// packages/react-refresh/src/ReactFreshRuntime.js (https://github.com/facebook/react), under
// the MIT License, whose copyright and permission notice follow whole:
//
// MIT License
//
// Copyright (c) Meta Platforms, Inc. and affiliates.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

const FORWARD_REF = Symbol.for("react.forward_ref")
const MEMO = Symbol.for("react.memo")

const familiesById = new Map()
const familyOfType = new WeakMap()
const editedFamilyOfType = new WeakMap()
let pending = []
const renderers = new Map()
const rendererOfRoot = new Map()
const mountedRoots = new Set()
const refreshes = []
let scheduled = false
let refreshing = false

const read = (object, key) => {
  try {
    return object[key]
  } catch {
    return undefined
  }
}

const wrapperKind = (value) => (value !== null && typeof value === "object" ? read(value, "$$typeof") : undefined)

const isComponentType = (value) => {
  if (typeof value === "function") return true
  const kind = wrapperKind(value)
  return kind === MEMO || kind === FORWARD_REF
}

const isClass = (type) => typeof type === "function" && !!type.prototype?.isReactComponent

function register(type, id) {
  if (familyOfType.has(type)) return
  const family = familiesById.get(id)
  if (family === undefined) {
    familiesById.set(id, { current: type })
  } else {
    pending.push([family, type])
    schedule()
  }
  familyOfType.set(type, familiesById.get(id))
  const kind = wrapperKind(type)
  if (kind === FORWARD_REF) register(type.render, `${id}$render`)
  else if (kind === MEMO) register(type.type, `${id}$type`)
}

function registerObject(object, id) {
  for (const key of Object.keys(object)) {
    const value = read(object, key)
    if (isComponentType(value)) {
      register(value, `${id}.${key}`)
    } else if (value !== null && typeof value === "object" && !Array.isArray(value) && wrapperKind(value) === undefined) {
      for (const inner of Object.keys(value)) {
        const type = read(value, inner)
        if (isComponentType(type)) register(type, `${id}.${key}.${inner}`)
      }
    }
  }
}

// The objects of a swap are constructed in the microtasks after the re-executed modules ran; one
// refresh after all of them.
function schedule() {
  if (scheduled) return
  scheduled = true
  setTimeout(refresh, 0)
}

function refresh() {
  scheduled = false
  if (pending.length === 0 || refreshing) return
  refreshing = true
  const start = performance.now()
  try {
    const updatedFamilies = new Set()
    const staleFamilies = new Set()
    for (const [family, next] of pending) {
      const previous = family.current
      editedFamilyOfType.set(previous, family)
      editedFamilyOfType.set(next, family)
      family.current = next
      ;(isClass(previous) || isClass(next) ? staleFamilies : updatedFamilies).add(family)
    }
    pending = []
    for (const renderer of renderers.values()) renderer.setRefreshHandler((type) => editedFamilyOfType.get(type))
    for (const root of [...mountedRoots]) rendererOfRoot.get(root).scheduleRefresh(root, { updatedFamilies, staleFamilies })
    refreshes.push({ at: start, ms: performance.now() - start, updated: updatedFamilies.size, stale: staleFamilies.size })
  } finally {
    refreshing = false
  }
}

function injectIntoGlobalHook(global) {
  let hook = global.__REACT_DEVTOOLS_GLOBAL_HOOK__
  if (hook === undefined) {
    let nextId = 0
    hook = global.__REACT_DEVTOOLS_GLOBAL_HOOK__ = {
      renderers: new Map(),
      supportsFiber: true,
      inject: () => nextId++,
      onScheduleFiberRoot() {},
      onCommitFiberRoot() {},
      onCommitFiberUnmount() {},
    }
  }
  if (hook.isDisabled) return
  const supportsRefresh = (injected) => typeof injected.scheduleRefresh === "function" && typeof injected.setRefreshHandler === "function"
  const inject = hook.inject
  hook.inject = function (injected) {
    const id = inject.apply(this, arguments)
    if (supportsRefresh(injected)) renderers.set(id, injected)
    return id
  }
  hook.renderers.forEach((injected, id) => supportsRefresh(injected) && renderers.set(id, injected))
  const onCommitFiberRoot = hook.onCommitFiberRoot
  hook.onCommitFiberRoot = function (id, root) {
    const renderer = renderers.get(id)
    if (renderer !== undefined) {
      rendererOfRoot.set(root, renderer)
      if (root.current.memoizedState?.element != null) mountedRoots.add(root)
      else mountedRoots.delete(root)
    }
    return onCommitFiberRoot.apply(this, arguments)
  }
}

if (import.meta.hot) {
  injectIntoGlobalHook(window)
  globalThis.$teqHotObject = registerObject
  window.__teqHotRefresh = { familiesById, renderers, mountedRoots, refreshes }
}
