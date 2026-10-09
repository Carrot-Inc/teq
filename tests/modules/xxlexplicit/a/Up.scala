package xe

// A `TupleXXL(..)` extractor the source wrote, in a published inline body: its binders are what
// `unapplySeq`'s `Seq[Any]` gives, `Any`, downstream as in the whole build and under scalac, where
// a tuple pattern's (tests/modules/xxlpattern) keep the elements' types. A binder typed `Int` by
// its own pattern leaves its siblings `Any`.
object Up:
  transparent inline def last(t: (Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int)): Any =
    t match
      case scala.runtime.TupleXXL(_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, x) => x

  transparent inline def lastBound(t: (Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int)): Any =
    t match
      case scala.runtime.TupleXXL(a @ (_: Int), _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, x) => x
      case _ => 0

  transparent inline def firstTyped(t: (Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int)): Any =
    t match
      case scala.runtime.TupleXXL(a: Int, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, x) => a
      case _ => 0
