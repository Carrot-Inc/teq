object P { def g(f: Int => Int) = f(1); val a = g { implicit x => x } }
