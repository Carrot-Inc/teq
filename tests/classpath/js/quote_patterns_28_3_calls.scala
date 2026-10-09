// jars: fixtures
// Holes in the arguments of calls in the quote patterns of a macro jar compiled by Scala 3.3
// (TASTy 28.3): a hole of a call, the holes of a curried call bound in the order they are
// written, a `Varargs` extractor in a hole, and the argument left as code for the macro; a hole
// ascribed a type variable (`$a: t`), whose binder is the hole and no `Type[t]` given.
import fix.qpat.{QpFns, QpProbe}

object Main:
  def main(args: Array[String]): Unit =
    println(QpProbe.one(QpFns.single(4)))
    println(QpProbe.curried(QpFns.pair(1)("x")))
    println(QpProbe.spread(QpFns.many(1, 2, 3)))
    println(QpProbe.one(3))
    println(QpProbe.kind("s"))
    println(QpProbe.kind("abc".length))
