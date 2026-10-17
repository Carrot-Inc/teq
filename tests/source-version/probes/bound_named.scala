trait C[T]; object P { def f[T: C](): Int = 1; val p = f[Int]()(evidence$1 = new C[Int]{}) }
