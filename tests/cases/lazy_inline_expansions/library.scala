// The library half of tests/cases/lazy_inline_expansions: inline methods declaring a local lazy val or a local class
// with lazy vals, compiled with the client in one build, each expansion numbered in the client's file
// (`Cx::local_file`, `ClassInfo::module_file`).
package lib

object Library:
  inline def value(i: Int): Int = { lazy val x = i + 1; x }
  inline def reader(i: Int): () => Int = { lazy val x = i + 1; () => x }
  inline def twice(i: Int): Int = { lazy val x = i * 2; lazy val y = x + 1; x + y }
  inline def local(n: Int): Int =
    class Local:
      lazy val m = n * 100
      def a(): Int = { lazy val x = n; x }
      def b(): Int = { lazy val x = n + 1; x }
    val c = new Local
    c.a() + c.b() + c.m
