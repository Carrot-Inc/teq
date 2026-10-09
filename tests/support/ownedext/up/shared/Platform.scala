package shared

// A trait nested in a trait: its outer accessor keeps teq's name across the two projects' class
// files (a jar's takes scalac's, tests/classpath/jvm/jar_outer_accessor.scala).
trait Platform:
  def base: Int
  trait Auto:
    def value: Int = base + 1
    inline def viaOuter: Int = base * 10
  object Impl extends Auto

object Lib extends Platform:
  def base = 4
