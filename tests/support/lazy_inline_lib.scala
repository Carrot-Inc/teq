// Inline methods declaring a local lazy val or a local class with lazy vals, for tests/cases/lazy_inline_expansions_jar:
// each expansion in the client is a lazy val (a class) of the client's own unit, its holder and initialiser its own.
package lazyinline

object Library:
  inline def value(i: Int): Int = { lazy val x = i + 1; x }
  inline def reader(i: Int): () => Int = { lazy val x = i + 1; () => x }
  inline def local(n: Int): Int =
    class Local:
      lazy val m = n * 100
      def a(): Int = { lazy val x = n; x }
      def b(): Int = { lazy val x = n + 1; x }
    val c = new Local
    c.a() + c.b() + c.m
