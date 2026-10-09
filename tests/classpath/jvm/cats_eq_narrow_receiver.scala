// jars: scala-library cats-kernel cats-core
// std: scala-library
// cats' `===` on a `Byte` and a `Short` (the application's gzip check of a cached value): no
// extension of the name takes the receiver, widened or not, and the conversion `catsSyntaxEq`
// that supplies the member takes it as its own type, boxed as one, which cats' `Eq[Byte]` unboxes.
import cats.syntax.all.*

def isGzipped(bytes: Array[Byte]): Boolean =
  bytes.length >= 2 && bytes(0) === 0x1f.toByte && bytes(1) === 0x8b.toByte

@main def main(): Unit =
  val b: Byte = 31
  val s: Short = 7
  println(isGzipped(Array[Byte](0x1f, 0x8b.toByte, 0)))
  println(isGzipped(Array[Byte](1, 2)))
  println(b === 31.toByte)
  println(s =!= 8.toShort)
  println(b.eqv(32.toByte))
