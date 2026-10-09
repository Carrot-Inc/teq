package org.scalajs.dom

import scala.scalajs.js
import scala.scalajs.js.annotation.JSGlobal

@js.native
@JSGlobal("ResizeObserver")
class ResizeObserver(callback: js.Function2[js.Array[ResizeObserverEntry], ResizeObserver, Any]) extends js.Object:
  def observe(target: Element, options: ResizeObserverOptions = js.native): Unit = js.native
  def unobserve(target: Element): Unit = js.native
  def disconnect(): Unit = js.native

@js.native
trait ResizeObserverEntry extends js.Object:
  def target: Element = js.native
  def contentRect: DOMRectReadOnly = js.native
  def borderBoxSize: js.Array[ResizeObserverSize] = js.native
  def contentBoxSize: js.Array[ResizeObserverSize] = js.native
  def devicePixelContentBoxSize: js.Array[ResizeObserverSize] = js.native

@js.native
trait ResizeObserverSize extends js.Object:
  def inlineSize: Double = js.native
  def blockSize: Double = js.native

trait ResizeObserverOptions extends js.Object:
  var box: js.UndefOr[String] = js.undefined

@js.native
@JSGlobal("MutationObserver")
class MutationObserver(callback: js.Function2[js.Array[MutationRecord], MutationObserver, Any]) extends js.Object:
  def observe(target: Node, options: MutationObserverInit = js.native): Unit = js.native
  def disconnect(): Unit = js.native
  def takeRecords(): js.Array[MutationRecord] = js.native

trait MutationObserverInit extends js.Object:
  var childList: js.UndefOr[Boolean] = js.undefined
  var attributes: js.UndefOr[Boolean] = js.undefined
  var characterData: js.UndefOr[Boolean] = js.undefined
  var subtree: js.UndefOr[Boolean] = js.undefined
  var attributeOldValue: js.UndefOr[Boolean] = js.undefined
  var characterDataOldValue: js.UndefOr[Boolean] = js.undefined
  var attributeFilter: js.UndefOr[js.Array[String]] = js.undefined

@js.native
trait MutationRecord extends js.Object:
  def `type`: String = js.native
  def target: Node = js.native
  def addedNodes: NodeList[Node] = js.native
  def removedNodes: NodeList[Node] = js.native
  def previousSibling: Node = js.native
  def nextSibling: Node = js.native
  def attributeName: String = js.native
  def attributeNamespace: String = js.native
  def oldValue: String = js.native

@js.native
@JSGlobal("IntersectionObserver")
class IntersectionObserver(callback: js.Function2[js.Array[IntersectionObserverEntry], IntersectionObserver, Any], options: IntersectionObserverInit = js.native) extends js.Object:
  def root: Element = js.native
  def rootMargin: String = js.native
  def thresholds: js.Array[Double] = js.native
  def observe(target: Element): Unit = js.native
  def unobserve(target: Element): Unit = js.native
  def disconnect(): Unit = js.native
  def takeRecords(): js.Array[IntersectionObserverEntry] = js.native

@js.native
trait IntersectionObserverEntry extends js.Object:
  def target: Element = js.native
  def time: Double = js.native
  def isIntersecting: Boolean = js.native
  def intersectionRatio: Double = js.native
  def boundingClientRect: DOMRectReadOnly = js.native
  def intersectionRect: DOMRectReadOnly = js.native
  def rootBounds: DOMRectReadOnly = js.native

trait IntersectionObserverInit extends js.Object:
  var root: js.UndefOr[Element] = js.undefined
  var rootMargin: js.UndefOr[String] = js.undefined
  var threshold: js.UndefOr[Double | Int | js.Array[Double]] = js.undefined
