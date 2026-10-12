// A language import inside a template, a block or an extension enables postfix operators within
// its statements alone (dotty's `languageImportContext`, restored after `templateStatSeq`,
// `blockStatSeq` and `extMethods`); the code around keeps the ordinary syntax.
object Enabled:
  import scala.language.postfixOps
  val one = List(1) size

object Local:
  def count(xs: List[Int]): Int =
    import scala.language.postfixOps
    xs size
  def plain(xs: List[Int]): Int = xs.size

extension (xs: List[Int])
  def twice: Int =
    import scala.language.postfixOps
    (xs size) * 2

@main def run(): Unit =
  println(Enabled.one)
  println(Local.count(List(1, 2, 3)))
  println(Local.plain(List(1)))
  println(List(4, 5).twice)
