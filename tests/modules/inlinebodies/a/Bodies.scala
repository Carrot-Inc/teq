// An upstream's inline bodies, expanded downstream from their pickles as the definition typed them
// (dotty's `Inlines.bodyToInline` over `TreeUnpickler`'s `LazyBodyAnnotation`): a constant argument
// is the parameter's path, `codeOf` shows the typed argument, each expansion's local class is a
// class of its own, an ascribed constant binds the proxy.
package iba

import scala.compiletime.{codeOf, constValue, erasedValue}

object Bodies:
  inline def value(x: Int): x.type = constValue[x.type]
  inline def code(x: Int): String = codeOf(x)
  inline def make(x: Int): () => Int =
    class C { def value: Int = x }
    val c = new C
    () => c.value
  inline def kindOf[T]: Int = inline erasedValue[T] match { case _: Int => 1; case _ => 2 }
  inline def known(n: Int): Boolean = inline erasedValue[n.type] match { case _: 1 => true; case _ => false }
