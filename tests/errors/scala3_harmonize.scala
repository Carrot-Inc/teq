// Adapted from scala3 tests/neg/harmonize.scala (Apache-2.0, see tests/scala3/README.md).
// try/catch and inline vals left out, AnyVal written as Any, the Array cases left out (Array.apply is overloaded in Scala).
// expect: 18:18: error: type mismatch: found Char | Int, required Int
// expect: 30:23: error: type mismatch: found Char | Long | Double, required Double
// expect: 33:28: error: type mismatch: found List[Double | Int | Char], required List[Double]
// expect: 37:32: error: type mismatch: found ArrayBuffer[Char], required ArrayBuffer[Int]
// expect: 39:32: error: type mismatch: found ArrayBuffer[Int | Char], required ArrayBuffer[Int]
// expect: 41:35: error: type mismatch: found ArrayBuffer[Double | Long], required ArrayBuffer[Double]
import collection.mutable.ArrayBuffer
object Test:
  final val nn = 2
  final val b = 33
  def f(): Int = b + 1
  def main(args: Array[String]): Unit =
    val x = true
    val n = 1
    val y = if x then 'A' else n
    val z: Int = y // error: no widening

    val yy1 = n match
      case 1 => 'A'
      case 2 => n
      case 3 => 1.0
    val zz1: Any = yy1 // no widening

    val yy3 = n match
      case 1 => 'A'
      case 2 => 3L
      case 3 => 1.0
    val zz3: Double = yy3 // error: no widening from Char to Double

    val xs = List(1.0, nn, 'c')
    val ys: List[Double] = xs // error: no widening

  def arrayBufferTest =
    val a1 = ArrayBuffer(b, 33, 'a')
    val b1: ArrayBuffer[Int] = a1    // error: no widening
    val a2 = ArrayBuffer(b, 33, 'a', f())
    val b2: ArrayBuffer[Int] = a2    // error: no widening
    val a4 = ArrayBuffer(1.0, 1L)
    val b4: ArrayBuffer[Double] = a4 // error: no widening
    val a5 = ArrayBuffer(1.0, 1L, f())
    val b5: ArrayBuffer[Double | Long | Int] = a5
    val a6 = ArrayBuffer(1.0, 1234567890)
    val b6: ArrayBuffer[Double] = a6

  def totalDuration(results: List[Long], cond: Boolean): Long =
    results.map(r => if cond then r else 0).sum
  def totalDuration2(results: List[Long], cond: Boolean): Long =
    results.map { r =>
      cond match
        case true => r
        case false => 0
    }.sum
