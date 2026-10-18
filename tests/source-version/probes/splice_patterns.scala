object P { def f(xs: List[Int]) = xs match { case List(1, rest: _*) => rest.size; case Seq(a, b @ _*) => b.size; case _ => 0 } }
