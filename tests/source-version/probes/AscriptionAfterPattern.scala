object P { def f(x: Option[Int]): Int = x match { case Some(y): Option[Int] => y; case _ => 0 } }
