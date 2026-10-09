// expect: 16:49: error: type mismatch: found String, required Int
// expect: 16:56: error: not found: undefinedName
// expect: 2 errors found
// A member whose first list fails gives way to a conversion of its receiver; the converted call's
// later list holds an application of its own (`g`) whose first list fails: that application types
// its later list for its errors, as scalac's `typedArgs` does, though the member's argument cache is
// installed around it (the cache is the member's own application's).
import scala.quoted.*
inline def unused: String = ${ impl }
def impl(using Quotes): Expr[String] = Expr("unused")
import scala.language.implicitConversions
def g(a: Int)(b: String): String = b
class C { def f(a: Int)(b: String): String = b }
class D { def f(a: String, n: Int)(b: String): String = b }
given Conversion[C,D] with { def apply(c: C): D = new D }
@main def run(): Unit = println(C().f("s", 0)(g("bad")(undefinedName)))
