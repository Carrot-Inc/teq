// scalajs-dom 2.x's `experimental` package object and `raw` object: the deprecated aliases of the
// names that moved to `org.scalajs.dom`, for library bodies compiled against them (sttp's fetch
// backend). Only what the layer defines is aliased.
package org.scalajs.dom.experimental:

  import scala.scalajs.js
  import scala.scalajs.js.annotation.{JSGlobal, JSGlobalScope}

  type RequestInfo = org.scalajs.dom.RequestInfo
  type HeadersInit = org.scalajs.dom.HeadersInit
  type ByteString = org.scalajs.dom.ByteString
  type BodyInit = org.scalajs.dom.BodyInit
  type AbortController = org.scalajs.dom.AbortController
  type AbortSignal = org.scalajs.dom.AbortSignal
  type Request = org.scalajs.dom.Request
  type RequestInit = org.scalajs.dom.RequestInit
  type Response = org.scalajs.dom.Response
  type ResponseInit = org.scalajs.dom.ResponseInit
  type Body = org.scalajs.dom.Body
  type Headers = org.scalajs.dom.Headers
  type HttpMethod = org.scalajs.dom.HttpMethod
  type RequestMode = org.scalajs.dom.RequestMode
  type RequestCredentials = org.scalajs.dom.RequestCredentials
  type RequestCache = org.scalajs.dom.RequestCache
  type RequestRedirect = org.scalajs.dom.RequestRedirect
  type ResponseType = org.scalajs.dom.ResponseType
  type Notification = org.scalajs.dom.Notification
  type NotificationOptions = org.scalajs.dom.NotificationOptions
  type ReadableStream[+T] = org.scalajs.dom.ReadableStream[T]
  type ReadableStreamReader[+T] = org.scalajs.dom.ReadableStreamReader[T]
  type Chunk[+T] = org.scalajs.dom.Chunk[T]
  type URL = org.scalajs.dom.URL
  type URLSearchParams = org.scalajs.dom.URLSearchParams

  val HttpMethod: org.scalajs.dom.HttpMethod.type = org.scalajs.dom.HttpMethod
  val RequestMode: org.scalajs.dom.RequestMode.type = org.scalajs.dom.RequestMode
  val RequestCredentials: org.scalajs.dom.RequestCredentials.type = org.scalajs.dom.RequestCredentials
  val RequestCache: org.scalajs.dom.RequestCache.type = org.scalajs.dom.RequestCache
  val RequestRedirect: org.scalajs.dom.RequestRedirect.type = org.scalajs.dom.RequestRedirect
  val ResponseType: org.scalajs.dom.ResponseType.type = org.scalajs.dom.ResponseType

  @js.native
  @JSGlobalScope
  object Fetch extends js.Object:
    def fetch(info: org.scalajs.dom.RequestInfo, init: org.scalajs.dom.RequestInit = js.native): js.Promise[org.scalajs.dom.Response] = js.native

  @js.native
  @JSGlobal("Response")
  object Response extends js.Object:
    def error(): org.scalajs.dom.Response = js.native
    def redirect(url: String, status: Int = js.native): org.scalajs.dom.Response = js.native

  object ResponseInit:
    def apply(_status: Int = 200, _statusText: org.scalajs.dom.ByteString = "OK", _headers: org.scalajs.dom.HeadersInit = js.Dictionary[String]()): org.scalajs.dom.ResponseInit =
      new org.scalajs.dom.ResponseInit:
        status = _status
        statusText = _statusText
        headers = _headers

package org.scalajs.dom:

  object raw:
    type AnimationEvent = org.scalajs.dom.AnimationEvent
    type Attr = org.scalajs.dom.Attr
    type AudioBuffer = org.scalajs.dom.AudioBuffer
    type AudioBufferSourceNode = org.scalajs.dom.AudioBufferSourceNode
    type AudioContext = org.scalajs.dom.AudioContext
    type AudioDestinationNode = org.scalajs.dom.AudioDestinationNode
    type AudioNode = org.scalajs.dom.AudioNode
    type AudioParam = org.scalajs.dom.AudioParam
    type BeforeUnloadEvent = org.scalajs.dom.BeforeUnloadEvent
    type Blob = org.scalajs.dom.Blob
    type BlobPropertyBag = org.scalajs.dom.BlobPropertyBag
    type CanvasGradient = org.scalajs.dom.CanvasGradient
    type CharacterData = org.scalajs.dom.CharacterData
    type ClientRect = org.scalajs.dom.DOMRect
    type ClipboardEvent = org.scalajs.dom.ClipboardEvent
    type CloseEvent = org.scalajs.dom.CloseEvent
    type Comment = org.scalajs.dom.Comment
    type CompositionEvent = org.scalajs.dom.CompositionEvent
    type Console = org.scalajs.dom.Console
    type Coordinates = org.scalajs.dom.Coordinates
    type CSSStyleDeclaration = org.scalajs.dom.CSSStyleDeclaration
    type CustomEvent = org.scalajs.dom.CustomEvent
    type DataTransfer = org.scalajs.dom.DataTransfer
    type Document = org.scalajs.dom.Document
    type DocumentFragment = org.scalajs.dom.DocumentFragment
    type DOMRect = org.scalajs.dom.DOMRect
    type DOMTokenList = org.scalajs.dom.DOMTokenList
    type DragEvent = org.scalajs.dom.DragEvent
    type Element = org.scalajs.dom.Element
    type ErrorEvent = org.scalajs.dom.ErrorEvent
    type Event = org.scalajs.dom.Event
    type EventInit = org.scalajs.dom.EventInit
    type EventSource = org.scalajs.dom.EventSource
    type EventTarget = org.scalajs.dom.EventTarget
    type File = org.scalajs.dom.File
    type FileList = org.scalajs.dom.FileList
    type FileReader = org.scalajs.dom.FileReader
    type FocusEvent = org.scalajs.dom.FocusEvent
    type FormData = org.scalajs.dom.FormData
    type GainNode = org.scalajs.dom.GainNode
    type Geolocation = org.scalajs.dom.Geolocation
    type HashChangeEvent = org.scalajs.dom.HashChangeEvent
    type History = org.scalajs.dom.History
    type HTMLAnchorElement = org.scalajs.dom.HTMLAnchorElement
    type HTMLAudioElement = org.scalajs.dom.HTMLAudioElement
    type HTMLBodyElement = org.scalajs.dom.HTMLBodyElement
    type HTMLBRElement = org.scalajs.dom.HTMLBRElement
    type HTMLButtonElement = org.scalajs.dom.HTMLButtonElement
    type HTMLCanvasElement = org.scalajs.dom.HTMLCanvasElement
    type HTMLCollectionElement = org.scalajs.dom.HTMLCollection
    type HTMLDivElement = org.scalajs.dom.HTMLDivElement
    type HTMLDListElement = org.scalajs.dom.HTMLDListElement
    type HTMLDocument = org.scalajs.dom.HTMLDocument
    type HTMLElement = org.scalajs.dom.HTMLElement
    type HTMLFieldSetElement = org.scalajs.dom.HTMLFieldSetElement
    type HTMLFormElement = org.scalajs.dom.HTMLFormElement
    type HTMLHeadElement = org.scalajs.dom.HTMLHeadElement
    type HTMLHeadingElement = org.scalajs.dom.HTMLHeadingElement
    type HTMLHRElement = org.scalajs.dom.HTMLHRElement
    type HTMLHtmlElement = org.scalajs.dom.HTMLHtmlElement
    type HTMLIFrameElement = org.scalajs.dom.HTMLIFrameElement
    type HTMLImageElement = org.scalajs.dom.HTMLImageElement
    type HTMLInputElement = org.scalajs.dom.HTMLInputElement
    type HTMLLabelElement = org.scalajs.dom.HTMLLabelElement
    type HTMLLegendElement = org.scalajs.dom.HTMLLegendElement
    type HTMLLIElement = org.scalajs.dom.HTMLLIElement
    type HTMLLinkElement = org.scalajs.dom.HTMLLinkElement
    type HTMLMediaElement = org.scalajs.dom.HTMLMediaElement
    type HTMLMetaElement = org.scalajs.dom.HTMLMetaElement
    type HTMLOListElement = org.scalajs.dom.HTMLOListElement
    type HTMLOptGroupElement = org.scalajs.dom.HTMLOptGroupElement
    type HTMLOptionElement = org.scalajs.dom.HTMLOptionElement
    type HTMLParagraphElement = org.scalajs.dom.HTMLParagraphElement
    type HTMLPreElement = org.scalajs.dom.HTMLPreElement
    type HTMLProgressElement = org.scalajs.dom.HTMLProgressElement
    type HTMLScriptElement = org.scalajs.dom.HTMLScriptElement
    type HTMLSelectElement = org.scalajs.dom.HTMLSelectElement
    type HTMLSpanElement = org.scalajs.dom.HTMLSpanElement
    type HTMLStyleElement = org.scalajs.dom.HTMLStyleElement
    type HTMLTableCaptionElement = org.scalajs.dom.HTMLTableCaptionElement
    type HTMLTableCellElement = org.scalajs.dom.HTMLTableCellElement
    type HTMLTableElement = org.scalajs.dom.HTMLTableElement
    type HTMLTableRowElement = org.scalajs.dom.HTMLTableRowElement
    type HTMLTableSectionElement = org.scalajs.dom.HTMLTableSectionElement
    type HTMLTextAreaElement = org.scalajs.dom.HTMLTextAreaElement
    type HTMLTitleElement = org.scalajs.dom.HTMLTitleElement
    type HTMLUListElement = org.scalajs.dom.HTMLUListElement
    type HTMLUnknownElement = org.scalajs.dom.HTMLUnknownElement
    type HTMLVideoElement = org.scalajs.dom.HTMLVideoElement
    type ImageData = org.scalajs.dom.ImageData
    type KeyboardEvent = org.scalajs.dom.KeyboardEvent
    type Location = org.scalajs.dom.Location
    type MediaQueryList = org.scalajs.dom.MediaQueryList
    type MessageEvent = org.scalajs.dom.MessageEvent
    type MouseEvent = org.scalajs.dom.MouseEvent
    type MutationObserver = org.scalajs.dom.MutationObserver
    type MutationObserverInit = org.scalajs.dom.MutationObserverInit
    type MutationRecord = org.scalajs.dom.MutationRecord
    type NamedNodeMap = org.scalajs.dom.NamedNodeMap
    type Navigator = org.scalajs.dom.Navigator
    type Node = org.scalajs.dom.Node
    type OscillatorNode = org.scalajs.dom.OscillatorNode
    type Performance = org.scalajs.dom.Performance
    type PointerEvent = org.scalajs.dom.PointerEvent
    type PopStateEvent = org.scalajs.dom.PopStateEvent
    type Position = org.scalajs.dom.Position
    type PositionError = org.scalajs.dom.PositionError
    type PositionOptions = org.scalajs.dom.PositionOptions
    type ProgressEvent = org.scalajs.dom.ProgressEvent
    type Range = org.scalajs.dom.Range
    type Screen = org.scalajs.dom.Screen
    type Selection = org.scalajs.dom.Selection
    type Storage = org.scalajs.dom.Storage
    type StorageEvent = org.scalajs.dom.StorageEvent
    type Text = org.scalajs.dom.Text
    type TextMetrics = org.scalajs.dom.TextMetrics
    type Touch = org.scalajs.dom.Touch
    type TouchEvent = org.scalajs.dom.TouchEvent
    type TouchList = org.scalajs.dom.TouchList
    type TransitionEvent = org.scalajs.dom.TransitionEvent
    type UIEvent = org.scalajs.dom.UIEvent
    type ValidityState = org.scalajs.dom.ValidityState
    type WebSocket = org.scalajs.dom.WebSocket
    type WheelEvent = org.scalajs.dom.WheelEvent
    type Window = org.scalajs.dom.Window
