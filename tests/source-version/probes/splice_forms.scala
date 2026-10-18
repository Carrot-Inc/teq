object P { def f(x: Int*) = x.size; val xs = Seq(1); val a = f(xs: _*); val b = f((xs): _*); def g(y: Int)(x: Int*) = x.size; val d = g(1)(xs: _*); val e = f(xs.map(_ + 1): _*) }
