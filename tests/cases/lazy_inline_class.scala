// An inline method declaring a local class whose methods declare lazy vals and which has a lazy member, expanded
// twice in this file, and one declaring a class nothing constructs: each expansion's copy of a class is numbered
// in this unit with the rest (dotty's FreshNameCreator is the compilation unit's, and the expansion's TypeDef is
// in it), so its holders, accessors and initialisers are apart from the other copy's and the file's own.
object Library:
  inline def call(n: Int): Int =
    class Local:
      lazy val m = n * 100
      def a(): Int = { lazy val x = n; x }
      def b(): Int = { lazy val x = n + 1; x }
    val c = new Local
    c.a() + c.b() + c.m

  inline def unused(n: Int): Int =
    class Never:
      lazy val y = n
      def get(): Int = { lazy val z = n; z }
    n * 2

object Main:
  def main(args: Array[String]): Unit =
    lazy val x = 1000
    println(Library.call(1))
    println(Library.call(3))
    println(Library.unused(4))
    println(x)
