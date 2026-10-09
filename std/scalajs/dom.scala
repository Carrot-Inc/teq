// org.scalajs.dom trimmed from scalajs-dom 2.8.1 to what the app and the scalajs-react replacement use.
// Overloads of the original are merged into union-typed or default parameters. Nothing is emitted for
// these declarations: every class is the browser's own constructor, so `case el: HTMLElement` is instanceof.
package org.scalajs.dom

import scala.scalajs.js
import scala.scalajs.js.annotation.JSGlobal
import scala.scalajs.js.typedarray.{ArrayBuffer, ArrayBufferView, Uint8Array}

type BufferSource = ArrayBufferView | ArrayBuffer
type BlobPart = BufferSource | Blob | String
type BodyInit = Blob | BufferSource | FormData | String | ReadableStream[Uint8Array] | URLSearchParams
type ByteString = String
type HeadersInit = Headers | js.Array[js.Array[String]] | js.Dictionary[String]
type RequestInfo = String | Request

@js.native
@JSGlobal("window")
object window extends Window

@js.native
@JSGlobal("document")
object document extends HTMLDocument

@js.native
@JSGlobal("console")
object console extends Console

@js.native
@JSGlobal("fetch")
def fetch(info: RequestInfo, init: RequestInit = js.native): js.Promise[Response] = js.native
