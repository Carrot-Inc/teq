import scala.language.implicitConversions

final case class Served(label: String):
  def served: String = label

class Dsl(prefix: String):
  def shout(s: String): String = prefix + s.toUpperCase
  extension (s: String) def tagged: String = s"$prefix[$s]"
  implicit def serving(n: Int): Served = Served(s"$prefix#$n")

object Holder:
  val dsl = Dsl("!")

def render(dsl: Dsl, words: List[String]): List[String] =
  import dsl.*
  words.map(w => shout(w.tagged)) :+ 7.served

def renderNamed(dsl: Dsl, w: String): String =
  import dsl.{shout, tagged}
  shout(w.tagged)

def renderLocal(words: List[String]): String =
  val local = Dsl("~")
  import local.*
  words.map(_.tagged).mkString

class Offset(val n: Int):
  extension (i: Int)
    def a: Int = n + i
    def b: Int = n + i

def byImportedValue(x: Offset, y: Offset): (Int, Int) =
  import x.a
  import y.b
  (0.a, 0.b)

@main def run(): Unit =
  println(byImportedValue(Offset(1), Offset(2)))
  println(render(Dsl("#"), List("a", "b")))
  println(renderNamed(Dsl("$"), "c"))
  println(renderLocal(List("d", "e")))
  import Holder.dsl.*
  println(shout("f") + "g".tagged + 3.served)
