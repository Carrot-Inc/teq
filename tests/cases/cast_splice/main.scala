def attempt(name: String)(f: => Any): Unit =
  try
    f
    println(name + " passed")
  catch case e: ClassCastException => println(name + ": " + e.getMessage.takeWhile(_ != '(').trim)

@main def run(): Unit =
  attempt("splice, discarded") { M.cast(new A); () }
  attempt("splice, used") { val b = M.cast(new A); b == null }
  attempt("splice of B") { val b = M.cast(new B); b == null }
  attempt("splice to B") { M.castTo[B](new A); () }
  attempt("splice to Int") { M.castTo[Int]("text") + 1 }
  println(M.castTo[Int](null) + 1)
