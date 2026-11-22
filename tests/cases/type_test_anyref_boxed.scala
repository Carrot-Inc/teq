// A type test of `AnyRef` or `Object` takes everything but `null`, as scalac's `instanceof Object` and Scala.js's
// test of `Object` do: a boxed number, a boolean, a char and `()` are objects too. `isInstanceOf`, a pattern, a
// binder, a type parameter's test given a value, a union with `Int`, and a statically typed primitive. The
// interpreter answered `false` for every boxed value and `()`; JavaScript did for numbers, booleans, `Long`s and `()`,
// and answered `true` for `null`.
final case class P(i: Int)
def ref(x: Any): Boolean = x.isInstanceOf[AnyRef]
def obj(x: Any): Boolean = x.isInstanceOf[Object]
def caseRef(x: Any): Int = x match { case _: AnyRef => 1; case _ => 2 }
def caseObj(x: Any): Int = x match { case _: Object => 1; case _ => 2 }
def bindRef(x: Any): String = x match { case r: AnyRef => "r"; case _ => "n" }
def generic[T](t: T): Boolean = t.isInstanceOf[AnyRef]
def unionRef(x: Any): Boolean = x.isInstanceOf[AnyRef | Int]
@main def run(): Unit =
  val values: List[(String, Any)] = List("null" -> null, "int" -> 5, "long" -> 5L, "double" -> 1.5, "float" -> 1.5f,
    "boolean" -> true, "char" -> 'c', "byte" -> 5.toByte, "short" -> 5.toShort, "unit" -> (), "string" -> "s",
    "object" -> new Object, "array" -> Array(1), "list" -> List(1), "function" -> ((x: Int) => x), "case" -> P(1))
  for (name, v) <- values do
    println(s"$name: ref=${ref(v)} obj=${obj(v)} caseRef=${caseRef(v)} caseObj=${caseObj(v)} bindRef=${bindRef(v)} generic=${generic(v)} union=${unionRef(v)}")
  val i: Int = 5
  val l: Long = 5L
  val c: Char = 'c'
  val b: Boolean = true
  val u: Unit = ()
  val d: Double = 1.5
  println(s"static: int=${i.isInstanceOf[AnyRef]} long=${l.isInstanceOf[AnyRef]} char=${c.isInstanceOf[AnyRef]} boolean=${b.isInstanceOf[AnyRef]} unit=${u.isInstanceOf[AnyRef]} double=${d.isInstanceOf[Object]}")
  println(s"generic: int=${generic(5)} unit=${generic(())} char=${generic('c')}")
  println(s"casts: ${(5: Any).asInstanceOf[AnyRef]} ${('c': Any).asInstanceOf[Object]} ${(null: Any).asInstanceOf[AnyRef]} ${i.asInstanceOf[AnyRef]}")
