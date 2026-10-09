package org.scalajs.dom

import scala.scalajs.js
import scala.scalajs.js.annotation.JSGlobal
import scala.scalajs.js.typedarray.{ArrayBuffer, Uint8Array}

trait BlobPropertyBag extends js.Object:
  var `type`: js.UndefOr[String] = js.undefined
  var endings: js.UndefOr[String] = js.undefined

object BlobPropertyBag:
  def apply(`type`: js.UndefOr[String] = js.undefined): BlobPropertyBag =
    val bag = new BlobPropertyBag {}
    if !js.isUndefined(`type`) then bag.`type` = `type`
    bag

trait FilePropertyBag extends BlobPropertyBag:
  var lastModified: js.UndefOr[Double] = js.undefined

@js.native
@JSGlobal("Blob")
class Blob(blobParts: js.Array[BlobPart] | js.Iterable[BlobPart] = js.native, options: BlobPropertyBag = js.native) extends js.Object:
  def size: Double = js.native
  def `type`: String = js.native
  def slice(start: Double = js.native, end: Double = js.native, contentType: String = js.native): Blob = js.native
  def text(): js.Promise[String] = js.native
  def arrayBuffer(): js.Promise[ArrayBuffer] = js.native
  def stream(): ReadableStream[Uint8Array] = js.native

@js.native
@JSGlobal("File")
class File(bits: js.Array[BlobPart] | js.Iterable[BlobPart], fileName: String, options: FilePropertyBag = js.native) extends Blob:
  def name: String = js.native
  def lastModified: Double = js.native
  def webkitRelativePath: String = js.native

@js.native
@JSGlobal("FileList")
class FileList extends DOMList[File]:
  def item(index: Int): File = js.native

@js.native
@JSGlobal("FileReader")
class FileReader() extends EventTarget:
  def result: js.Any = js.native
  def error: js.Object = js.native
  def readyState: Int = js.native
  var onload: js.Function1[ProgressEvent, Any] = js.native
  var onerror: js.Function1[ProgressEvent, Any] = js.native
  var onabort: js.Function1[ProgressEvent, Any] = js.native
  var onloadstart: js.Function1[ProgressEvent, Any] = js.native
  var onloadend: js.Function1[ProgressEvent, Any] = js.native
  var onprogress: js.Function1[ProgressEvent, Any] = js.native
  def abort(): Unit = js.native
  def readAsArrayBuffer(blob: Blob): Unit = js.native
  def readAsDataURL(blob: Blob): Unit = js.native
  def readAsText(blob: Blob, encoding: String = js.native): Unit = js.native

@js.native
@JSGlobal("FileReader")
object FileReader extends js.Object:
  val EMPTY: Int = js.native
  val LOADING: Int = js.native
  val DONE: Int = js.native

@js.native
@JSGlobal("URL")
class URL(url: String, base: String = js.native) extends js.Object:
  var href: String = js.native
  def origin: String = js.native
  var protocol: String = js.native
  var username: String = js.native
  var password: String = js.native
  var host: String = js.native
  var hostname: String = js.native
  var port: String = js.native
  var pathname: String = js.native
  var search: String = js.native
  var hash: String = js.native
  def searchParams: URLSearchParams = js.native
  def toJSON(): String = js.native

@js.native
@JSGlobal("URL")
object URL extends js.Object:
  def createObjectURL(blob: Blob): String = js.native
  def revokeObjectURL(url: String): Unit = js.native
  def canParse(url: String, base: String = js.native): Boolean = js.native

@js.native
@JSGlobal("URLSearchParams")
class URLSearchParams(init: String | js.Dictionary[String] | js.Array[js.Array[String]] = js.native) extends js.Object:
  def size: Int = js.native
  def append(name: String, value: String): Unit = js.native
  def delete(name: String, value: String = js.native): Unit = js.native
  def get(name: String): String = js.native
  def getAll(name: String): js.Array[String] = js.native
  def has(name: String, value: String = js.native): Boolean = js.native
  def set(name: String, value: String): Unit = js.native
  def sort(): Unit = js.native
  def forEach(callback: js.Function2[String, String, Any]): Unit = js.native

@js.native
@JSGlobal("FormData")
class FormData(form: HTMLFormElement = js.native) extends js.Object:
  def append(name: String, value: String | Blob, filename: String = js.native): Unit = js.native
  def delete(name: String): Unit = js.native
  def get(name: String): js.Any = js.native
  def getAll(name: String): js.Array[js.Any] = js.native
  def has(name: String): Boolean = js.native
  def set(name: String, value: String | Blob, filename: String = js.native): Unit = js.native

@js.native
@JSGlobal("TextEncoder")
class TextEncoder() extends js.Object:
  def encoding: String = js.native
  def encode(input: String = js.native): js.typedarray.Uint8Array = js.native

@js.native
@JSGlobal("TextDecoder")
class TextDecoder(label: String = js.native, options: js.Object = js.native) extends js.Object:
  def encoding: String = js.native
  def decode(input: BufferSource = js.native, options: js.Object = js.native): String = js.native
