// The jar of tests/classpath/jvm/jar_outer_accessor.scala: a trait nested in a trait, whose outer
// accessor scalac names `outerlib$Platform$Auto$$$outer` (doobie's `ReadPlatform.Auto`).
package outerlib

trait Platform {
  def base: Int
  trait Auto {
    def value: Int = base + 1
    inline def viaOuter: Int = base * 10
  }
  object Impl extends Auto
}

object Lib extends Platform {
  def base = 4
}

// A sealed trait nested in a trait, reached through an object deriving from it and projected
// (scalajs-react's `Custom.Subsequent.AtStep1[I, H1]#Next`, tests/classpath/js/jar_nested_projection.scala):
// `Steps.At[I]#Next` is the alias of `At` seen from `Steps.At[I]`, applied in a result and passed
// unapplied to an alias of the self type's object.
trait Dsl {
  sealed trait At[I] { type Next[H] = (I, H) }
}
object Steps extends Dsl

trait Step[I] {
  type Next[H]
  def next[H](i: I, h: H): Next[H]
}

trait StepsOf { self: Projections.type =>
  type AtStep[I] = To[I, Steps.At[I]#Next]
  implicit def atStep[I]: AtStep[I] = new Step[I] {
    type Next[H] = Steps.At[I]#Next[H]
    def next[H](i: I, h: H): Next[H] = (i, h)
  }
}

object Projections extends StepsOf {
  type To[I, N[_]] = Step[I] { type Next[H] = N[H] }
  def pair[I, H](i: I, h: H): Steps.At[I]#Next[H] = (i, h)
}
