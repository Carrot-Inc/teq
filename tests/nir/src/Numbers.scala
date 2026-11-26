// A program for the NIR writer's fixtures: Scala Native's plugin tells two NaNs apart by their bits and
// writes each as the canonical one, so its file holds an entry for each NaN of other bits, all of the
// same bytes, which the writer has to write again as they were (tests/nir/README). Compiled on x86-64,
// where the JVM makes 0.0 / 0.0 a NaN of other bits than Double.NaN's. It has the plugin write an insertion
// too (a struct's `new`), and a volatile local's acquiring load and releasing store.
package fix

import scala.scalanative.runtime.{Intrinsics, RawPtr, struct}

object NaNs:
  def doubles: Array[Double] = Array(0.0 / 0.0, Double.NaN)
  def floats: Array[Float] = Array(0.0f / 0.0f, Float.NaN)
  def again: Array[Double] = Array(Double.NaN, 2.0, Double.NaN)
  def alone: Double = 0.0 / 0.0
  def zeros: Array[Double] = Array(-0.0, 0.0, 1.0)
  def floatZeros: Array[Float] = Array(-0.0f, 0.0f, 1.0f)
  def pair(): NaNPair = new NaNPair(0.0 / 0.0, Double.NaN)

class NaNFields:
  var x: Double = Double.NaN
  var y: Double = 0.0 / 0.0
  var f: Float = Float.NaN
  var g: Float = 0.0f / 0.0f

@struct class NaNPair(val x: Double, val y: Double)

object Orders:
  def acquired(p: RawPtr): Int =
    @volatile var at = p
    Intrinsics.storeInt(at, 1)
    Intrinsics.loadInt(at)
