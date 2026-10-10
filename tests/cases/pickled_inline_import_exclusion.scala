// An inline body's wildcard import that leaves an implicit out (`hidden as _`), pickled with the
// exclusion: the expansion does not find it (`0`), teq's own builds' as scalac's from teq's pickle
// (`ImportInfo.importedImplicits`' `excluded`).
object G { implicit val hidden: Int = 7 }

object Lib:
  inline def pickHidden: Int =
    import G.{hidden as _, *}
    scala.compiletime.summonFrom { case _: Int => 1; case _ => 0 }

object ImportExclusion:
  def main(args: Array[String]): Unit =
    println(Lib.pickHidden)
