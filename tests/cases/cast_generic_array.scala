// An array whose element is generic erases to `Object` (`TypeErasure.isGenericArrayElement`,
// `eraseArray`): an abstract type no JVM array holds every value of, a union where either part
// is, an intersection where both are. A cast to it checks nothing beyond `Object`; a bound that
// fits one array, or an intersection with `AnyVal`, keeps the array's check.
def attempt(label: String)(f: => Any): Unit =
  try { f; println(label + " ok") }
  catch case _: ClassCastException => println(label + " CCE")
def cast[T](x: Any): Unit = { x.asInstanceOf[Array[T | Int]]; () }
def bounded[T <: Matchable](x: Any): Unit = { x.asInstanceOf[Array[T]]; () }
def inter[T](x: Any): Unit = { x.asInstanceOf[Array[T & AnyVal]]; () }
def union[T](x: Any): Unit = { x.asInstanceOf[Array[T | Int]]; () }
def plain[T](x: Any): Unit = { x.asInstanceOf[Array[T]]; () }
def refs[T <: AnyRef](x: Any): Unit = { x.asInstanceOf[Array[T]]; () }
@main def run(): Unit =
  attempt("union of a parameter") { cast[Any]("text") }
  attempt("Matchable bound") { bounded[Int]("text") }
  attempt("intersection generic") { inter[Any]("text") }
  attempt("union generic") { union[Any]("text") }
  attempt("unbounded") { plain[String]("text") }
  attempt("reference bound") { refs[String]("text") }
