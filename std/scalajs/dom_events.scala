package org.scalajs.dom

import scala.scalajs.js
import scala.scalajs.js.annotation.JSGlobal

trait EventListenerOptions extends js.Object:
  var capture: js.UndefOr[Boolean] = js.undefined
  var once: js.UndefOr[Boolean] = js.undefined
  var passive: js.UndefOr[Boolean] = js.undefined
  var signal: js.UndefOr[AbortSignal] = js.undefined

trait EventInit extends js.Object:
  var bubbles: js.UndefOr[Boolean] = js.undefined
  var cancelable: js.UndefOr[Boolean] = js.undefined
  var composed: js.UndefOr[Boolean] = js.undefined

@js.native
@JSGlobal("EventTarget")
class EventTarget extends js.Object:
  def addEventListener[T <: Event](`type`: String, listener: js.Function1[T, Any], useCapture: Boolean | EventListenerOptions = js.native): Unit = js.native
  def removeEventListener[T <: Event](`type`: String, listener: js.Function1[T, Any], useCapture: Boolean | EventListenerOptions = js.native): Unit = js.native
  def dispatchEvent(evt: Event): Boolean = js.native

@js.native
@JSGlobal("Event")
class Event(typeArg: String, init: js.UndefOr[EventInit] = js.native) extends js.Object:
  def `type`: String = js.native
  def target: EventTarget = js.native
  def currentTarget: EventTarget = js.native
  def eventPhase: Int = js.native
  def bubbles: Boolean = js.native
  def cancelable: Boolean = js.native
  def composed: Boolean = js.native
  def defaultPrevented: Boolean = js.native
  def isTrusted: Boolean = js.native
  def timeStamp: Double = js.native
  def preventDefault(): Unit = js.native
  def stopPropagation(): Unit = js.native
  def stopImmediatePropagation(): Unit = js.native
  def composedPath(): js.Array[EventTarget] = js.native

@js.native
@JSGlobal("Event")
object Event extends js.Object:
  val NONE: Int = js.native
  val CAPTURING_PHASE: Int = js.native
  val AT_TARGET: Int = js.native
  val BUBBLING_PHASE: Int = js.native

@js.native
@JSGlobal("UIEvent")
class UIEvent(typeArg: String, init: js.UndefOr[js.Object] = js.native) extends Event:
  def detail: Int = js.native
  def view: Window = js.native

@js.native
@JSGlobal("MouseEvent")
class MouseEvent(typeArg: String, init: js.UndefOr[js.Object] = js.native) extends UIEvent:
  def screenX: Double = js.native
  def screenY: Double = js.native
  def clientX: Double = js.native
  def clientY: Double = js.native
  def pageX: Double = js.native
  def pageY: Double = js.native
  def offsetX: Double = js.native
  def offsetY: Double = js.native
  def movementX: Double = js.native
  def movementY: Double = js.native
  def x: Double = js.native
  def y: Double = js.native
  def button: Int = js.native
  def buttons: Int = js.native
  def relatedTarget: EventTarget = js.native
  def altKey: Boolean = js.native
  def ctrlKey: Boolean = js.native
  def metaKey: Boolean = js.native
  def shiftKey: Boolean = js.native
  def getModifierState(keyArg: String): Boolean = js.native

@js.native
@JSGlobal("PointerEvent")
class PointerEvent(typeArg: String, init: js.UndefOr[js.Object] = js.native) extends MouseEvent:
  def pointerId: Double = js.native
  def width: Double = js.native
  def height: Double = js.native
  def pressure: Double = js.native
  def tangentialPressure: Double = js.native
  def tiltX: Double = js.native
  def tiltY: Double = js.native
  def twist: Double = js.native
  def pointerType: String = js.native
  def isPrimary: Boolean = js.native

@js.native
@JSGlobal("WheelEvent")
class WheelEvent(typeArg: String, init: js.UndefOr[js.Object] = js.native) extends MouseEvent:
  def deltaX: Double = js.native
  def deltaY: Double = js.native
  def deltaZ: Double = js.native
  def deltaMode: Int = js.native

@js.native
@JSGlobal("DragEvent")
class DragEvent(typeArg: String, init: js.UndefOr[js.Object] = js.native) extends MouseEvent:
  def dataTransfer: DataTransfer = js.native

@js.native
@JSGlobal("KeyboardEvent")
class KeyboardEvent(typeArg: String, init: js.UndefOr[js.Object] = js.native) extends UIEvent:
  def key: String = js.native
  def code: String = js.native
  def location: Int = js.native
  def keyCode: Int = js.native
  def charCode: Int = js.native
  def which: Int = js.native
  def repeat: Boolean = js.native
  def isComposing: Boolean = js.native
  def altKey: Boolean = js.native
  def ctrlKey: Boolean = js.native
  def metaKey: Boolean = js.native
  def shiftKey: Boolean = js.native
  def getModifierState(keyArg: String): Boolean = js.native

@js.native
@JSGlobal("KeyboardEvent")
object KeyboardEvent extends js.Object:
  val DOM_KEY_LOCATION_STANDARD: Int = js.native
  val DOM_KEY_LOCATION_LEFT: Int = js.native
  val DOM_KEY_LOCATION_RIGHT: Int = js.native
  val DOM_KEY_LOCATION_NUMPAD: Int = js.native

@js.native
@JSGlobal("TouchEvent")
class TouchEvent(typeArg: String, init: js.UndefOr[js.Object] = js.native) extends UIEvent:
  def touches: TouchList = js.native
  def targetTouches: TouchList = js.native
  def changedTouches: TouchList = js.native
  def altKey: Boolean = js.native
  def ctrlKey: Boolean = js.native
  def metaKey: Boolean = js.native
  def shiftKey: Boolean = js.native

@js.native
@JSGlobal("Touch")
class Touch extends js.Object:
  def identifier: Double = js.native
  def target: EventTarget = js.native
  def screenX: Double = js.native
  def screenY: Double = js.native
  def clientX: Double = js.native
  def clientY: Double = js.native
  def pageX: Double = js.native
  def pageY: Double = js.native
  def radiusX: Double = js.native
  def radiusY: Double = js.native
  def rotationAngle: Double = js.native
  def force: Double = js.native

@js.native
@JSGlobal("TouchList")
class TouchList extends DOMList[Touch]:
  def item(index: Int): Touch = js.native

@js.native
@JSGlobal("FocusEvent")
class FocusEvent(typeArg: String, init: js.UndefOr[js.Object] = js.native) extends UIEvent:
  def relatedTarget: EventTarget = js.native

@js.native
@JSGlobal("InputEvent")
class InputEvent(typeArg: String, init: js.UndefOr[js.Object] = js.native) extends UIEvent:
  def data: String = js.native
  def inputType: String = js.native
  def isComposing: Boolean = js.native
  def dataTransfer: DataTransfer = js.native

@js.native
@JSGlobal("CompositionEvent")
class CompositionEvent(typeArg: String, init: js.UndefOr[js.Object] = js.native) extends UIEvent:
  def data: String = js.native
  def locale: String = js.native

@js.native
@JSGlobal("ClipboardEvent")
class ClipboardEvent(typeArg: String, init: js.UndefOr[js.Object] = js.native) extends Event:
  def clipboardData: DataTransfer = js.native

@js.native
@JSGlobal("AnimationEvent")
class AnimationEvent(typeArg: String, init: js.UndefOr[js.Object] = js.native) extends Event:
  def animationName: String = js.native
  def elapsedTime: Double = js.native
  def pseudoElement: String = js.native

@js.native
@JSGlobal("TransitionEvent")
class TransitionEvent(typeArg: String, init: js.UndefOr[js.Object] = js.native) extends Event:
  def propertyName: String = js.native
  def elapsedTime: Double = js.native
  def pseudoElement: String = js.native

@js.native
@JSGlobal("PopStateEvent")
class PopStateEvent(typeArg: String, init: js.UndefOr[js.Object] = js.native) extends Event:
  def state: js.Any = js.native

@js.native
@JSGlobal("HashChangeEvent")
class HashChangeEvent(typeArg: String, init: js.UndefOr[js.Object] = js.native) extends Event:
  def oldURL: String = js.native
  def newURL: String = js.native

@js.native
@JSGlobal("MessageEvent")
class MessageEvent(typeArg: String, init: js.UndefOr[js.Object] = js.native) extends Event:
  def data: js.Any = js.native
  def origin: String = js.native
  def lastEventId: String = js.native
  def source: js.Any = js.native

@js.native
@JSGlobal("CloseEvent")
class CloseEvent(typeArg: String, init: js.UndefOr[js.Object] = js.native) extends Event:
  def wasClean: Boolean = js.native
  def code: Int = js.native
  def reason: String = js.native

@js.native
@JSGlobal("ErrorEvent")
class ErrorEvent(typeArg: String, init: js.UndefOr[js.Object] = js.native) extends Event:
  def message: String = js.native
  def filename: String = js.native
  def lineno: Int = js.native
  def colno: Int = js.native
  def error: js.Any = js.native

@js.native
@JSGlobal("ProgressEvent")
class ProgressEvent(typeArg: String, init: js.UndefOr[js.Object] = js.native) extends Event:
  def lengthComputable: Boolean = js.native
  def loaded: Double = js.native
  def total: Double = js.native

@js.native
@JSGlobal("CustomEvent")
class CustomEvent(typeArg: String, init: js.UndefOr[js.Object] = js.native) extends Event:
  def detail: js.Any = js.native

@js.native
@JSGlobal("BeforeUnloadEvent")
class BeforeUnloadEvent extends Event:
  var returnValue: String = js.native

@js.native
@JSGlobal("StorageEvent")
class StorageEvent(typeArg: String, init: js.UndefOr[js.Object] = js.native) extends Event:
  def key: String = js.native
  def oldValue: String = js.native
  def newValue: String = js.native
  def url: String = js.native
  def storageArea: Storage = js.native

@js.native
@JSGlobal("DataTransfer")
class DataTransfer extends js.Object:
  var dropEffect: String = js.native
  var effectAllowed: String = js.native
  def files: FileList = js.native
  def types: js.Array[String] = js.native
  def items: DataTransferItemList = js.native
  def setData(format: String, data: String): Unit = js.native
  def getData(format: String): String = js.native
  def clearData(format: String = js.native): Unit = js.native
  def setDragImage(image: Element, x: Double, y: Double): Unit = js.native

@js.native
@JSGlobal("DataTransferItemList")
class DataTransferItemList extends DOMList[DataTransferItem]:
  def add(data: String | File, `type`: String = js.native): DataTransferItem = js.native
  def remove(index: Int): Unit = js.native
  def clear(): Unit = js.native

@js.native
@JSGlobal("DataTransferItem")
class DataTransferItem extends js.Object:
  def kind: String = js.native
  def `type`: String = js.native
  def getAsFile(): File = js.native
  def getAsString(callback: js.Function1[String, Any]): Unit = js.native

@js.native
@JSGlobal("AbortController")
class AbortController() extends js.Object:
  def signal: AbortSignal = js.native
  def abort(reason: js.Any = js.native): Unit = js.native

@js.native
@JSGlobal("AbortSignal")
class AbortSignal extends EventTarget:
  def aborted: Boolean = js.native
  def reason: js.Any = js.native
  var onabort: js.Function1[Event, Any] = js.native
