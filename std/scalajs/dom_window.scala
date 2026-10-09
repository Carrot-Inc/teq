package org.scalajs.dom

import scala.scalajs.js
import scala.scalajs.js.annotation.JSGlobal

@js.native
@JSGlobal("Window")
class Window extends EventTarget:
  def window: Window = js.native
  def self: Window = js.native
  def document: HTMLDocument = js.native
  def location: Location = js.native
  def history: History = js.native
  def navigator: Navigator = js.native
  def screen: Screen = js.native
  def console: Console = js.native
  def localStorage: Storage = js.native
  def sessionStorage: Storage = js.native
  def performance: Performance = js.native
  def visualViewport: VisualViewport = js.native
  def parent: Window = js.native
  def top: Window = js.native
  def opener: Window = js.native
  def frameElement: Element = js.native
  def closed: Boolean = js.native
  var name: String = js.native
  def origin: String = js.native
  def isSecureContext: Boolean = js.native
  def innerWidth: Double = js.native
  def innerHeight: Double = js.native
  def outerWidth: Int = js.native
  def outerHeight: Int = js.native
  def screenX: Int = js.native
  def screenY: Int = js.native
  def scrollX: Double = js.native
  def scrollY: Double = js.native
  def pageXOffset: Double = js.native
  def pageYOffset: Double = js.native
  def devicePixelRatio: Double = js.native
  def alert(message: String = js.native): Unit = js.native
  def confirm(message: String = js.native): Boolean = js.native
  def prompt(message: String = js.native, default: String = js.native): String = js.native
  def print(): Unit = js.native
  def focus(): Unit = js.native
  def blur(): Unit = js.native
  def close(): Unit = js.native
  def stop(): Unit = js.native
  def open(url: String = js.native, target: String = js.native, features: String = js.native): Window = js.native
  def scroll(x: Double | Int | ScrollToOptions, y: Double = js.native): Unit = js.native
  def scrollTo(x: Double | Int | ScrollToOptions, y: Double = js.native): Unit = js.native
  def scrollBy(x: Double | Int | ScrollToOptions, y: Double = js.native): Unit = js.native
  def getSelection(): Selection = js.native
  def getComputedStyle(elt: Element, pseudoElt: String = js.native): CSSStyleDeclaration = js.native
  def matchMedia(mediaQuery: String): MediaQueryList = js.native
  def requestAnimationFrame(callback: js.Function1[Double, Any]): Int = js.native
  def cancelAnimationFrame(handle: Int): Unit = js.native
  def requestIdleCallback(callback: js.Function1[js.Object, Any], options: js.Object = js.native): Int = js.native
  def cancelIdleCallback(handle: Int): Unit = js.native
  def setTimeout(handler: js.Function0[Any], timeout: Double = js.native): Int = js.native
  def clearTimeout(handle: Int): Unit = js.native
  def setInterval(handler: js.Function0[Any], timeout: Double): Int = js.native
  def clearInterval(handle: Int): Unit = js.native
  def queueMicrotask(callback: js.Function0[Any]): Unit = js.native
  def postMessage(message: js.Any, targetOrigin: String = js.native, transfer: js.Array[js.Object] = js.native): Unit = js.native
  def fetch(info: RequestInfo, init: RequestInit = js.native): js.Promise[Response] = js.native
  def atob(encoded: String): String = js.native
  def btoa(raw: String): String = js.native
  def structuredClone[T](value: T): T = js.native
  var onload: js.Function1[Event, Any] = js.native
  var onunload: js.Function1[Event, Any] = js.native
  var onbeforeunload: js.Function1[BeforeUnloadEvent, Any] = js.native
  var onerror: js.Function5[Event | String, String, Int, Int, Any, Any] = js.native
  var onresize: js.Function1[UIEvent, Any] = js.native
  var onscroll: js.Function1[UIEvent, Any] = js.native
  var onfocus: js.Function1[FocusEvent, Any] = js.native
  var onblur: js.Function1[FocusEvent, Any] = js.native
  var onhashchange: js.Function1[HashChangeEvent, Any] = js.native
  var onpopstate: js.Function1[PopStateEvent, Any] = js.native
  var onmessage: js.Function1[MessageEvent, Any] = js.native
  var onstorage: js.Function1[StorageEvent, Any] = js.native
  var ononline: js.Function1[Event, Any] = js.native
  var onoffline: js.Function1[Event, Any] = js.native
  var onkeydown: js.Function1[KeyboardEvent, Any] = js.native
  var onkeyup: js.Function1[KeyboardEvent, Any] = js.native
  var onclick: js.Function1[MouseEvent, Any] = js.native
  var onmousedown: js.Function1[MouseEvent, Any] = js.native
  var onmousemove: js.Function1[MouseEvent, Any] = js.native
  var onmouseup: js.Function1[MouseEvent, Any] = js.native
  var onpointerdown: js.Function1[PointerEvent, Any] = js.native
  var onpointermove: js.Function1[PointerEvent, Any] = js.native
  var onpointerup: js.Function1[PointerEvent, Any] = js.native

@js.native
trait Location extends js.Object:
  var href: String = js.native
  var protocol: String = js.native
  var host: String = js.native
  var hostname: String = js.native
  var port: String = js.native
  var pathname: String = js.native
  var search: String = js.native
  var hash: String = js.native
  def origin: String = js.native
  def reload(): Unit = js.native
  def assign(url: String): Unit = js.native
  def replace(url: String): Unit = js.native

@js.native
@JSGlobal("History")
class History extends js.Object:
  def length: Int = js.native
  def state: js.Any = js.native
  var scrollRestoration: String = js.native
  def back(): Unit = js.native
  def forward(): Unit = js.native
  def go(delta: Int = js.native): Unit = js.native
  def pushState(statedata: js.Any, title: String, url: String = js.native): Unit = js.native
  def replaceState(statedata: js.Any, title: String, url: String = js.native): Unit = js.native

@js.native
@JSGlobal("Navigator")
class Navigator extends js.Object:
  def userAgent: String = js.native
  def appName: String = js.native
  def appVersion: String = js.native
  def platform: String = js.native
  def vendor: String = js.native
  def language: String = js.native
  def languages: js.Array[String] = js.native
  def onLine: Boolean = js.native
  def cookieEnabled: Boolean = js.native
  def hardwareConcurrency: Int = js.native
  def maxTouchPoints: Int = js.native
  def pdfViewerEnabled: Boolean = js.native
  def clipboard: Clipboard = js.native
  def locks: LockManager = js.native
  def geolocation: Geolocation = js.native
  def mediaDevices: js.Object = js.native
  def permissions: js.Object = js.native
  def serviceWorker: js.Object = js.native
  def storage: js.Object = js.native
  def userActivation: js.Object = js.native
  def vibrate(pattern: Double | Int | js.Array[Double]): Boolean = js.native
  def sendBeacon(url: String, data: BodyInit = js.native): Boolean = js.native
  def canShare(data: js.Object = js.native): Boolean = js.native
  def share(data: js.Object = js.native): js.Promise[Unit] = js.native

@js.native
@JSGlobal("Screen")
class Screen extends js.Object:
  def width: Double = js.native
  def height: Double = js.native
  def availWidth: Double = js.native
  def availHeight: Double = js.native
  def colorDepth: Int = js.native
  def pixelDepth: Int = js.native
  def orientation: js.Object = js.native

@js.native
@JSGlobal("VisualViewport")
class VisualViewport extends EventTarget:
  def offsetLeft: Double = js.native
  def offsetTop: Double = js.native
  def pageLeft: Double = js.native
  def pageTop: Double = js.native
  def width: Double = js.native
  def height: Double = js.native
  def scale: Double = js.native

@js.native
@JSGlobal("Performance")
class Performance extends EventTarget:
  def now(): Double = js.native
  def timeOrigin: Double = js.native
  def mark(name: String, options: js.Object = js.native): js.Object = js.native
  def measure(name: String, startMark: String = js.native, endMark: String = js.native): js.Object = js.native
  def getEntriesByName(name: String, `type`: String = js.native): js.Array[js.Object] = js.native
  def clearMarks(name: String = js.native): Unit = js.native
  def clearMeasures(name: String = js.native): Unit = js.native

@js.native
@JSGlobal("Storage")
class Storage extends js.Object:
  def length: Int = js.native
  def key(index: Int): String = js.native
  def getItem(key: String): String = js.native
  def setItem(key: String, data: String): Unit = js.native
  def removeItem(key: String): Unit = js.native
  def clear(): Unit = js.native

@js.native
trait Console extends js.Object:
  def log(message: Any, optionalParams: Any*): Unit = js.native
  def info(message: Any, optionalParams: Any*): Unit = js.native
  def warn(message: Any, optionalParams: Any*): Unit = js.native
  def error(message: Any, optionalParams: Any*): Unit = js.native
  def debug(message: Any, optionalParams: Any*): Unit = js.native
  def trace(message: Any = js.native, optionalParams: Any*): Unit = js.native
  def dir(value: Any, optionalParams: Any*): Unit = js.native
  def table(data: Any, columns: js.Array[String] = js.native): Unit = js.native
  def assert(test: Boolean, message: Any = js.native, optionalParams: Any*): Unit = js.native
  def clear(): Unit = js.native
  def count(label: String = js.native): Unit = js.native
  def countReset(label: String = js.native): Unit = js.native
  def group(label: Any = js.native): Unit = js.native
  def groupCollapsed(label: Any = js.native): Unit = js.native
  def groupEnd(): Unit = js.native
  def time(label: String = js.native): Unit = js.native
  def timeEnd(label: String = js.native): Unit = js.native
  def timeLog(label: String = js.native, optionalParams: Any*): Unit = js.native

@js.native
trait Clipboard extends EventTarget:
  def readText(): js.Promise[String] = js.native
  def writeText(newClipText: String): js.Promise[Unit] = js.native
  def read(): js.Promise[js.Array[js.Object]] = js.native
  def write(data: js.Array[js.Object]): js.Promise[Unit] = js.native

@js.native
trait MediaQueryList extends EventTarget:
  def matches: Boolean = js.native
  def media: String = js.native
  def addListener(listener: js.Function1[MediaQueryList, Any]): Unit = js.native
  def removeListener(listener: js.Function1[MediaQueryList, Any]): Unit = js.native
  var onchange: js.Function1[MediaQueryList, Any] = js.native

@js.native
@JSGlobal("LockManager")
class LockManager extends js.Object:
  def request(name: String, options: LockOptions, callback: js.Function1[Lock, js.Promise[Unit]]): js.Promise[Unit] = js.native
  def query(): js.Promise[js.Object] = js.native

@js.native
@JSGlobal("Lock")
class Lock extends js.Object:
  def name: String = js.native
  def mode: String = js.native

trait LockOptions extends js.Object:
  var mode: js.UndefOr[String] = js.undefined
  var ifAvailable: js.UndefOr[Boolean] = js.undefined
  var steal: js.UndefOr[Boolean] = js.undefined
  var signal: js.UndefOr[AbortSignal] = js.undefined

@js.native
trait Geolocation extends js.Object:
  def getCurrentPosition(successCallback: js.Function1[Position, Any], errorCallback: js.Function1[PositionError, Any] = js.native, options: PositionOptions = js.native): Unit = js.native
  def watchPosition(successCallback: js.Function1[Position, Any], errorCallback: js.Function1[PositionError, Any] = js.native, options: PositionOptions = js.native): Int = js.native
  def clearWatch(watchId: Int): Unit = js.native

@js.native
trait Position extends js.Object:
  def coords: Coordinates = js.native
  def timestamp: Double = js.native

@js.native
trait Coordinates extends js.Object:
  def latitude: Double = js.native
  def longitude: Double = js.native
  def accuracy: Double = js.native
  def altitude: Double = js.native
  def altitudeAccuracy: Double = js.native
  def heading: Double = js.native
  def speed: Double = js.native

@js.native
trait PositionError extends js.Object:
  def code: Int = js.native
  def message: String = js.native

@js.native
@JSGlobal("GeolocationPositionError")
object PositionError extends js.Object:
  val PERMISSION_DENIED: Int = js.native
  val POSITION_UNAVAILABLE: Int = js.native
  val TIMEOUT: Int = js.native

trait PositionOptions extends js.Object:
  var enableHighAccuracy: js.UndefOr[Boolean] = js.undefined
  var timeout: js.UndefOr[Double] = js.undefined
  var maximumAge: js.UndefOr[Double] = js.undefined
