// expect: context_function_inferred.scala:14:13: error: no given instance of type String was found for parameter of (String) ?=> Int
// expect: context_function_inferred.scala:15:10: error: no given instance of type String was found for parameter of (String) ?=> Int
// expect: context_function_inferred.scala:16:13: error: no given instance of type String was found for parameter of (String) ?=> Int
// expect: context_function_inferred.scala:17:19: error: no given instance of type String was found for parameter of (String) ?=> Int
// expect: context_function_inferred.scala:18:17: error: no given instance of type String was found for parameter of (String) ?=> Int
// expect: context_function_inferred.scala:19:18: error: no given instance of type String was found for parameter of (String) ?=> Int
// A value of a context function type is applied to the givens in scope wherever no context
// function is expected of it, an inferred type's wildcard and a type variable among them (dotty's
// `Typer.adaptNoArgsOther`): without a given it is an error (E172) at the end of the value.
val f: Int => (String ?=> Int) = n => (s: String) ?=> n + s.length
val h: String ?=> Int = summon[String].length
def g(n: Int): String ?=> Int = n + summon[String].length
def use(x: Any): Unit = ()
val a = f(1)
val b = h
val c = g(1)
val d = f.apply(1)
val u = use(f(1))
val l = List(f(1))
