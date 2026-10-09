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
