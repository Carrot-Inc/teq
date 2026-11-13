// `ValueOf` of a path through a top-level export of an inline val, and through a package object's.
package paths

object Origin:
  inline val width = 3

export Origin.width

def atTop: Int = valueOf[width.type]

package object inner:
  object Deep:
    inline val depth = 7
  export Deep.depth

@main def run(): Unit =
  println(s"$atTop ${valueOf[width.type]} $width")
  println(s"${valueOf[inner.depth.type]} ${inner.depth}")
