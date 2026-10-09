// An inline body's wildcard import that leaves an implicit out (`hidden as _`), pickled with the
// exclusion: scalac's expansion from teq's pickle does not find it (`0`). teq's own builds find
// it (`1`), the search's line of the queue.
object G { implicit val hidden: Int = 7 }

object Lib:
  inline def pickHidden: Int =
    import G.{hidden as _, *}
    scala.compiletime.summonFrom { case _: Int => 1; case _ => 0 }

object ImportExclusion:
  def main(args: Array[String]): Unit =
    println(Lib.pickHidden)
