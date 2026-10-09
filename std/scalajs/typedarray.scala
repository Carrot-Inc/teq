package scala.scalajs.js.typedarray

import scala.scalajs.js
import scala.scalajs.js.annotation.{JSBracketAccess, JSGlobal}

@js.native
@JSGlobal("ArrayBuffer")
class ArrayBuffer(size: Int) extends js.Object:
  def byteLength: Int = js.native
  def slice(begin: Int, end: Int = js.native): ArrayBuffer = js.native

@js.native
@JSGlobal("ArrayBuffer")
object ArrayBuffer extends js.Object:
  def isView(arg: scala.Any): Boolean = js.native

@js.native
trait ArrayBufferView extends js.Object:
  def buffer: ArrayBuffer = js.native
  def byteLength: Int = js.native
  def byteOffset: Int = js.native

@js.native
trait TypedArray[T, Repr] extends ArrayBufferView:
  def length: Int = js.native
  @JSBracketAccess
  def apply(index: Int): T = js.native
  @JSBracketAccess
  def update(index: Int, value: T): Unit = js.native
  def set(array: Repr | scala.Array[T], offset: Int = js.native): Unit = js.native
  def subarray(begin: Int, end: Int = js.native): Repr = js.native
  def slice(begin: Int = js.native, end: Int = js.native): Repr = js.native
  def fill(value: T, begin: Int = js.native, end: Int = js.native): Repr = js.native

@js.native
@JSGlobal("DataView")
class DataView(buffer: ArrayBuffer, byteOffset: Int = js.native, byteLength: Int = js.native) extends ArrayBufferView:
  def getInt8(byteOffset: Int): Byte = js.native
  def getUint8(byteOffset: Int): Short = js.native
  def getInt16(byteOffset: Int, littleEndian: Boolean = js.native): Short = js.native
  def getUint16(byteOffset: Int, littleEndian: Boolean = js.native): Int = js.native
  def getInt32(byteOffset: Int, littleEndian: Boolean = js.native): Int = js.native
  def getUint32(byteOffset: Int, littleEndian: Boolean = js.native): Double = js.native
  def getFloat32(byteOffset: Int, littleEndian: Boolean = js.native): Float = js.native
  def getFloat64(byteOffset: Int, littleEndian: Boolean = js.native): Double = js.native
  def setInt8(byteOffset: Int, value: Byte): Unit = js.native
  def setUint8(byteOffset: Int, value: Short): Unit = js.native
  def setInt16(byteOffset: Int, value: Short, littleEndian: Boolean = js.native): Unit = js.native
  def setUint16(byteOffset: Int, value: Int, littleEndian: Boolean = js.native): Unit = js.native
  def setInt32(byteOffset: Int, value: Int, littleEndian: Boolean = js.native): Unit = js.native
  def setUint32(byteOffset: Int, value: Double, littleEndian: Boolean = js.native): Unit = js.native
  def setFloat32(byteOffset: Int, value: Float, littleEndian: Boolean = js.native): Unit = js.native
  def setFloat64(byteOffset: Int, value: Double, littleEndian: Boolean = js.native): Unit = js.native

/** The one argument is a length, a buffer (then offset and count apply), a JS array or another typed array. */
@js.native
@JSGlobal("Int8Array")
class Int8Array(source: Int | ArrayBuffer | ArrayBufferView | scala.Array[Int], offset: Int = js.native, count: Int = js.native)
    extends TypedArray[Int, Int8Array]

@js.native
@JSGlobal("Uint8Array")
class Uint8Array(source: Int | ArrayBuffer | ArrayBufferView | scala.Array[Int], offset: Int = js.native, count: Int = js.native)
    extends TypedArray[Int, Uint8Array]

@js.native
@JSGlobal("Uint8ClampedArray")
class Uint8ClampedArray(source: Int | ArrayBuffer | ArrayBufferView | scala.Array[Int], offset: Int = js.native, count: Int = js.native)
    extends TypedArray[Int, Uint8ClampedArray]

@js.native
@JSGlobal("Int32Array")
class Int32Array(source: Int | ArrayBuffer | ArrayBufferView | scala.Array[Int], offset: Int = js.native, count: Int = js.native)
    extends TypedArray[Int, Int32Array]

@js.native
@JSGlobal("Float32Array")
class Float32Array(source: Int | ArrayBuffer | ArrayBufferView | scala.Array[Double], offset: Int = js.native, count: Int = js.native)
    extends TypedArray[Double, Float32Array]

@js.native
@JSGlobal("Float64Array")
class Float64Array(source: Int | ArrayBuffer | ArrayBufferView | scala.Array[Double], offset: Int = js.native, count: Int = js.native)
    extends TypedArray[Double, Float64Array]

// The object names are those of the original's implicit classes.
object ByteArrayOps:
  extension (bytes: scala.Array[Byte])
    @js("new Int8Array($0)")
    def toTypedArray: Int8Array

object Int8ArrayOps:
  extension (array: Int8Array)
    @js("Array.from($0)")
    def toArray: scala.Array[Byte]

given AB2TA: ByteArrayOps.type = ByteArrayOps

// A `ByteBuffer` over a typed array, which the std's buffers never are: `typedArray` gives a copy
// of the heap array.
final class TypedArrayBufferOps(buffer: java.nio.ByteBuffer):
  def hasTypedArray(): Boolean = false
  def typedArray(): Int8Array = ByteArrayOps.toTypedArray(buffer.array())

object TypedArrayBufferOps:
  implicit def byteBufferOps(buffer: java.nio.ByteBuffer): TypedArrayBufferOps = new TypedArrayBufferOps(buffer)
given TA2AB: Int8ArrayOps.type = Int8ArrayOps
