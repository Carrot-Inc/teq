class C[T]; object P { def f[T: C](x: T): T = x; val x = f[Int](1)(new C[Int]) }
