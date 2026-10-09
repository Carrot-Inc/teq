package scala.scalajs.js.typedarray

import scala.scalajs.js

@js.native
@JSGlobal("ArrayBuffer")
class ArrayBuffer(length: Int) extends js.Object:
  def byteLength: Int = js.native
