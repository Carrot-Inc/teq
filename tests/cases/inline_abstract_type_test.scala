// An inline body's type test of its own type parameter, expanded in a method whose type parameter the
// call passes: the test is the erased one of that parameter (`TypeTestsCasts` erases the tested type to
// its upper bound, `Any`'s `Object`), which takes everything but `null`; where the call passes a class,
// the class's test. A lambda the body makes (a magnolia default's check) tests the same way.
inline def check[P](d: Any): Boolean = d.isInstanceOf[P]
inline def lazyCheck[P](d: Any): () => Option[P] = () =>
  val v: Any = d
  if (v: @unchecked).isInstanceOf[P] then Some(v.asInstanceOf[P]) else None
def gen[T](d: Any): Boolean = check[T](d)
def genLazy[T](d: Any): Option[T] = lazyCheck[T](d)()
@main def run(): Unit =
  println(s"${gen[String](null)},${gen[String](1)},${gen[String]("s")}")
  println(s"${genLazy[String](null)},${genLazy[String](1)},${genLazy[String]("s")}")
  println(s"${lazyCheck[String](1)()},${lazyCheck[String]("s")()},${lazyCheck[String](null)()}")
  println(s"${check[String](1)},${check[String]("s")},${check[String](null)}")
