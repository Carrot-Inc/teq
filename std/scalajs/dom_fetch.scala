package org.scalajs.dom

import scala.scalajs.js
import scala.scalajs.js.annotation.{JSGlobal, JSGlobalScope}
import scala.scalajs.js.typedarray.{ArrayBuffer, Uint8Array}

/** A string at run time, as in the original, which casts the literals; the others are opaque
  * aliases of `String`, as in scalajs-dom 2.x for Scala 3. */
@js.native
trait HttpMethod

object HttpMethod:
  val GET: HttpMethod = "GET".asInstanceOf[HttpMethod]
  val POST: HttpMethod = "POST".asInstanceOf[HttpMethod]
  val PUT: HttpMethod = "PUT".asInstanceOf[HttpMethod]
  val PATCH: HttpMethod = "PATCH".asInstanceOf[HttpMethod]
  val DELETE: HttpMethod = "DELETE".asInstanceOf[HttpMethod]
  val HEAD: HttpMethod = "HEAD".asInstanceOf[HttpMethod]
  val OPTIONS: HttpMethod = "OPTIONS".asInstanceOf[HttpMethod]

opaque type RequestMode <: String = String

object RequestMode:
  val cors: RequestMode = "cors"
  val `no-cors`: RequestMode = "no-cors"
  val `same-origin`: RequestMode = "same-origin"
  val navigate: RequestMode = "navigate"

opaque type RequestCredentials <: String = String

object RequestCredentials:
  val omit: RequestCredentials = "omit"
  val `same-origin`: RequestCredentials = "same-origin"
  val include: RequestCredentials = "include"

opaque type RequestCache <: String = String

object RequestCache:
  val default: RequestCache = "default"
  val `no-store`: RequestCache = "no-store"
  val reload: RequestCache = "reload"
  val `no-cache`: RequestCache = "no-cache"
  val `force-cache`: RequestCache = "force-cache"
  val `only-if-cached`: RequestCache = "only-if-cached"

opaque type ResponseType <: String = String

object ResponseType:
  val basic: ResponseType = "basic"
  val cors: ResponseType = "cors"
  val default: ResponseType = "default"
  val error: ResponseType = "error"
  val opaque: ResponseType = "opaque"
  val opaqueredirect: ResponseType = "opaqueredirect"

opaque type RequestRedirect <: String = String

object RequestRedirect:
  val follow: RequestRedirect = "follow"
  val error: RequestRedirect = "error"
  val manual: RequestRedirect = "manual"

trait RequestInit extends js.Object:
  var method: js.UndefOr[HttpMethod] = js.undefined
  var headers: js.UndefOr[HeadersInit] = js.undefined
  var body: js.UndefOr[BodyInit] = js.undefined
  var referrer: js.UndefOr[String] = js.undefined
  var referrerPolicy: js.UndefOr[String] = js.undefined
  var mode: js.UndefOr[RequestMode] = js.undefined
  var credentials: js.UndefOr[RequestCredentials] = js.undefined
  var cache: js.UndefOr[RequestCache] = js.undefined
  var redirect: js.UndefOr[RequestRedirect] = js.undefined
  var integrity: js.UndefOr[String] = js.undefined
  var keepalive: js.UndefOr[Boolean] = js.undefined
  var signal: js.UndefOr[AbortSignal] = js.undefined
  var priority: js.UndefOr[String] = js.undefined
  var window: js.UndefOr[Null] = js.undefined

@js.native
@JSGlobal("Headers")
class Headers(init: HeadersInit = js.native) extends js.Iterable[js.Array[ByteString]]:
  def append(name: String, value: String): Unit = js.native
  def set(name: String, value: String): Unit = js.native
  def delete(name: String): Unit = js.native
  def get(name: String): String = js.native
  def has(name: String): Boolean = js.native
  def forEach(callback: js.Function2[String, String, Any]): Unit = js.native
  def getSetCookie(): js.Array[String] = js.native

trait ResponseInit extends js.Object:
  var status: js.UndefOr[Int] = js.undefined
  var statusText: js.UndefOr[ByteString] = js.undefined
  var headers: js.UndefOr[HeadersInit] = js.undefined

@js.native
trait ReadableStream[+T] extends js.Object:
  def locked: Boolean = js.native
  def cancel(reason: js.UndefOr[Any] = js.native): js.Promise[Unit] = js.native
  def getReader(): ReadableStreamReader[T] = js.native
  def tee(): js.Array[ReadableStream[T]] = js.native

@js.native
trait ReadableStreamReader[+T] extends js.Object:
  def closed: js.Promise[ReadableStreamReader[T]] = js.native
  def cancel(reason: js.UndefOr[Any] = js.native): js.Promise[Unit] = js.native
  def read(): js.Promise[Chunk[T]] = js.native
  def releaseLock(): Unit = js.native

@js.native
trait Chunk[+T] extends js.Object:
  def done: Boolean = js.native
  def value: T = js.native

@js.native
trait Body extends js.Object:
  def bodyUsed: Boolean = js.native
  def body: ReadableStream[Uint8Array] = js.native
  def arrayBuffer(): js.Promise[ArrayBuffer] = js.native
  def blob(): js.Promise[Blob] = js.native
  def formData(): js.Promise[FormData] = js.native
  def json(): js.Promise[js.Any] = js.native
  def text(): js.Promise[String] = js.native

@js.native
@JSGlobal("Request")
class Request(input: RequestInfo, init: RequestInit = js.native) extends Body:
  def method: HttpMethod = js.native
  def url: String = js.native
  def headers: Headers = js.native
  def mode: RequestMode = js.native
  def credentials: RequestCredentials = js.native
  def cache: RequestCache = js.native
  def redirect: RequestRedirect = js.native
  def referrer: String = js.native
  def signal: AbortSignal = js.native
  def clone(): Request = js.native

@js.native
@JSGlobal("Response")
class Response(content: BodyInit = js.native, init: ResponseInit = js.native) extends Body:
  def `type`: ResponseType = js.native
  def url: String = js.native
  def redirected: Boolean = js.native
  def status: Int = js.native
  def ok: Boolean = js.native
  def statusText: String = js.native
  def headers: Headers = js.native
  def clone(): Response = js.native

@js.native
@JSGlobal("Response")
object Response extends js.Object:
  def error(): Response = js.native
  def redirect(url: String, status: Int = js.native): Response = js.native
  def json(data: js.Any, init: js.Object = js.native): Response = js.native

/** The global `fetch` as scalajs-dom's `Fetch` object names it. */
@js.native
@JSGlobalScope
object Fetch extends js.Object:
  def fetch(info: RequestInfo, init: RequestInit = js.native): js.Promise[Response] = js.native
