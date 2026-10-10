// Inline methods that tests/cases/inline_bodies_*.scala and tests/errors/inline_bodies_*.scala expand
// from scalac's pickles (the jar inlinebodies-lib): each body the definition typed, read back from
// TASTy and expanded as a program's own stored body is, so that the expansion is the one the same
// source gives (dotty's `Inlines.bodyToInline` over the `LazyBodyAnnotation` `TreeUnpickler` attaches).
package inlinebodies

import scala.compiletime.{codeOf, constValue, erasedValue}

object Bodies:
  inline def value(x: Int): x.type = constValue[x.type]
  inline def code(x: Int): String = codeOf(x)
  inline def codeInline(inline x: Int): String = codeOf(x)
  inline def code2(inline x: Int): String = codeInline(x + 1)
  inline def proxied(x: Int): Boolean = codeOf(x).startsWith("x")
  inline def make(x: Int): () => Int =
    class C { def value: Int = x }
    val c = new C
    () => c.value
  inline def made(x: Int): String =
    class D(val y: Int) { override def toString = s"D($y,$x)" }
    new D(x + 1).toString
  inline def kind: Int = inline erasedValue[Int] match { case _: Int => 7 }
  inline def kindOf[T]: Int = inline erasedValue[T] match { case _: Int => 1; case _: String => 2; case _ => 3 }
  inline def zero[T]: T = erasedValue[T]

class Counter(start: Int):
  inline def plus(n: Int): Int = start + Bodies.value(n)
