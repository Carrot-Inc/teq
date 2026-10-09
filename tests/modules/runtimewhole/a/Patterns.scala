package rwa

// A sequence pattern and a split, which call helpers of teq's runtime (`seqPattern` of
// `scala.runtime.jvm$package$`, `split$x0` of `scala.string$package$`): a directory of products
// holds the runtime whole, so the class still runs after the module's other sources, which reach
// other helpers of the same classes, are compiled again alone into the directory.
object Patterns:
  def pair(xs: List[Int]): Int = xs match
    case List(a, b) => a + b
    case _ => 0
  def parts(s: String): Int = s.split(",").length
