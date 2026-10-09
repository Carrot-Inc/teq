// A macro's run calls a method through the parent where the child declares another of its name
// and erasure under another `@targetName`, written through an imported alias: the expansion is the
// parent's answer, as under scalac.
package tna

import scala.quoted.*
import scala.annotation.{targetName as tn}

class Parent:
  @tn("ints") def f(x: List[Int]): String = "parent-ints"
class Child extends Parent:
  @tn("strings") def f(x: List[String]): String = "child-strings"

inline def result: String = ${ impl }

def impl(using Quotes): Expr[String] =
  val c = new Child
  val p: Parent = c
  Expr(p.f(List(1)) + "," + c.f(List("s")))
