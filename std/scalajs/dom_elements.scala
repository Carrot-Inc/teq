package org.scalajs.dom

import scala.scalajs.js
import scala.scalajs.js.annotation.JSGlobal

@js.native
@JSGlobal("HTMLElement")
class HTMLElement extends Element:
  def style: CSSStyleDeclaration = js.native
  def dataset: js.Dictionary[String] = js.native
  def parentElement: HTMLElement = js.native
  def offsetParent: Element = js.native
  def offsetTop: Double = js.native
  def offsetLeft: Double = js.native
  def offsetWidth: Double = js.native
  def offsetHeight: Double = js.native
  var innerText: String = js.native
  var outerText: String = js.native
  var title: String = js.native
  var lang: String = js.native
  var dir: String = js.native
  var hidden: Boolean = js.native
  var inert: Boolean = js.native
  var tabIndex: Int = js.native
  var accessKey: String = js.native
  var draggable: Boolean = js.native
  var spellcheck: Boolean = js.native
  var autofocus: Boolean = js.native
  var contentEditable: String = js.native
  def isContentEditable: Boolean = js.native
  var popover: String = js.native
  def focus(options: js.Object = js.native): Unit = js.native
  def blur(): Unit = js.native
  def click(): Unit = js.native
  def showPopover(): Unit = js.native
  def hidePopover(): Unit = js.native
  var onclick: js.Function1[MouseEvent, Any] = js.native
  var ondblclick: js.Function1[MouseEvent, Any] = js.native
  var oncontextmenu: js.Function1[MouseEvent, Any] = js.native
  var onmousedown: js.Function1[MouseEvent, Any] = js.native
  var onmouseup: js.Function1[MouseEvent, Any] = js.native
  var onmousemove: js.Function1[MouseEvent, Any] = js.native
  var onmouseenter: js.Function1[MouseEvent, Any] = js.native
  var onmouseleave: js.Function1[MouseEvent, Any] = js.native
  var onmouseover: js.Function1[MouseEvent, Any] = js.native
  var onmouseout: js.Function1[MouseEvent, Any] = js.native
  var onwheel: js.Function1[WheelEvent, Any] = js.native
  var onkeydown: js.Function1[KeyboardEvent, Any] = js.native
  var onkeyup: js.Function1[KeyboardEvent, Any] = js.native
  var onkeypress: js.Function1[KeyboardEvent, Any] = js.native
  var onfocus: js.Function1[FocusEvent, Any] = js.native
  var onblur: js.Function1[FocusEvent, Any] = js.native
  var onchange: js.Function1[Event, Any] = js.native
  var oninput: js.Function1[Event, Any] = js.native
  var onsubmit: js.Function1[Event, Any] = js.native
  var onreset: js.Function1[Event, Any] = js.native
  var onscroll: js.Function1[UIEvent, Any] = js.native
  var onload: js.Function1[Event, Any] = js.native
  var onerror: js.Function1[Event, Any] = js.native
  var onabort: js.Function1[UIEvent, Any] = js.native
  var ondrag: js.Function1[DragEvent, Any] = js.native
  var ondragstart: js.Function1[DragEvent, Any] = js.native
  var ondragend: js.Function1[DragEvent, Any] = js.native
  var ondragenter: js.Function1[DragEvent, Any] = js.native
  var ondragleave: js.Function1[DragEvent, Any] = js.native
  var ondragover: js.Function1[DragEvent, Any] = js.native
  var ondrop: js.Function1[DragEvent, Any] = js.native
  var onpointerdown: js.Function1[PointerEvent, Any] = js.native
  var onpointerup: js.Function1[PointerEvent, Any] = js.native
  var onpointermove: js.Function1[PointerEvent, Any] = js.native
  var onpointercancel: js.Function1[PointerEvent, Any] = js.native
  var onpointerenter: js.Function1[PointerEvent, Any] = js.native
  var onpointerleave: js.Function1[PointerEvent, Any] = js.native
  var onpointerover: js.Function1[PointerEvent, Any] = js.native
  var onpointerout: js.Function1[PointerEvent, Any] = js.native
  var ontouchstart: js.Function1[TouchEvent, Any] = js.native
  var ontouchmove: js.Function1[TouchEvent, Any] = js.native
  var ontouchend: js.Function1[TouchEvent, Any] = js.native
  var ontouchcancel: js.Function1[TouchEvent, Any] = js.native
  var oncopy: js.Function1[ClipboardEvent, Any] = js.native
  var oncut: js.Function1[ClipboardEvent, Any] = js.native
  var onpaste: js.Function1[ClipboardEvent, Any] = js.native
  var onanimationend: js.Function1[AnimationEvent, Any] = js.native
  var ontransitionend: js.Function1[TransitionEvent, Any] = js.native

@js.native
@JSGlobal("HTMLDivElement")
class HTMLDivElement extends HTMLElement

@js.native
@JSGlobal("HTMLSpanElement")
class HTMLSpanElement extends HTMLElement

@js.native
@JSGlobal("HTMLParagraphElement")
class HTMLParagraphElement extends HTMLElement

@js.native
@JSGlobal("HTMLHeadingElement")
class HTMLHeadingElement extends HTMLElement

@js.native
@JSGlobal("HTMLPreElement")
class HTMLPreElement extends HTMLElement

@js.native
@JSGlobal("HTMLBRElement")
class HTMLBRElement extends HTMLElement

@js.native
@JSGlobal("HTMLHRElement")
class HTMLHRElement extends HTMLElement

@js.native
@JSGlobal("HTMLBodyElement")
class HTMLBodyElement extends HTMLElement

@js.native
@JSGlobal("HTMLHeadElement")
class HTMLHeadElement extends HTMLElement

@js.native
@JSGlobal("HTMLHtmlElement")
class HTMLHtmlElement extends HTMLElement

@js.native
@JSGlobal("HTMLUListElement")
class HTMLUListElement extends HTMLElement

@js.native
@JSGlobal("HTMLOListElement")
class HTMLOListElement extends HTMLElement:
  var start: Int = js.native
  var reversed: Boolean = js.native
  var `type`: String = js.native

@js.native
@JSGlobal("HTMLLIElement")
class HTMLLIElement extends HTMLElement:
  var value: Int = js.native

@js.native
@JSGlobal("HTMLDListElement")
class HTMLDListElement extends HTMLElement

@js.native
@JSGlobal("HTMLAnchorElement")
class HTMLAnchorElement extends HTMLElement:
  var href: String = js.native
  var target: String = js.native
  var download: String = js.native
  var rel: String = js.native
  var hreflang: String = js.native
  var `type`: String = js.native
  var text: String = js.native
  var protocol: String = js.native
  var host: String = js.native
  var hostname: String = js.native
  var port: String = js.native
  var pathname: String = js.native
  var search: String = js.native
  var hash: String = js.native
  def origin: String = js.native

@js.native
@JSGlobal("HTMLImageElement")
class HTMLImageElement extends HTMLElement:
  var src: String = js.native
  var srcset: String = js.native
  var sizes: String = js.native
  var alt: String = js.native
  var width: Int = js.native
  var height: Int = js.native
  def naturalWidth: Int = js.native
  def naturalHeight: Int = js.native
  def complete: Boolean = js.native
  def currentSrc: String = js.native
  var crossOrigin: String = js.native
  var loading: String = js.native
  var decoding: String = js.native
  var referrerPolicy: String = js.native
  var isMap: Boolean = js.native
  var useMap: String = js.native
  def x: Int = js.native
  def y: Int = js.native
  def decode(): js.Promise[Unit] = js.native

/** `new Image()` and `new Image(width, height)` build an img element. */
@js.native
@JSGlobal("Image")
class Image(widthArg: Int = js.native, heightArg: Int = js.native) extends HTMLImageElement

@js.native
@JSGlobal("HTMLCanvasElement")
class HTMLCanvasElement extends HTMLElement:
  var width: Int = js.native
  var height: Int = js.native
  def getContext(contextId: String, options: js.Any = js.native): js.Dynamic = js.native
  def toDataURL(`type`: String = js.native, quality: js.Any = js.native): String = js.native
  def toBlob(callback: js.Function1[Blob, Any], `type`: String = js.native, quality: js.Any = js.native): Unit = js.native

@js.native
@JSGlobal("HTMLInputElement")
class HTMLInputElement extends HTMLElement:
  var value: String = js.native
  var defaultValue: String = js.native
  var `type`: String = js.native
  var name: String = js.native
  var checked: Boolean = js.native
  var defaultChecked: Boolean = js.native
  var indeterminate: Boolean = js.native
  var disabled: Boolean = js.native
  var readOnly: Boolean = js.native
  var required: Boolean = js.native
  var multiple: Boolean = js.native
  var placeholder: String = js.native
  var accept: String = js.native
  var autocomplete: String = js.native
  var pattern: String = js.native
  var min: String = js.native
  var max: String = js.native
  var step: String = js.native
  var minLength: Int = js.native
  var maxLength: Int = js.native
  var size: Int = js.native
  var src: String = js.native
  var alt: String = js.native
  var width: Int = js.native
  var height: Int = js.native
  var valueAsNumber: Double = js.native
  var valueAsDate: js.Date = js.native
  var selectionStart: Int = js.native
  var selectionEnd: Int = js.native
  var selectionDirection: String = js.native
  var files: FileList = js.native
  def form: HTMLFormElement = js.native
  def list: HTMLElement = js.native
  def labels: NodeList[HTMLLabelElement] = js.native
  def validity: ValidityState = js.native
  def validationMessage: String = js.native
  def willValidate: Boolean = js.native
  def select(): Unit = js.native
  def setSelectionRange(start: Int, end: Int, direction: String = js.native): Unit = js.native
  def setRangeText(replacement: String, start: Int = js.native, end: Int = js.native, selectionMode: String = js.native): Unit = js.native
  def stepUp(n: Int = js.native): Unit = js.native
  def stepDown(n: Int = js.native): Unit = js.native
  def checkValidity(): Boolean = js.native
  def reportValidity(): Boolean = js.native
  def setCustomValidity(error: String): Unit = js.native
  def showPicker(): Unit = js.native

@js.native
@JSGlobal("ValidityState")
class ValidityState extends js.Object:
  def valid: Boolean = js.native
  def valueMissing: Boolean = js.native
  def typeMismatch: Boolean = js.native
  def patternMismatch: Boolean = js.native
  def tooLong: Boolean = js.native
  def tooShort: Boolean = js.native
  def rangeUnderflow: Boolean = js.native
  def rangeOverflow: Boolean = js.native
  def stepMismatch: Boolean = js.native
  def badInput: Boolean = js.native
  def customError: Boolean = js.native

@js.native
@JSGlobal("HTMLTextAreaElement")
class HTMLTextAreaElement extends HTMLElement:
  var value: String = js.native
  var defaultValue: String = js.native
  var name: String = js.native
  var placeholder: String = js.native
  var rows: Int = js.native
  var cols: Int = js.native
  var wrap: String = js.native
  var disabled: Boolean = js.native
  var readOnly: Boolean = js.native
  var required: Boolean = js.native
  var minLength: Int = js.native
  var maxLength: Int = js.native
  var selectionStart: Int = js.native
  var selectionEnd: Int = js.native
  var selectionDirection: String = js.native
  def `type`: String = js.native
  def form: HTMLFormElement = js.native
  def textLength: Int = js.native
  def validity: ValidityState = js.native
  def validationMessage: String = js.native
  def select(): Unit = js.native
  def setSelectionRange(start: Int, end: Int, direction: String = js.native): Unit = js.native
  def setRangeText(replacement: String, start: Int = js.native, end: Int = js.native, selectionMode: String = js.native): Unit = js.native
  def checkValidity(): Boolean = js.native
  def reportValidity(): Boolean = js.native
  def setCustomValidity(error: String): Unit = js.native

@js.native
@JSGlobal("HTMLSelectElement")
class HTMLSelectElement extends HTMLElement:
  var value: String = js.native
  var name: String = js.native
  var selectedIndex: Int = js.native
  var disabled: Boolean = js.native
  var required: Boolean = js.native
  var multiple: Boolean = js.native
  var size: Int = js.native
  var length: Int = js.native
  def `type`: String = js.native
  def form: HTMLFormElement = js.native
  def options: HTMLCollection[HTMLOptionElement] = js.native
  def selectedOptions: HTMLCollection[HTMLOptionElement] = js.native
  def item(index: Int): HTMLOptionElement = js.native
  def namedItem(name: String): HTMLOptionElement = js.native
  def add(element: HTMLElement, before: HTMLElement | Int = js.native): Unit = js.native
  def remove(index: Int = js.native): Unit = js.native
  def checkValidity(): Boolean = js.native
  def reportValidity(): Boolean = js.native
  def setCustomValidity(error: String): Unit = js.native

@js.native
@JSGlobal("HTMLOptionElement")
class HTMLOptionElement extends HTMLElement:
  var value: String = js.native
  var text: String = js.native
  var label: String = js.native
  var selected: Boolean = js.native
  var defaultSelected: Boolean = js.native
  var disabled: Boolean = js.native
  def index: Int = js.native
  def form: HTMLFormElement = js.native

@js.native
@JSGlobal("HTMLOptGroupElement")
class HTMLOptGroupElement extends HTMLElement:
  var label: String = js.native
  var disabled: Boolean = js.native

@js.native
@JSGlobal("HTMLButtonElement")
class HTMLButtonElement extends HTMLElement:
  var `type`: String = js.native
  var name: String = js.native
  var value: String = js.native
  var disabled: Boolean = js.native
  var formAction: String = js.native
  var formMethod: String = js.native
  var formTarget: String = js.native
  var popoverTargetAction: String = js.native
  def form: HTMLFormElement = js.native
  def validity: ValidityState = js.native
  def validationMessage: String = js.native
  def checkValidity(): Boolean = js.native
  def reportValidity(): Boolean = js.native
  def setCustomValidity(error: String): Unit = js.native

@js.native
@JSGlobal("HTMLLabelElement")
class HTMLLabelElement extends HTMLElement:
  var htmlFor: String = js.native
  def control: HTMLElement = js.native
  def form: HTMLFormElement = js.native

@js.native
@JSGlobal("HTMLFormElement")
class HTMLFormElement extends HTMLElement:
  var action: String = js.native
  var method: String = js.native
  var enctype: String = js.native
  var encoding: String = js.native
  var target: String = js.native
  var name: String = js.native
  var autocomplete: String = js.native
  var noValidate: Boolean = js.native
  var acceptCharset: String = js.native
  def length: Int = js.native
  def elements: HTMLCollection[Element] = js.native
  def submit(): Unit = js.native
  def requestSubmit(submitter: HTMLElement = js.native): Unit = js.native
  def reset(): Unit = js.native
  def checkValidity(): Boolean = js.native
  def reportValidity(): Boolean = js.native

@js.native
@JSGlobal("HTMLFieldSetElement")
class HTMLFieldSetElement extends HTMLElement:
  var disabled: Boolean = js.native
  var name: String = js.native
  def elements: HTMLCollection[Element] = js.native

@js.native
@JSGlobal("HTMLLegendElement")
class HTMLLegendElement extends HTMLElement

@js.native
@JSGlobal("HTMLDialogElement")
class HTMLDialogElement extends HTMLElement:
  var open: Boolean = js.native
  var returnValue: String = js.native
  def show(): Unit = js.native
  def showModal(): Unit = js.native
  def close(returnValue: String = js.native): Unit = js.native
  var onclose: js.Function1[Event, Any] = js.native
  var oncancel: js.Function1[Event, Any] = js.native

@js.native
@JSGlobal("HTMLIFrameElement")
class HTMLIFrameElement extends HTMLElement:
  var src: String = js.native
  var srcdoc: String = js.native
  var name: String = js.native
  var width: String = js.native
  var height: String = js.native
  var allow: String = js.native
  var allowFullscreen: Boolean = js.native
  var referrerPolicy: String = js.native
  var loading: String = js.native
  def sandbox: DOMTokenList = js.native
  def contentWindow: Window = js.native
  def contentDocument: Document = js.native

@js.native
@JSGlobal("HTMLLinkElement")
class HTMLLinkElement extends HTMLElement:
  var href: String = js.native
  var rel: String = js.native
  var `type`: String = js.native
  var media: String = js.native
  var as: String = js.native
  var crossOrigin: String = js.native
  var integrity: String = js.native
  var hreflang: String = js.native
  var disabled: Boolean = js.native
  def relList: DOMTokenList = js.native
  def sheet: js.Object = js.native

@js.native
@JSGlobal("HTMLStyleElement")
class HTMLStyleElement extends HTMLElement:
  var media: String = js.native
  var `type`: String = js.native
  var disabled: Boolean = js.native
  def sheet: js.Object = js.native

@js.native
@JSGlobal("HTMLScriptElement")
class HTMLScriptElement extends HTMLElement:
  var src: String = js.native
  var `type`: String = js.native
  var text: String = js.native
  var async: Boolean = js.native
  var defer: Boolean = js.native
  var noModule: Boolean = js.native
  var crossOrigin: String = js.native
  var integrity: String = js.native
  var referrerPolicy: String = js.native
  var charset: String = js.native

@js.native
@JSGlobal("HTMLTitleElement")
class HTMLTitleElement extends HTMLElement:
  var text: String = js.native

@js.native
@JSGlobal("HTMLMetaElement")
class HTMLMetaElement extends HTMLElement:
  var name: String = js.native
  var content: String = js.native
  var httpEquiv: String = js.native

@js.native
@JSGlobal("HTMLTableElement")
class HTMLTableElement extends HTMLElement:
  var caption: HTMLElement = js.native
  var tHead: HTMLTableSectionElement = js.native
  var tFoot: HTMLTableSectionElement = js.native
  def tBodies: HTMLCollection[HTMLTableSectionElement] = js.native
  def rows: HTMLCollection[HTMLTableRowElement] = js.native
  def createTHead(): HTMLTableSectionElement = js.native
  def createTBody(): HTMLTableSectionElement = js.native
  def insertRow(index: Int = js.native): HTMLTableRowElement = js.native
  def deleteRow(index: Int): Unit = js.native

@js.native
@JSGlobal("HTMLTableSectionElement")
class HTMLTableSectionElement extends HTMLElement:
  def rows: HTMLCollection[HTMLTableRowElement] = js.native
  def insertRow(index: Int = js.native): HTMLTableRowElement = js.native
  def deleteRow(index: Int): Unit = js.native

@js.native
@JSGlobal("HTMLTableRowElement")
class HTMLTableRowElement extends HTMLElement:
  def rowIndex: Int = js.native
  def sectionRowIndex: Int = js.native
  def cells: HTMLCollection[HTMLTableCellElement] = js.native
  def insertCell(index: Int = js.native): HTMLTableCellElement = js.native
  def deleteCell(index: Int): Unit = js.native

@js.native
@JSGlobal("HTMLTableCellElement")
class HTMLTableCellElement extends HTMLElement:
  def cellIndex: Int = js.native
  var colSpan: Int = js.native
  var rowSpan: Int = js.native
  var headers: String = js.native
  var scope: String = js.native
  var abbr: String = js.native

@js.native
@JSGlobal("HTMLTableCaptionElement")
class HTMLTableCaptionElement extends HTMLElement

@js.native
@JSGlobal("HTMLMediaElement")
class HTMLMediaElement extends HTMLElement:
  var src: String = js.native
  var srcObject: js.Any = js.native
  var currentTime: Double = js.native
  var volume: Double = js.native
  var muted: Boolean = js.native
  var loop: Boolean = js.native
  var autoplay: Boolean = js.native
  var controls: Boolean = js.native
  var playbackRate: Double = js.native
  var preload: String = js.native
  var crossOrigin: String = js.native
  def currentSrc: String = js.native
  def duration: Double = js.native
  def paused: Boolean = js.native
  def ended: Boolean = js.native
  def seeking: Boolean = js.native
  def readyState: Int = js.native
  def networkState: Int = js.native
  def play(): js.Promise[Unit] = js.native
  def pause(): Unit = js.native
  def load(): Unit = js.native
  def canPlayType(`type`: String): String = js.native

@js.native
@JSGlobal("HTMLAudioElement")
class HTMLAudioElement extends HTMLMediaElement

@js.native
@JSGlobal("HTMLVideoElement")
class HTMLVideoElement extends HTMLMediaElement:
  var width: Int = js.native
  var height: Int = js.native
  var poster: String = js.native
  var playsInline: Boolean = js.native
  def videoWidth: Int = js.native
  def videoHeight: Int = js.native

@js.native
@JSGlobal("HTMLProgressElement")
class HTMLProgressElement extends HTMLElement:
  var value: Double = js.native
  var max: Double = js.native
  def position: Double = js.native

@js.native
@JSGlobal("HTMLTemplateElement")
class HTMLTemplateElement extends HTMLElement:
  def content: DocumentFragment = js.native

@js.native
@JSGlobal("HTMLDetailsElement")
class HTMLDetailsElement extends HTMLElement:
  var open: Boolean = js.native

@js.native
@JSGlobal("HTMLUnknownElement")
class HTMLUnknownElement extends HTMLElement
