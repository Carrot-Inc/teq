// expect: value x is unusable in value z because it refers to an erased expression
// scalac 3.8.4 (tests/neg/erased-inline-unrealizable-path.scala): -- Error: ...:22:11 value x is unusable in value z
// because it refers to an erased expression in the selector of an inline match that reduces to `erased val
// $scrutinee1: A & B = scala.compiletime.erasedValue[A & B]; erased val x: A & B = $scrutinee1; ...`.
// The gate of 5010f41f8's typing harness: different -> accepts. master rejected it by a type mismatch (found
// (y : String), required x.L), its member bounds of `x: A & B` taking one side's; the head merges them as dotty's
// `TypeBounds.&` does (`>: String <: Int`), so the typing passes and dotty's rejection is the erased binder's
// use check (the erasure's part, with D4). Pinned for the queued part inlining-lila-folds.
import scala.compiletime.erasedValue
class Box[T]
trait T:
  type L
class A extends T:
  type L <: Int
class B extends T:
  type L >: String
inline def f() =
  inline erasedValue[A & B] match
    case x: (A & B) =>
      val y: String = "String"
      val z: x.L = y
      z: Int
@main def Test =
  println(f().abs)
