package org.scalajs.dom

import scala.scalajs.js
import scala.scalajs.js.annotation.JSGlobal
import scala.scalajs.js.typedarray.{ArrayBuffer, ArrayBufferView, Float32Array, Uint8ClampedArray}

@js.native
@JSGlobal("CanvasRenderingContext2D")
class CanvasRenderingContext2D extends js.Object:
  def canvas: HTMLCanvasElement = js.native
  var fillStyle: js.Any = js.native
  var strokeStyle: js.Any = js.native
  var lineWidth: Double = js.native
  var lineCap: String = js.native
  var lineJoin: String = js.native
  var lineDashOffset: Double = js.native
  var miterLimit: Double = js.native
  var globalAlpha: Double = js.native
  var globalCompositeOperation: String = js.native
  var imageSmoothingEnabled: Boolean = js.native
  var imageSmoothingQuality: String = js.native
  var font: String = js.native
  var textAlign: String = js.native
  var textBaseline: String = js.native
  var direction: String = js.native
  var filter: String = js.native
  var shadowBlur: Double = js.native
  var shadowColor: String = js.native
  var shadowOffsetX: Double = js.native
  var shadowOffsetY: Double = js.native
  def save(): Unit = js.native
  def restore(): Unit = js.native
  def reset(): Unit = js.native
  def scale(x: Double, y: Double): Unit = js.native
  def rotate(angle: Double): Unit = js.native
  def translate(x: Double, y: Double): Unit = js.native
  def transform(m11: Double, m12: Double, m21: Double, m22: Double, dx: Double, dy: Double): Unit = js.native
  def setTransform(m11: Double, m12: Double, m21: Double, m22: Double, dx: Double, dy: Double): Unit = js.native
  def resetTransform(): Unit = js.native
  def beginPath(): Unit = js.native
  def closePath(): Unit = js.native
  def moveTo(x: Double, y: Double): Unit = js.native
  def lineTo(x: Double, y: Double): Unit = js.native
  def rect(x: Double, y: Double, w: Double, h: Double): Unit = js.native
  def roundRect(x: Double, y: Double, w: Double, h: Double, radii: Double | Int | js.Array[Double] = js.native): Unit = js.native
  def arc(x: Double, y: Double, radius: Double, startAngle: Double, endAngle: Double, counterclockwise: Boolean = js.native): Unit = js.native
  def arcTo(x1: Double, y1: Double, x2: Double, y2: Double, radius: Double): Unit = js.native
  def ellipse(x: Double, y: Double, radiusX: Double, radiusY: Double, rotation: Double, startAngle: Double, endAngle: Double, counterclockwise: Boolean = js.native): Unit = js.native
  def quadraticCurveTo(cpx: Double, cpy: Double, x: Double, y: Double): Unit = js.native
  def bezierCurveTo(cp1x: Double, cp1y: Double, cp2x: Double, cp2y: Double, x: Double, y: Double): Unit = js.native
  def fill(fillRule: String = js.native): Unit = js.native
  def stroke(): Unit = js.native
  def clip(fillRule: String = js.native): Unit = js.native
  def isPointInPath(x: Double, y: Double, fillRule: String = js.native): Boolean = js.native
  def isPointInStroke(x: Double, y: Double): Boolean = js.native
  def fillRect(x: Double, y: Double, w: Double, h: Double): Unit = js.native
  def strokeRect(x: Double, y: Double, w: Double, h: Double): Unit = js.native
  def clearRect(x: Double, y: Double, w: Double, h: Double): Unit = js.native
  def fillText(text: String, x: Double, y: Double, maxWidth: Double = js.native): Unit = js.native
  def strokeText(text: String, x: Double, y: Double, maxWidth: Double = js.native): Unit = js.native
  def measureText(text: String): TextMetrics = js.native
  def drawImage(image: Element, offsetX: Double, offsetY: Double, width: Double = js.native, height: Double = js.native, canvasOffsetX: Double = js.native, canvasOffsetY: Double = js.native, canvasImageWidth: Double = js.native, canvasImageHeight: Double = js.native): Unit = js.native
  def createImageData(widthOrData: Double | Int | ImageData, height: Double = js.native): ImageData = js.native
  def getImageData(sx: Double, sy: Double, sw: Double, sh: Double): ImageData = js.native
  def putImageData(imagedata: ImageData, dx: Double, dy: Double, dirtyX: Double = js.native, dirtyY: Double = js.native, dirtyWidth: Double = js.native, dirtyHeight: Double = js.native): Unit = js.native
  def createLinearGradient(x0: Double, y0: Double, x1: Double, y1: Double): CanvasGradient = js.native
  def createRadialGradient(x0: Double, y0: Double, r0: Double, x1: Double, y1: Double, r1: Double): CanvasGradient = js.native
  def createConicGradient(startAngle: Double, x: Double, y: Double): CanvasGradient = js.native
  def createPattern(image: Element, repetition: String): js.Object = js.native
  def setLineDash(segments: js.Array[Double]): Unit = js.native
  def getLineDash(): js.Array[Double] = js.native

@js.native
@JSGlobal("CanvasGradient")
class CanvasGradient extends js.Object:
  def addColorStop(offset: Double, color: String): Unit = js.native

@js.native
@JSGlobal("TextMetrics")
class TextMetrics extends js.Object:
  def width: Double = js.native
  def actualBoundingBoxLeft: Double = js.native
  def actualBoundingBoxRight: Double = js.native
  def actualBoundingBoxAscent: Double = js.native
  def actualBoundingBoxDescent: Double = js.native
  def fontBoundingBoxAscent: Double = js.native
  def fontBoundingBoxDescent: Double = js.native

/** `new ImageData(width, height)` or `new ImageData(data, width[, height])`. */
@js.native
@JSGlobal("ImageData")
class ImageData(dataOrWidth: Uint8ClampedArray | Int, widthOrHeight: Int, heightArg: Int = js.native) extends js.Object:
  def width: Int = js.native
  def height: Int = js.native
  def data: Uint8ClampedArray = js.native
  def colorSpace: String = js.native

@js.native
@JSGlobal("AudioContext")
class AudioContext(options: js.Object = js.native) extends EventTarget:
  def currentTime: Double = js.native
  def destination: AudioDestinationNode = js.native
  def sampleRate: Double = js.native
  def state: String = js.native
  def baseLatency: Double = js.native
  def close(): js.Promise[Unit] = js.native
  def resume(): js.Promise[Unit] = js.native
  def suspend(): js.Promise[Unit] = js.native
  def createBuffer(numOfChannels: Int, length: Int, sampleRate: Double): AudioBuffer = js.native
  def createBufferSource(): AudioBufferSourceNode = js.native
  def createGain(): GainNode = js.native
  def createOscillator(): OscillatorNode = js.native
  def createMediaElementSource(mediaElement: HTMLMediaElement): AudioNode = js.native
  def decodeAudioData(audioData: ArrayBuffer, successCallback: js.Function1[AudioBuffer, Any] = js.native, errorCallback: js.Function1[js.Any, Any] = js.native): js.Promise[AudioBuffer] = js.native
  var onstatechange: js.Function1[Event, Any] = js.native

@js.native
trait AudioNode extends EventTarget:
  def context: AudioContext = js.native
  def numberOfInputs: Int = js.native
  def numberOfOutputs: Int = js.native
  var channelCount: Int = js.native
  var channelCountMode: String = js.native
  var channelInterpretation: String = js.native
  def connect(destination: AudioNode | AudioParam, output: Int = js.native, input: Int = js.native): Unit = js.native
  def disconnect(destination: AudioNode | AudioParam = js.native): Unit = js.native

@js.native
trait AudioDestinationNode extends AudioNode:
  def maxChannelCount: Int = js.native

@js.native
trait AudioParam extends js.Object:
  var value: Double = js.native
  def defaultValue: Double = js.native
  def minValue: Double = js.native
  def maxValue: Double = js.native
  def setValueAtTime(value: Double, startTime: Double): AudioParam = js.native
  def linearRampToValueAtTime(value: Double, endTime: Double): AudioParam = js.native
  def exponentialRampToValueAtTime(value: Double, endTime: Double): AudioParam = js.native
  def cancelScheduledValues(startTime: Double): AudioParam = js.native

@js.native
trait AudioBuffer extends js.Object:
  def sampleRate: Double = js.native
  def length: Int = js.native
  def duration: Double = js.native
  def numberOfChannels: Int = js.native
  def getChannelData(channel: Int): Float32Array = js.native
  def copyFromChannel(destination: Float32Array, channelNumber: Int, startInChannel: Int = js.native): Unit = js.native
  def copyToChannel(source: Float32Array, channelNumber: Int, startInChannel: Int = js.native): Unit = js.native

@js.native
trait AudioScheduledSourceNode extends AudioNode:
  def start(when: Double = js.native): Unit = js.native
  def stop(when: Double = js.native): Unit = js.native
  var onended: js.Function1[Event, Any] = js.native

@js.native
trait AudioBufferSourceNode extends AudioNode:
  var buffer: AudioBuffer = js.native
  var loop: Boolean = js.native
  var loopStart: Double = js.native
  var loopEnd: Double = js.native
  def playbackRate: AudioParam = js.native
  def detune: AudioParam = js.native
  def start(when: Double = js.native, offset: Double = js.native, duration: Double = js.native): Unit = js.native
  def stop(when: Double = js.native): Unit = js.native
  var onended: js.Function1[Event, Any] = js.native

@js.native
trait GainNode extends AudioNode:
  def gain: AudioParam = js.native

@js.native
trait OscillatorNode extends AudioScheduledSourceNode:
  var `type`: String = js.native
  def frequency: AudioParam = js.native
  def detune: AudioParam = js.native

@js.native
@JSGlobal("Notification")
class Notification(title: String, options: NotificationOptions = js.native) extends EventTarget:
  def body: String = js.native
  def data: js.Any = js.native
  def dir: String = js.native
  def icon: String = js.native
  def badge: String = js.native
  def image: String = js.native
  def lang: String = js.native
  def tag: String = js.native
  def silent: Boolean = js.native
  def requireInteraction: Boolean = js.native
  def timestamp: Double = js.native
  def vibrate: js.Array[Double] = js.native
  def close(): Unit = js.native
  var onclick: js.Function1[Event, Any] = js.native
  var onclose: js.Function1[Event, Any] = js.native
  var onerror: js.Function1[Event, Any] = js.native
  var onshow: js.Function1[Event, Any] = js.native

@js.native
@JSGlobal("Notification")
object Notification extends js.Object:
  def permission: String = js.native
  def maxActions: Int = js.native
  def requestPermission(callback: js.Function1[String, Any] = js.native): js.Promise[String] = js.native

trait NotificationOptions extends js.Object:
  var body: js.UndefOr[String] = js.undefined
  var dir: js.UndefOr[String] = js.undefined
  var icon: js.UndefOr[String] = js.undefined
  var badge: js.UndefOr[String] = js.undefined
  var image: js.UndefOr[String] = js.undefined
  var lang: js.UndefOr[String] = js.undefined
  var tag: js.UndefOr[String] = js.undefined
  var data: js.UndefOr[js.Any] = js.undefined
  var silent: js.UndefOr[Boolean] = js.undefined
  var renotify: js.UndefOr[Boolean] = js.undefined
  var requireInteraction: js.UndefOr[Boolean] = js.undefined
  var timestamp: js.UndefOr[Double] = js.undefined
  var vibrate: js.UndefOr[js.Array[Double]] = js.undefined

@js.native
@JSGlobal("WebSocket")
class WebSocket(urlArg: String, protocols: String | js.Array[String] = js.native) extends EventTarget:
  def url: String = js.native
  def protocol: String = js.native
  def extensions: String = js.native
  def readyState: Int = js.native
  def bufferedAmount: Double = js.native
  var binaryType: String = js.native
  var onopen: js.Function1[Event, Any] = js.native
  var onmessage: js.Function1[MessageEvent, Any] = js.native
  var onclose: js.Function1[CloseEvent, Any] = js.native
  var onerror: js.Function1[Event, Any] = js.native
  def send(data: String | Blob | ArrayBuffer | ArrayBufferView): Unit = js.native
  def close(code: Int = js.native, reason: String = js.native): Unit = js.native

@js.native
@JSGlobal("WebSocket")
object WebSocket extends js.Object:
  val CONNECTING: Int = js.native
  val OPEN: Int = js.native
  val CLOSING: Int = js.native
  val CLOSED: Int = js.native

@js.native
@JSGlobal("EventSource")
class EventSource(url: String, init: js.Object = js.native) extends EventTarget:
  def readyState: Int = js.native
  def withCredentials: Boolean = js.native
  var onopen: js.Function1[Event, Any] = js.native
  var onmessage: js.Function1[MessageEvent, Any] = js.native
  var onerror: js.Function1[Event, Any] = js.native
  def close(): Unit = js.native

@js.native
@JSGlobal("Crypto")
class Crypto extends js.Object:
  def randomUUID(): String = js.native
  def getRandomValues[T <: ArrayBufferView](array: T): T = js.native
  def subtle: js.Object = js.native

@js.native
@JSGlobal("crypto")
object crypto extends Crypto
